use std::collections::{BTreeMap, HashMap, HashSet};

use rust_decimal::Decimal;
use time::{Date, Duration, Time};

use crate::domain::{
    Assumption, AssumptionRules, ComponentPreparation, ConsumedAmount, ConsumptionRecord,
    ConsumptionRecordId, HouseholdMemberId, MealAttendance, MealItemRef, MealOccasion,
    MealOccasionId, MealParticipantAllocation, MealPlanComponent, MealPlanComponentId,
    MealPlanEntry, MealPlanEntryId, MealPlanStatus, MealSlot, MealTimes, NUTRIENT_KEYS,
    NutritionFacts, NutritionGoals, NutritionQuality, PreparedBatch, Quantity, Revision,
    StockLevel, Unit, derive_participant_status, outcomes_for_component,
    participant_status_to_meal, preparation_for, resolve_on, sum_nutrition,
};
use crate::error::Result;
use crate::ports::{MemberQuery, PageRequest};

use super::MealPlanService;

#[derive(Debug, Clone, Default)]
pub struct NutritionSummary {
    pub nutrition: NutritionFacts,
    pub unknown_count: i64,
    pub partial_count: i64,
}

#[derive(Debug, Clone)]
pub struct MealPlanComponentView {
    pub component: MealPlanComponent,
    pub item_name: String,
    pub nutrition: NutritionFacts,
    pub quality: NutritionQuality,
    pub consumption_record: Option<ConsumptionRecord>,
    pub preparation: ComponentPreparation,
    pub cooked: Option<PreparedBatch>,
    pub status: MealPlanStatus,
    pub subject_status: MealPlanStatus,
}

#[derive(Debug, Clone)]
pub struct MealParticipantView {
    pub member_id: HouseholdMemberId,
    pub display_name: String,
    pub note: Option<String>,
    pub status: MealPlanStatus,
    pub allocations: Vec<MealParticipantAllocation>,
    pub nutrition: NutritionSummary,
}

#[derive(Debug, Clone)]
pub struct NeedsReview {
    pub personal: Vec<MealPlanEntryView>,
    pub household: Vec<MealPlanEntryView>,
}

#[derive(Debug, Clone)]
pub struct MealPlanEntryView {
    pub entry: MealPlanEntry,
    pub subject_member_id: Option<HouseholdMemberId>,
    pub components: Vec<MealPlanComponentView>,
    pub participants: Vec<MealParticipantView>,
    pub planned: NutritionSummary,
    pub actual: Option<NutritionSummary>,
    pub needs_attention: bool,
    pub assumption: Assumption,
    pub status: MealPlanStatus,
    pub subject_status: MealPlanStatus,
}

impl MealPlanEntryView {
    pub fn name(&self) -> String {
        self.entry.display_name(|| {
            self.components
                .iter()
                .map(|component| component.item_name.clone())
                .collect()
        })
    }
}

#[derive(Debug, Clone)]
pub struct MealDiner {
    pub member_id: HouseholdMemberId,
    pub display_name: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MealGroupView {
    pub entry: MealPlanEntryView,
    pub name: String,
    pub diners: Vec<MealDiner>,
    pub guest_count: i32,
    pub guests: Vec<MealGuestView>,
    pub serves: i32,
    pub cook_minutes: Option<i32>,
    pub leftover_servings_available: Option<Decimal>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MealGuestView {
    pub id: crate::domain::MealGuestGroupId,
    pub name: Option<String>,
    pub note: Option<String>,
    pub count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookingItemKind {
    Recipe,
    Food,
    CookedFood,
}

#[derive(Debug, Clone)]
pub struct CookingItemView {
    pub item: MealItemRef,
    pub name: String,
    pub kind: CookingItemKind,
    pub amount: ConsumedAmount,
    pub extra_servings: Option<Decimal>,
    pub member_ids: Vec<HouseholdMemberId>,
    pub group_ids: Vec<MealPlanEntryId>,
    pub cook_minutes: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct MealOccasionView {
    pub occasion: MealOccasion,
    pub effective_time: Option<Time>,
    pub groups: Vec<MealGroupView>,
    pub cooking: Vec<CookingItemView>,
    pub absent_member_ids: Vec<HouseholdMemberId>,
    pub unaccounted_member_ids: Vec<HouseholdMemberId>,
}

#[derive(Debug, Clone)]
pub struct PlannerMember {
    pub id: HouseholdMemberId,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct PlannerDay {
    pub date: Date,
    pub occasions: Vec<Option<MealOccasionView>>,
}

#[derive(Debug, Clone)]
pub struct PlannerWeek {
    pub week_start: Date,
    pub week_end: Date,
    pub usual_times: MealTimes,
    pub members: Vec<PlannerMember>,
    pub days: Vec<PlannerDay>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MealItemSource {
    Planned {
        entry_id: MealPlanEntryId,
        component_id: MealPlanComponentId,
    },
    Logged {
        record_id: ConsumptionRecordId,
    },
}

#[derive(Debug, Clone)]
pub struct MealItem {
    pub source: MealItemSource,
    pub record_id: Option<ConsumptionRecordId>,
    pub status: MealPlanStatus,
    pub item: MealItemRef,
    pub item_name: String,
    pub amount: ConsumedAmount,
    pub planned_amount: Option<ConsumedAmount>,
    pub planned_on: Option<Date>,
    pub at: Option<Time>,
    pub consumed_at: Option<time::OffsetDateTime>,
    pub nutrition: NutritionFacts,
    pub quality: NutritionQuality,
    pub needs_attention: bool,
    pub revision: Revision,
    pub record_revision: Option<Revision>,
}

type MealItemOrder = uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MealSlotView {
    pub slot: MealSlot,
    pub occasion_id: Option<MealOccasionId>,
    pub attendance: Option<MealAttendance>,
    pub group_name: Option<String>,
    pub items: Vec<MealItem>,
    pub nutrition: NutritionSummary,
}

#[derive(Debug, Clone)]
pub struct MealPlanDay {
    pub date: Date,
    pub entries: Vec<MealPlanEntryView>,
    pub slots: Vec<MealSlotView>,
    pub actual: NutritionSummary,
    pub remaining_planned: NutritionSummary,
    pub projected: NutritionSummary,
    pub target: Option<NutritionGoals>,
}

#[derive(Debug, Clone)]
pub struct MealPlanWeek {
    pub member_id: crate::domain::HouseholdMemberId,
    pub week_start: Date,
    pub week_end: Date,
    pub days: Vec<MealPlanDay>,
    pub actual: NutritionSummary,
    pub remaining_planned: NutritionSummary,
    pub projected: NutritionSummary,
    pub target: Option<NutritionGoals>,
    pub insufficient_target_coverage: Vec<String>,
}

type SlotAttendance = (MealOccasionId, MealAttendance, Option<String>);

impl MealPlanService {
    pub async fn week(
        &self,
        member_id: crate::domain::HouseholdMemberId,
        week_start: Date,
    ) -> Result<MealPlanWeek> {
        let week_end = week_start + Duration::days(6);
        let occasions = self.plans.list_occasions(week_start, week_end).await?;
        let active = self.active_member_ids().await?;
        let records = self
            .consumption
            .list_period(member_id, week_start, week_end)
            .await?;
        let targets = self.targets.list_for_member(member_id).await?;
        let mut records_by_entry: HashMap<MealPlanEntryId, Vec<ConsumptionRecord>> = HashMap::new();
        for record in &records {
            if let Some(entry_id) = record.meal_plan_entry_id {
                records_by_entry
                    .entry(entry_id)
                    .or_default()
                    .push(record.clone());
            }
        }

        let rules = self.assumption_rules().await?;
        let mut presented_by_date: BTreeMap<Date, Vec<MealPlanEntryView>> = BTreeMap::new();
        let mut items_by_slot: HashMap<(Date, MealSlot), Vec<(MealItemOrder, MealItem)>> =
            HashMap::new();
        let mut attendance_by_slot: HashMap<(Date, MealSlot), SlotAttendance> = HashMap::new();
        for mut occasion in occasions {
            self.materialise_occasion(&mut occasion, &active);
            let attendance = occasion.attendance_of(member_id);
            let key = (occasion.planned_on, occasion.slot);
            let mut group_name = None;
            if let MealAttendance::Eating { group_id, .. } = &attendance
                && let Some(group) = occasion
                    .groups
                    .into_iter()
                    .find(|group| group.id == *group_id)
            {
                let linked = records_by_entry
                    .get(&group.id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let date = group.planned_on;
                let view = self
                    .present_with(&rules, group, linked, Some(member_id))
                    .await?;
                group_name = Some(view.name());
                for (item_date, order, item) in items_for_entry(&view) {
                    let bucket = (item_date, view.entry.slot);
                    items_by_slot.entry(bucket).or_default().push((order, item));
                }
                presented_by_date.entry(date).or_default().push(view);
            }
            attendance_by_slot.insert(key, (occasion.id, attendance, group_name));
        }
        let logged_catalogue = self
            .catalogue_for(
                records
                    .iter()
                    .filter(|record| record.meal_plan_component_id.is_none())
                    .map(|record| record.item),
            )
            .await?;
        for record in &records {
            if record.meal_plan_component_id.is_some() {
                continue;
            }
            let item_name = logged_catalogue.name_of(record.item);
            let bucket = (record.consumed_on, record.slot);
            items_by_slot
                .entry(bucket)
                .or_default()
                .push((record.id.as_uuid(), logged_item(record, item_name)));
        }

        let mut days = Vec::with_capacity(7);
        for offset in 0..7 {
            let date = week_start + Duration::days(offset);
            let mut day_entries = presented_by_date.remove(&date).unwrap_or_default();
            day_entries.sort_by_key(|view| {
                (
                    view.entry.slot.order(),
                    view.entry.planned_time,
                    view.entry.created_at,
                    view.entry.id,
                )
            });
            let actual = summary(records.iter().filter(|record| record.consumed_on == date));
            let remaining_planned = summary_from_views(day_entries.iter());
            let projected = combine_summaries(&actual, &remaining_planned);
            let target = resolve_on(&targets, date).map(|target| target.goals.clone());
            let slots = MealSlot::ALL
                .into_iter()
                .map(|slot| {
                    let mut items = items_by_slot.remove(&(date, slot)).unwrap_or_default();
                    items.sort_by_key(|(order, _)| *order);
                    let items: Vec<_> = items.into_iter().map(|(_, item)| item).collect();
                    let nutrition = item_summary(&items);
                    let attendance = attendance_by_slot.remove(&(date, slot));
                    MealSlotView {
                        slot,
                        occasion_id: attendance.as_ref().map(|(id, _, _)| *id),
                        attendance: attendance.as_ref().map(|(_, kind, _)| kind.clone()),
                        group_name: attendance.and_then(|(_, _, name)| name),
                        items,
                        nutrition,
                    }
                })
                .collect();
            days.push(MealPlanDay {
                date,
                entries: day_entries,
                slots,
                actual,
                remaining_planned,
                projected,
                target,
            });
        }

        let actual = combine_many(days.iter().map(|day| &day.actual));
        let remaining_planned = combine_many(days.iter().map(|day| &day.remaining_planned));
        let projected = combine_summaries(&actual, &remaining_planned);
        let (target, insufficient_target_coverage) =
            weekly_goals(days.iter().map(|day| day.target.as_ref()));
        Ok(MealPlanWeek {
            member_id,
            week_start,
            week_end,
            days,
            actual,
            remaining_planned,
            projected,
            target,
            insufficient_target_coverage,
        })
    }

    pub async fn planner_week(&self, week_start: Date) -> Result<PlannerWeek> {
        let week_end = week_start + Duration::days(6);
        let occasions = self.plans.list_occasions(week_start, week_end).await?;
        let members = self.active_members().await?;
        let active: Vec<HouseholdMemberId> = members.iter().map(|member| member.id).collect();
        let rules = self.assumption_rules().await?;
        let mut by_cell: HashMap<(Date, MealSlot), MealOccasionView> = HashMap::new();
        for mut occasion in occasions {
            self.materialise_occasion(&mut occasion, &active);
            let key = (occasion.planned_on, occasion.slot);
            let view = self.present_occasion(&rules, occasion).await?;
            by_cell.insert(key, view);
        }
        let mut days = Vec::with_capacity(7);
        for offset in 0..7 {
            let date = week_start + Duration::days(offset);
            let occasions = MealSlot::ALL
                .into_iter()
                .map(|slot| by_cell.remove(&(date, slot)))
                .collect();
            days.push(PlannerDay { date, occasions });
        }
        Ok(PlannerWeek {
            week_start,
            week_end,
            usual_times: rules.meal_times,
            members,
            days,
        })
    }

    pub async fn needs_review(
        &self,
        member_id: HouseholdMemberId,
        include_household: bool,
    ) -> Result<NeedsReview> {
        let rules = self.assumption_rules().await?;
        let today = rules.now.date();
        let active = self.active_member_ids().await?;
        let mut personal = Vec::new();
        let mut household = Vec::new();
        for mut occasion in self.plans.list_occasions_through(today).await? {
            self.materialise_occasion(&mut occasion, &active);
            for group in occasion.groups {
                let records = self.records_for_entry(group.id).await?;
                if group.participant_for(member_id).is_some() {
                    let view = self
                        .present_with(&rules, group.clone(), &records, Some(member_id))
                        .await?;
                    if view.subject_status == MealPlanStatus::Assumed {
                        personal.push(view);
                    }
                }
                if include_household {
                    let view = self.present_with(&rules, group, &records, None).await?;
                    if view.status == MealPlanStatus::Assumed
                        || (view.status == MealPlanStatus::PartiallyResolved
                            && view.assumption.assumed)
                    {
                        household.push(view);
                    }
                }
            }
        }

        personal.sort_by_key(|view| (view.entry.planned_on, view.entry.slot.order()));
        household.sort_by_key(|view| (view.entry.planned_on, view.entry.slot.order()));
        Ok(NeedsReview {
            personal,
            household,
        })
    }

    pub(super) async fn active_members(&self) -> Result<Vec<PlannerMember>> {
        let page = self
            .members
            .list(&MemberQuery {
                include_archived: false,
                page: PageRequest::new(1, PageRequest::MAX_PER_PAGE),
                ..Default::default()
            })
            .await?;
        Ok(page
            .items
            .into_iter()
            .filter(|member| !member.is_archived())
            .map(|member| PlannerMember {
                id: member.id,
                display_name: member.display_name,
            })
            .collect())
    }

    pub(super) async fn present_occasion(
        &self,
        rules: &AssumptionRules,
        occasion: MealOccasion,
    ) -> Result<MealOccasionView> {
        let active = self.active_member_ids().await?;
        let effective_time = occasion.effective_time(&rules.meal_times);
        let mut groups = Vec::with_capacity(occasion.groups.len());
        let mut seated: HashSet<HouseholdMemberId> = HashSet::new();
        for group in &occasion.groups {
            let records = self.records_for_entry(group.id).await?;
            let view = self
                .present_with(rules, group.clone(), &records, None)
                .await?;
            let name = view.name();
            let diners: Vec<MealDiner> = view
                .participants
                .iter()
                .map(|participant| MealDiner {
                    member_id: participant.member_id,
                    display_name: participant.display_name.clone(),
                    note: participant.note.clone(),
                })
                .collect();
            seated.extend(diners.iter().map(|diner| diner.member_id));
            let cook_minutes = self.cook_minutes(group).await?;
            let leftover_servings_available = self.leftover_servings(group).await?;
            groups.push(MealGroupView {
                name,
                diners,
                guest_count: group.guest_count(),
                guests: group
                    .guest_groups
                    .iter()
                    .map(|guest| MealGuestView {
                        id: guest.id,
                        name: guest.name.clone(),
                        note: guest.note.clone(),
                        count: guest.count,
                    })
                    .collect(),
                serves: group.serves(),
                cook_minutes,
                leftover_servings_available,
                entry: view,
            });
        }
        let absent_member_ids: Vec<HouseholdMemberId> = occasion
            .absences
            .iter()
            .map(|absence| absence.member_id)
            .collect();
        let unaccounted_member_ids = active
            .into_iter()
            .filter(|member_id| {
                !seated.contains(member_id) && !absent_member_ids.contains(member_id)
            })
            .collect();
        let cooking = self.cooking_items(&groups).await?;
        Ok(MealOccasionView {
            effective_time,
            groups,
            cooking,
            absent_member_ids,
            unaccounted_member_ids,
            occasion,
        })
    }

    async fn cooking_items(&self, groups: &[MealGroupView]) -> Result<Vec<CookingItemView>> {
        struct Bucket {
            item: MealItemRef,
            name: String,
            kind: CookingItemKind,
            amount_kind: &'static str,
            unit: Option<Unit>,
            total: Decimal,
            extra: Decimal,
            member_ids: Vec<HouseholdMemberId>,
            group_ids: Vec<MealPlanEntryId>,
            cook_minutes: Option<i32>,
            order: usize,
        }

        let mut buckets: Vec<Bucket> = Vec::new();
        for group in groups {
            if group.entry.entry.ad_hoc.is_some() {
                continue;
            }
            let member_ids: Vec<HouseholdMemberId> =
                group.diners.iter().map(|diner| diner.member_id).collect();
            for component in &group.entry.components {
                let item = component.component.item;
                let unit = match component.component.amount {
                    ConsumedAmount::Measure(quantity) => Some(quantity.unit),
                    _ => None,
                };
                let amount_kind = component.component.amount.kind_code();
                let cook_minutes = if let MealItemRef::Recipe { recipe_id } = item {
                    self.recipes.get(recipe_id).await?.and_then(|recipe| {
                        let minutes = recipe.preparation_minutes.unwrap_or(0)
                            + recipe.cooking_minutes.unwrap_or(0);
                        (recipe.preparation_minutes.is_some() || recipe.cooking_minutes.is_some())
                            .then_some(minutes)
                    })
                } else {
                    None
                };
                let extra = if matches!(item, MealItemRef::Recipe { .. }) {
                    let effective = component.component.effective_cooking_servings(group.serves);
                    Decimal::from((effective - group.serves).max(0))
                } else {
                    Decimal::ZERO
                };
                let kind = match item {
                    MealItemRef::Recipe { .. } => CookingItemKind::Recipe,
                    MealItemRef::Dish { .. } => CookingItemKind::CookedFood,
                    _ => CookingItemKind::Food,
                };
                match buckets.iter_mut().find(|bucket| {
                    bucket.item == item && bucket.amount_kind == amount_kind && bucket.unit == unit
                }) {
                    Some(bucket) => {
                        bucket.total += component.component.amount.value();
                        bucket.extra += extra;
                        for member_id in &member_ids {
                            if !bucket.member_ids.contains(member_id) {
                                bucket.member_ids.push(*member_id);
                            }
                        }
                        if !bucket.group_ids.contains(&group.entry.entry.id) {
                            bucket.group_ids.push(group.entry.entry.id);
                        }
                        bucket.cook_minutes = match (bucket.cook_minutes, cook_minutes) {
                            (Some(a), Some(b)) => Some(a.max(b)),
                            (Some(a), None) => Some(a),
                            (None, other) => other,
                        };
                    }
                    None => {
                        buckets.push(Bucket {
                            item,
                            name: component.item_name.clone(),
                            kind,
                            amount_kind,
                            unit,
                            total: component.component.amount.value(),
                            extra,
                            member_ids: member_ids.clone(),
                            group_ids: vec![group.entry.entry.id],
                            cook_minutes,
                            order: buckets.len(),
                        });
                    }
                }
            }
        }

        buckets
            .sort_by_key(|bucket| (if bucket.group_ids.len() > 1 { 0 } else { 1 }, bucket.order));
        Ok(buckets
            .into_iter()
            .map(|bucket| CookingItemView {
                item: bucket.item,
                name: bucket.name,
                kind: bucket.kind,
                amount: rebuild_amount(bucket.amount_kind, bucket.total, bucket.unit),
                extra_servings: (bucket.extra > Decimal::ZERO).then_some(bucket.extra),
                member_ids: bucket.member_ids,
                group_ids: bucket.group_ids,
                cook_minutes: bucket.cook_minutes,
            })
            .collect())
    }

    async fn cook_minutes(&self, group: &MealPlanEntry) -> Result<Option<i32>> {
        let mut total: Option<i32> = None;
        for component in &group.components {
            let MealItemRef::Recipe { recipe_id } = component.item else {
                continue;
            };
            if let Some(recipe) = self.recipes.get(recipe_id).await? {
                let minutes =
                    recipe.preparation_minutes.unwrap_or(0) + recipe.cooking_minutes.unwrap_or(0);
                if recipe.preparation_minutes.is_some() || recipe.cooking_minutes.is_some() {
                    total = Some(total.unwrap_or(0).max(minutes));
                }
            }
        }
        Ok(total)
    }

    async fn leftover_servings(&self, group: &MealPlanEntry) -> Result<Option<Decimal>> {
        let mut total: Option<Decimal> = None;
        for component in &group.components {
            let MealItemRef::Dish { recipe_id } = component.item else {
                continue;
            };
            let mut available = Decimal::ZERO;
            for batch in self.batches.held_for_recipe(recipe_id).await? {
                for portion in self.batches.portions(batch.id).await? {
                    if portion.is_archived()
                        || !portion
                            .usability_deadline
                            .as_ref()
                            .is_some_and(|deadline| deadline.date >= group.planned_on)
                    {
                        continue;
                    }
                    match portion.level {
                        StockLevel::Exact { quantity } | StockLevel::Estimated { quantity }
                            if quantity.unit == Unit::Serving
                                && quantity.amount > Decimal::ZERO =>
                        {
                            available += quantity.amount;
                        }
                        _ => {}
                    }
                }
            }
            total = Some(total.unwrap_or(Decimal::ZERO) + available);
        }
        Ok(total)
    }

    pub(super) async fn present(
        &self,
        entry: MealPlanEntry,
        records: &[ConsumptionRecord],
        requested_subject: Option<HouseholdMemberId>,
    ) -> Result<MealPlanEntryView> {
        let rules = self.assumption_rules().await?;
        self.present_with(&rules, entry, records, requested_subject)
            .await
    }

    pub(super) async fn present_with(
        &self,
        rules: &AssumptionRules,
        entry: MealPlanEntry,
        records: &[ConsumptionRecord],
        requested_subject: Option<HouseholdMemberId>,
    ) -> Result<MealPlanEntryView> {
        let subject = requested_subject.or_else(|| entry.participants.first().map(|p| p.member_id));
        let assumption = rules.for_entry(&entry);
        let catalogue = self
            .catalogue_for(entry.components.iter().map(|component| component.item))
            .await?;
        let records_by_key: HashMap<(MealPlanComponentId, HouseholdMemberId), ConsumptionRecord> =
            records
                .iter()
                .filter_map(|record| {
                    record
                        .meal_plan_component_id
                        .map(|component_id| ((component_id, record.member_id), record.clone()))
                })
                .collect();

        let cooked_by_component = {
            let ids: Vec<MealPlanComponentId> = entry
                .components
                .iter()
                .map(|component| component.id)
                .collect();
            self.batches.for_components(&ids).await?
        };

        let component_ids: HashSet<MealPlanComponentId> = entry
            .components
            .iter()
            .map(|component| component.id)
            .collect();

        let mut needs_attention = false;
        let mut components = Vec::with_capacity(entry.components.len());
        for component in &entry.components {
            let subject_alloc = subject.and_then(|member| {
                entry.participant_for(member).and_then(|participant| {
                    participant
                        .allocations
                        .iter()
                        .find(|a| a.component_id == component.id)
                        .map(|a| a.allocated)
                })
            });
            let display_amount = subject_alloc.unwrap_or(component.amount);
            let subject_record = subject
                .and_then(|member| records_by_key.get(&(component.id, member)))
                .cloned();

            let (item_name, nutrition, quality) = if let Some(record) = &subject_record {
                let name = component
                    .snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.item_name.clone())
                    .unwrap_or_else(|| catalogue.name_of(component.item));
                (name, record.nutrition.clone(), record.quality)
            } else if let Some(snapshot) = &component.snapshot {
                (
                    snapshot.item_name.clone(),
                    snapshot.scaled_to(&component.amount, &display_amount),
                    snapshot.quality,
                )
            } else {
                let resolved = catalogue.resolve(component.item, &display_amount);
                if !resolved.resolvable {
                    needs_attention = true;
                }
                (
                    resolved.name,
                    resolved.nutrition.facts,
                    resolved.nutrition.quality,
                )
            };

            let cooked = cooked_by_component.get(&component.id).cloned();
            let outcomes = outcomes_for_component(&entry, &records_by_key, component.id);
            let prepared = cooked
                .as_ref()
                .map(|batch| ConsumedAmount::Servings(batch.servings_produced))
                .unwrap_or(component.amount);
            let preparation = preparation_for(&prepared, &outcomes);
            let subject_status = subject
                .and_then(|member| entry.participant_for(member))
                .and_then(|participant| {
                    participant
                        .allocations
                        .iter()
                        .find(|a| a.component_id == component.id)
                        .map(|a| participant_status_to_meal(a.status, assumption))
                })
                .unwrap_or_else(|| entry.component_status(component.id, assumption));

            components.push(MealPlanComponentView {
                component: component.clone(),
                item_name,
                nutrition,
                quality,
                consumption_record: subject_record,
                preparation,
                cooked,
                status: entry.component_status(component.id, assumption),
                subject_status,
            });
        }

        let mut participant_views = Vec::with_capacity(entry.participants.len());
        for participant in &entry.participants {
            let display_name = self
                .members
                .get(participant.member_id)
                .await?
                .map(|member| member.display_name)
                .unwrap_or_default();
            let participant_records: Vec<&ConsumptionRecord> = records
                .iter()
                .filter(|record| {
                    record.member_id == participant.member_id
                        && record
                            .meal_plan_component_id
                            .is_some_and(|id| component_ids.contains(&id))
                })
                .collect();
            participant_views.push(MealParticipantView {
                member_id: participant.member_id,
                display_name,
                note: participant.note.clone(),
                status: derive_participant_status(participant, assumption),
                allocations: participant.allocations.clone(),
                nutrition: summary(participant_records.into_iter()),
            });
        }

        let planned = summary_components(
            components
                .iter()
                .filter(|component| component.subject_status.is_unresolved()),
        );
        let subject_records: Vec<&ConsumptionRecord> = subject
            .map(|member| {
                records
                    .iter()
                    .filter(|record| record.member_id == member)
                    .collect()
            })
            .unwrap_or_default();
        let actual = if subject_records.is_empty() {
            None
        } else {
            Some(summary(subject_records.into_iter()))
        };

        let status = entry.status(assumption);
        let subject_status = subject
            .and_then(|member| entry.participant_for(member))
            .map(|participant| derive_participant_status(participant, assumption))
            .unwrap_or(status);

        Ok(MealPlanEntryView {
            entry,
            subject_member_id: subject,
            components,
            participants: participant_views,
            planned,
            actual,
            needs_attention,
            assumption,
            status,
            subject_status,
        })
    }
}

fn rebuild_amount(kind_code: &str, total: Decimal, unit: Option<Unit>) -> ConsumedAmount {
    match kind_code {
        "measure" => ConsumedAmount::Measure(Quantity::new(total, unit.unwrap_or(Unit::Gram))),
        "packs" => ConsumedAmount::Packs(total),
        _ => ConsumedAmount::Servings(total),
    }
}

fn items_for_entry(view: &MealPlanEntryView) -> Vec<(Date, MealItemOrder, MealItem)> {
    view.components
        .iter()
        .map(|component| {
            let order = component.component.display_order;
            let source = MealItemSource::Planned {
                entry_id: view.entry.id,
                component_id: component.component.id,
            };
            match &component.consumption_record {
                Some(record) => (
                    record.consumed_on,
                    order,
                    MealItem {
                        source,
                        record_id: Some(record.id),
                        status: MealPlanStatus::Eaten,
                        item: component.component.item,
                        item_name: component.item_name.clone(),
                        amount: record.amount,
                        planned_amount: (record.amount != component.component.amount)
                            .then_some(component.component.amount),
                        planned_on: (record.consumed_on != view.entry.planned_on)
                            .then_some(view.entry.planned_on),
                        at: None,
                        consumed_at: record.consumed_at,
                        nutrition: record.nutrition.clone(),
                        quality: record.quality,
                        needs_attention: view.needs_attention,
                        revision: component.component.revision,
                        record_revision: Some(record.revision),
                    },
                ),
                None => (
                    view.entry.planned_on,
                    order,
                    MealItem {
                        source,
                        record_id: None,
                        status: component.subject_status,
                        item: component.component.item,
                        item_name: component.item_name.clone(),
                        amount: component
                            .preparation
                            .allocated
                            .filter(|_| component.subject_status != MealPlanStatus::NotEaten)
                            .map(|_| component.component.amount)
                            .unwrap_or(component.component.amount),
                        planned_amount: None,
                        planned_on: None,
                        at: view.entry.planned_time,
                        consumed_at: None,
                        nutrition: component.nutrition.clone(),
                        quality: component.quality,
                        needs_attention: view.needs_attention,
                        revision: component.component.revision,
                        record_revision: None,
                    },
                ),
            }
        })
        .collect()
}

fn logged_item(record: &ConsumptionRecord, item_name: String) -> MealItem {
    MealItem {
        source: MealItemSource::Logged {
            record_id: record.id,
        },
        record_id: Some(record.id),
        status: MealPlanStatus::Eaten,
        item: record.item,
        item_name,
        amount: record.amount,
        planned_amount: None,
        planned_on: None,
        at: None,
        consumed_at: record.consumed_at,
        nutrition: record.nutrition.clone(),
        quality: record.quality,
        needs_attention: false,
        revision: record.revision,
        record_revision: Some(record.revision),
    }
}

fn item_summary(items: &[MealItem]) -> NutritionSummary {
    let included: Vec<_> = items
        .iter()
        .filter(|item| item.status != MealPlanStatus::NotEaten)
        .collect();
    NutritionSummary {
        nutrition: sum_nutrition(included.iter().map(|item| &item.nutrition)),
        unknown_count: included
            .iter()
            .filter(|item| item.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: included
            .iter()
            .filter(|item| item.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn summary_components<'a>(
    components: impl Iterator<Item = &'a MealPlanComponentView>,
) -> NutritionSummary {
    let components: Vec<_> = components.collect();
    NutritionSummary {
        nutrition: sum_nutrition(components.iter().map(|component| &component.nutrition)),
        unknown_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn summary<'a>(records: impl Iterator<Item = &'a ConsumptionRecord>) -> NutritionSummary {
    let records: Vec<_> = records.collect();
    NutritionSummary {
        nutrition: sum_nutrition(records.iter().map(|record| &record.nutrition)),
        unknown_count: records
            .iter()
            .filter(|record| record.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: records
            .iter()
            .filter(|record| record.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn summary_from_views<'a>(views: impl Iterator<Item = &'a MealPlanEntryView>) -> NutritionSummary {
    let components: Vec<_> = views
        .flat_map(|view| view.components.iter())
        .filter(|component| component.subject_status.is_unresolved())
        .collect();
    NutritionSummary {
        nutrition: sum_nutrition(components.iter().map(|component| &component.nutrition)),
        unknown_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Unknown)
            .count() as i64,
        partial_count: components
            .iter()
            .filter(|component| component.quality == NutritionQuality::Partial)
            .count() as i64,
    }
}

fn combine_many<'a>(summaries: impl Iterator<Item = &'a NutritionSummary>) -> NutritionSummary {
    let summaries: Vec<_> = summaries.collect();
    NutritionSummary {
        nutrition: sum_nutrition(summaries.iter().map(|summary| &summary.nutrition)),
        unknown_count: summaries.iter().map(|summary| summary.unknown_count).sum(),
        partial_count: summaries.iter().map(|summary| summary.partial_count).sum(),
    }
}

fn combine_summaries(left: &NutritionSummary, right: &NutritionSummary) -> NutritionSummary {
    combine_many([left, right].into_iter())
}

fn weekly_goals<'a>(
    daily: impl Iterator<Item = Option<&'a NutritionGoals>>,
) -> (Option<NutritionGoals>, Vec<String>) {
    let daily: Vec<Option<&NutritionGoals>> = daily.collect();
    let day_count = daily.len();
    let mut goals = NutritionGoals::default();
    let mut insufficient = Vec::new();
    for key in NUTRIENT_KEYS {
        let values: Vec<Decimal> = daily
            .iter()
            .filter_map(|day| day.and_then(|day| day.get(key)))
            .collect();
        let covered = values.len();
        if day_count > 0 && covered == day_count {
            goals.set(key, Some(values.iter().copied().sum()));
        } else if covered > 0 {
            insufficient.push(key.to_string());
        }
    }
    let target = if goals.is_empty() { None } else { Some(goals) };
    (target, insufficient)
}
