use rust_decimal::Decimal;
use time::Date;

use super::stock::{current_unit, fefo_key};
use super::{Confidence, DemandClaim, DemandGap, Quantity, StockItem, StockLevel, apply_take};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    pub shortfall: Option<Quantity>,
    pub required_by: Option<Date>,
    pub use_by_at_least: Option<Date>,
    pub uncovered: Vec<DemandClaim>,
    pub assumption_only: bool,
    pub gaps: Vec<DemandGap>,
    pub confidence: Confidence,
}

impl Default for Coverage {
    fn default() -> Self {
        Self {
            shortfall: None,
            required_by: None,
            use_by_at_least: None,
            uncovered: Vec::new(),
            assumption_only: false,
            gaps: Vec::new(),
            confidence: Confidence::Exact,
        }
    }
}

impl Coverage {
    pub fn is_short(&self) -> bool {
        self.shortfall.is_some()
    }
}

pub fn cover(items: &[StockItem], claims: &[DemandClaim]) -> Coverage {
    let live: Vec<&StockItem> = items.iter().filter(|item| !item.is_archived()).collect();

    if live.iter().any(|item| item.level.is_not_tracked()) {
        return Coverage::default();
    }

    let full = walk(&live, claims);
    if !full.is_short() {
        return full;
    }

    let confirmed: Vec<DemandClaim> = claims
        .iter()
        .filter(|claim| !claim.assumed)
        .cloned()
        .collect();
    let assumption_only = confirmed.len() != claims.len() && !walk(&live, &confirmed).is_short();

    Coverage {
        assumption_only,
        ..full
    }
}

fn walk(live: &[&StockItem], claims: &[DemandClaim]) -> Coverage {
    let mut levels: Vec<(&StockItem, StockLevel)> =
        live.iter().map(|item| (*item, item.level)).collect();

    let mut ordered: Vec<&DemandClaim> = claims.iter().collect();
    ordered.sort_by_key(|claim| (claim.planned_on, claim.entry_id.as_uuid()));

    let mut coverage = Coverage::default();

    for claim in ordered {
        if claim.quantity.amount <= Decimal::ZERO {
            continue;
        }
        let mut remaining = claim.quantity.amount;

        let mut order: Vec<usize> = (0..levels.len())
            .filter(|&i| usable_on(levels[i].0, claim.planned_on))
            .collect();
        order.sort_by_key(|&i| fefo_key(levels[i].0));

        for i in order {
            if remaining <= Decimal::ZERO {
                break;
            }
            let (_, level) = &levels[i];
            let (Some(unit), Some(available)) =
                (current_unit(level), level.conservative_quantity())
            else {
                continue;
            };
            let want_here = match Quantity::new(remaining, claim.quantity.unit).convert_to(unit) {
                Ok(converted) => converted.amount,
                Err(_) => {
                    note(&mut coverage.gaps, DemandGap::IncompatibleUnits);
                    continue;
                }
            };
            let take = want_here.min(available.amount);
            if take <= Decimal::ZERO {
                continue;
            }
            if level.is_estimated() {
                coverage.confidence = Confidence::Estimated;
            }
            if let Some(applied) = apply_take(level, Quantity::new(take, unit)) {
                levels[i].1 = applied.new_level;
            }
            remaining -= Quantity::new(take, unit)
                .convert_to(claim.quantity.unit)
                .map(|q| q.amount)
                .unwrap_or(Decimal::ZERO);
        }

        if remaining > Decimal::ZERO {
            add_shortfall(&mut coverage, Quantity::new(remaining, claim.quantity.unit));
            coverage.required_by = Some(match coverage.required_by {
                Some(existing) => existing.min(claim.planned_on),
                None => claim.planned_on,
            });
            coverage.use_by_at_least = Some(match coverage.use_by_at_least {
                Some(existing) => existing.max(claim.planned_on),
                None => claim.planned_on,
            });
            coverage.uncovered.push(claim.clone());
        }
    }

    coverage
}

fn usable_on(item: &StockItem, on: Date) -> bool {
    item.usability_deadline
        .as_ref()
        .is_none_or(|deadline| deadline.date >= on)
}

fn add_shortfall(coverage: &mut Coverage, missing: Quantity) {
    match coverage.shortfall {
        None => coverage.shortfall = Some(missing),
        Some(running) => match missing.convert_to(running.unit) {
            Ok(converted) => {
                coverage.shortfall = Some(Quantity::new(
                    running.amount + converted.amount,
                    running.unit,
                ));
            }
            Err(_) => note(&mut coverage.gaps, DemandGap::IncompatibleUnits),
        },
    }
}

fn note(gaps: &mut Vec<DemandGap>, gap: DemandGap) {
    if !gaps.contains(&gap) {
        gaps.push(gap);
    }
}

#[cfg(test)]
#[path = "coverage_tests.rs"]
mod tests;
