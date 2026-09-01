use std::fmt;
use std::str::FromStr;

use anyhow::{Context, bail};
use mmp_core::CoreError;
use mmp_core::domain::{
    AccessScope, ActualMealPlanComponent, ConfirmMealPlanComponent, ConfirmMealPlanEntry,
    ConsumedAmount, ConsumptionRecordId, HouseholdMember, HouseholdMemberId, IngredientId,
    MealCategory, MealItemRef, MealPlanEntryId, MealPlanScope, MealPlanStatus, MealSlot,
    NewConsumptionRecord, NewHouseholdMember, NewMealGuestAllocation, NewMealGuestGroup,
    NewMealParticipant, NewMealParticipantAllocation, NewMealPlanComponent, NewMealPlanEntry,
    NewNutritionTarget, NewProduct, NewRecipe, NewRecipeComponent, NewRecipeInstruction,
    NewStockItem, NewUser, NutritionFacts, NutritionGoals, OutcomeActor, Patch, ProductId,
    Provenance, Quantity, RecipeId, RecipePatch, RecipeRequirement, Role, SourceDate,
    SourceDateKind, StockLevel, StorageLocation, Unit, User, UserId,
};
use mmp_server::state::AppState;
use rust_decimal::Decimal;
use time::{Date, Duration, PrimitiveDateTime, Time};
use uuid::Uuid;

const SAMPLE_NAMESPACE: Uuid = Uuid::from_u128(0x6d6d_7073_616d_706c_6580_4c2f_923b_8d10);
const SAMPLE_RECIPE_IMAGE: &[u8] = include_bytes!("../assets/sample_recipe.png");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    Minimal,
    Full,
}

impl fmt::Display for Scenario {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Scenario::Minimal => "minimal",
            Scenario::Full => "full",
        })
    }
}

impl FromStr for Scenario {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "minimal" => Ok(Self::Minimal),
            "full" => Ok(Self::Full),
            _ => bail!("unknown scenario `{value}`; expected `minimal` or `full`"),
        }
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub users_created: usize,
    pub members_created: usize,
    pub products_created: usize,
    pub recipes_created: usize,
    pub targets_created: usize,
    pub stock_items_created: usize,
    pub meals_created: usize,
    pub meals_resolved: usize,
    pub stock_effects_applied: usize,
    pub household_participants_created: usize,
    pub diary_entries_created: usize,
}

struct Loader<'a> {
    state: &'a AppState,
    actor: User,
    member: HouseholdMember,
    week_start: Date,
    today: Date,
    report: Report,
}

#[derive(Clone, Copy)]
enum Outcome {
    Planned,
    PartiallyEaten { component_index: usize },
    Eaten { varied: bool },
    NotEaten,
}

struct StockSpec {
    product_key: &'static str,
    level: StockLevel,
    storage_location: StorageLocation,
    source_date: Option<SourceDate>,
    note: Option<&'static str>,
}

struct ProductSpec {
    key: &'static str,
    name: &'static str,
    brand: Option<&'static str>,
    ingredient_key: Option<&'static str>,
    package_quantity: Option<Quantity>,
    servings_per_pack: Option<i32>,
    nutrition: NutritionFacts,
}

pub async fn load(
    state: &AppState,
    actor_username: &str,
    scenario: Scenario,
    week_start: Date,
    today: Date,
) -> anyhow::Result<Report> {
    let actor = state
        .household
        .find_user_by_username(actor_username)
        .await?
        .with_context(|| format!("development account `{actor_username}` does not exist"))?;
    let member = state
        .household
        .find_member_by_linked_user(actor.id)
        .await?
        .context("the development account is not linked to a household member")?;
    let mut loader = Loader {
        state,
        actor,
        member,
        week_start,
        today,
        report: Report::default(),
    };

    loader.load_accounts().await?;
    loader.load_products().await?;
    loader.load_recipes().await?;
    loader.load_targets().await?;
    loader.load_stock().await?;

    match scenario {
        Scenario::Minimal => loader.load_minimal().await?,
        Scenario::Full => loader.load_full().await?,
    }

    Ok(loader.report)
}

impl Loader<'_> {
    async fn load_accounts(&mut self) -> anyhow::Result<()> {
        let manager = self
            .ensure_user(
                "manager",
                "sample.manager",
                "Morgan Sample",
                vec![Role::HouseholdManager],
            )
            .await?;
        self.ensure_member("manager", "Morgan Sample", manager.id)
            .await?;

        let basic = self
            .ensure_user(
                "basic-user",
                "sample.user",
                "Taylor Sample",
                vec![Role::BasicUser],
            )
            .await?;
        self.ensure_member("basic-user", "Taylor Sample", basic.id)
            .await?;

        let nutritionist = self
            .ensure_user(
                "nutritionist",
                "sample.nutritionist",
                "Casey Sample",
                vec![Role::Nutritionist],
            )
            .await?;
        self.state
            .household
            .grant_access(
                self.member.id,
                nutritionist.id,
                AccessScope::HealthData,
                Some(self.actor.id),
            )
            .await?;
        Ok(())
    }

    async fn ensure_user(
        &mut self,
        key: &str,
        username: &str,
        display_name: &str,
        roles: Vec<Role>,
    ) -> anyhow::Result<User> {
        let id = UserId::from_uuid(sample_uuid("user", key));
        let mut user = match self.state.household.get_user(id).await {
            Ok(user) => user,
            Err(CoreError::NotFound { .. }) => {
                if let Some(user) = self.state.household.find_user_by_username(username).await? {
                    user
                } else {
                    self.report.users_created += 1;
                    self.state
                        .household
                        .create_user(NewUser {
                            id: Some(id),
                            username: username.to_owned(),
                            display_name: Some(display_name.to_owned()),
                            roles: roles.clone(),
                        })
                        .await?
                }
            }
            Err(error) => return Err(error.into()),
        };
        if user.roles != roles {
            user = self
                .state
                .household
                .set_user_roles(user.id, user.revision, roles)
                .await?;
        }
        Ok(user)
    }

    async fn ensure_member(
        &mut self,
        key: &str,
        display_name: &str,
        user_id: UserId,
    ) -> anyhow::Result<HouseholdMember> {
        if let Some(member) = self
            .state
            .household
            .find_member_by_linked_user(user_id)
            .await?
        {
            return Ok(member);
        }

        let id = HouseholdMemberId::from_uuid(sample_uuid("household-member", key));
        let member = match self.state.household.get_member(id).await {
            Ok(member) => {
                self.state
                    .household
                    .link_account(member.id, member.revision, user_id)
                    .await?
            }
            Err(CoreError::NotFound { .. }) => {
                self.report.members_created += 1;
                self.state
                    .household
                    .create_member(NewHouseholdMember {
                        id: Some(id),
                        display_name: display_name.to_owned(),
                        linked_user_id: Some(user_id),
                    })
                    .await?
            }
            Err(error) => return Err(error.into()),
        };
        Ok(member)
    }

    async fn load_products(&mut self) -> anyhow::Result<()> {
        for spec in product_specs() {
            let id = product_id(spec.key);
            match self.state.catalogue.get_product(id).await {
                Ok(_) => continue,
                Err(CoreError::NotFound { .. }) => {}
                Err(error) => return Err(error.into()),
            }
            self.state
                .catalogue
                .create_product(NewProduct {
                    id: Some(id),
                    name: spec.name.to_owned(),
                    brand: spec.brand.map(str::to_owned),
                    barcode: None,
                    retailer: Some("Sample Supermarket".to_owned()),
                    shopping_section: Some("Sample data".to_owned()),
                    package_quantity: spec.package_quantity,
                    servings_per_pack: spec.servings_per_pack,
                    mapped_ingredient_id: spec.ingredient_key.map(IngredientId::seeded),
                    nutrition: spec.nutrition,
                    provenance: Provenance::local(),
                })
                .await?;
            self.report.products_created += 1;
        }
        Ok(())
    }

    async fn load_recipes(&mut self) -> anyhow::Result<()> {
        for spec in recipe_specs() {
            let id = recipe_id(spec.key);
            let countries: Vec<String> = spec
                .country_categories
                .iter()
                .map(|country| (*country).to_owned())
                .collect();
            let tags: Vec<String> = spec.tags.iter().map(|tag| (*tag).to_owned()).collect();
            let instructions: Vec<NewRecipeInstruction> = spec
                .instructions
                .iter()
                .map(|text| NewRecipeInstruction {
                    id: None,
                    text: (*text).to_owned(),
                })
                .collect();
            let existing = match self.state.recipes.get_recipe(id, self.actor.id).await {
                Ok(recipe) => Some(recipe),
                Err(CoreError::NotFound { .. }) => None,
                Err(error) => return Err(error.into()),
            };
            let recipe = if let Some(mut recipe) = existing {
                let patch = RecipePatch {
                    description: Patch::Set(spec.description.to_owned()),
                    preparation_minutes: Patch::Set(spec.preparation_minutes),
                    cooking_minutes: Patch::Set(spec.cooking_minutes),
                    notes: Patch::Set(spec.notes.to_owned()),
                    instructions: Some(instructions.clone()),
                    meal_categories: Some(spec.meal_categories.clone()),
                    country_categories: Some(countries.clone()),
                    tags: Some(tags.clone()),
                    ..RecipePatch::default()
                };
                if recipe.description.as_deref() != Some(spec.description)
                    || recipe.preparation_minutes != Some(spec.preparation_minutes)
                    || recipe.cooking_minutes != Some(spec.cooking_minutes)
                    || recipe.notes.as_deref() != Some(spec.notes)
                    || recipe
                        .instructions
                        .iter()
                        .map(|step| step.text.as_str())
                        .collect::<Vec<_>>()
                        != spec.instructions
                    || recipe.meal_categories != spec.meal_categories
                    || recipe.country_categories != countries
                    || recipe.tags != tags
                {
                    recipe = self
                        .state
                        .recipes
                        .update_recipe(id, recipe.revision, patch, self.actor.id)
                        .await?;
                }
                recipe
            } else {
                let recipe = self
                    .state
                    .recipes
                    .create_recipe(NewRecipe {
                        id: Some(id),
                        name: spec.name.to_owned(),
                        description: Some(spec.description.to_owned()),
                        servings: spec.servings,
                        preparation_minutes: Some(spec.preparation_minutes),
                        cooking_minutes: Some(spec.cooking_minutes),
                        notes: Some(spec.notes.to_owned()),
                        components: spec
                            .components
                            .iter()
                            .map(|(line, amount)| NewRecipeComponent {
                                id: None,
                                requirement: line.requirement(),
                                amount: *amount,
                            })
                            .collect(),
                        instructions,
                        meal_categories: spec.meal_categories.clone(),
                        country_categories: countries,
                        tags,
                        actor_id: self.actor.id,
                    })
                    .await?;
                self.report.recipes_created += 1;
                recipe
            };
            if spec.photo && recipe.photo_version.is_none() {
                let derivatives = mmp_server::photo::process(SAMPLE_RECIPE_IMAGE)
                    .map_err(|error| anyhow::anyhow!(error))?;
                self.state
                    .recipes
                    .replace_photo(recipe.id, recipe.revision, derivatives, self.actor.id)
                    .await?;
            }
        }
        Ok(())
    }

    async fn load_targets(&mut self) -> anyhow::Result<()> {
        self.ensure_target(
            self.week_start - Duration::weeks(6),
            nutrition_goals([2000, 100, 240, 70, 65, 20, 30, 6, 300]),
        )
        .await?;
        self.ensure_target(
            self.week_start - Duration::weeks(2),
            nutrition_goals([2200, 120, 260, 75, 70, 22, 35, 6, 300]),
        )
        .await
    }

    async fn ensure_target(
        &mut self,
        effective_from: Date,
        goals: NutritionGoals,
    ) -> anyhow::Result<()> {
        let existing = self.state.nutrition_targets.list(self.member.id).await?;
        if existing
            .iter()
            .any(|target| target.effective_from == effective_from)
        {
            return Ok(());
        }
        self.state
            .nutrition_targets
            .create(NewNutritionTarget {
                member_id: self.member.id,
                effective_from,
                goals,
            })
            .await?;
        self.report.targets_created += 1;
        Ok(())
    }

    async fn load_stock(&mut self) -> anyhow::Result<()> {
        let use_by = SourceDate {
            date: self.week_start + Duration::days(2),
            kind: SourceDateKind::UseBy,
        };
        let specs: Vec<StockSpec> = vec![
            StockSpec {
                product_key: "chicken-breast",
                level: StockLevel::Exact {
                    quantity: quantity(400, Unit::Gram),
                },
                storage_location: StorageLocation::Chilled,
                source_date: Some(use_by),
                note: Some("back left of the fridge"),
            },
            StockSpec {
                product_key: "chicken-breast",
                level: StockLevel::Exact {
                    quantity: quantity(650, Unit::Gram),
                },
                storage_location: StorageLocation::Frozen,
                source_date: None,
                note: Some("frozen flat, bought in bulk"),
            },
            StockSpec {
                product_key: "rolled-oats",
                level: StockLevel::Exact {
                    quantity: quantity(500, Unit::Gram),
                },
                storage_location: StorageLocation::Ambient,
                source_date: None,
                note: None,
            },
            StockSpec {
                product_key: "whole-milk",
                level: StockLevel::Estimated {
                    low: Decimal::new(300, 0),
                    high: Decimal::new(1500, 0),
                    unit: Unit::Millilitre,
                },
                storage_location: StorageLocation::Chilled,
                source_date: None,
                note: Some("guessing from the weight of the bottle"),
            },
            StockSpec {
                product_key: "broccoli",
                level: StockLevel::Exact {
                    quantity: quantity(120, Unit::Gram),
                },
                storage_location: StorageLocation::Chilled,
                source_date: None,
                note: None,
            },
            StockSpec {
                product_key: "basmati-rice",
                level: StockLevel::NotTracked,
                storage_location: StorageLocation::Ambient,
                source_date: None,
                note: None,
            },
        ];

        let mut expected_per_product: std::collections::HashMap<&str, i64> =
            std::collections::HashMap::new();
        for spec in &specs {
            *expected_per_product.entry(spec.product_key).or_default() += 1;
        }

        for StockSpec {
            product_key,
            level,
            storage_location,
            source_date,
            note,
        } in specs
        {
            let product = product_id(product_key);
            let existing = self
                .state
                .stock
                .list(&mmp_core::ports::StockQuery {
                    product_id: Some(product),
                    ..Default::default()
                })
                .await?;
            if existing.total >= expected_per_product[product_key] {
                continue;
            }
            self.state
                .stock
                .create(
                    NewStockItem {
                        product_id: product,
                        level,
                        storage_location,
                        source_date,
                        usability_deadline: None,
                        note: note.map(str::to_owned),
                    },
                    self.actor.id,
                    Some(self.member.id),
                )
                .await?;
            self.report.stock_items_created += 1;
        }
        Ok(())
    }

    async fn load_minimal(&mut self) -> anyhow::Result<()> {
        for slot in MealSlot::ALL {
            self.ensure_meal(self.week_start, slot, Outcome::Planned)
                .await?;
        }
        Ok(())
    }

    async fn load_full(&mut self) -> anyhow::Result<()> {
        for week_offset in [-3_i64, -2] {
            let week = self.week_start + Duration::weeks(week_offset);
            for day_offset in 0..7 {
                let date = week + Duration::days(day_offset);
                for slot in MealSlot::ALL {
                    self.ensure_meal(date, slot, Outcome::Eaten { varied: false })
                        .await?;
                }
            }
        }

        self.load_previous_partial_week().await?;
        self.load_current_partial_week().await?;
        self.load_household_meals().await
    }

    async fn load_household_meals(&mut self) -> anyhow::Result<()> {
        let manager = HouseholdMemberId::from_uuid(sample_uuid("household-member", "manager"));
        let basic = HouseholdMemberId::from_uuid(sample_uuid("household-member", "basic-user"));
        let owner = self.member.id;

        let thursday = self.week_start + Duration::days(3);
        self.ensure_household_meal(
            thursday,
            MealSlot::Lunch,
            "chicken-and-rice",
            servings(4),
            &[(owner, 1), (manager, 1), (basic, 1)],
            1,
        )
        .await?;

        let friday = self.week_start + Duration::days(4);
        self.ensure_personal_meal(friday, MealSlot::Lunch, owner, "self-catered")
            .await?;
        self.ensure_household_meal(
            friday,
            MealSlot::Lunch,
            "chicken-and-rice",
            servings(2),
            &[(manager, 1), (basic, 1)],
            0,
        )
        .await?;

        let saturday = self.week_start + Duration::days(5);
        self.ensure_household_meal(
            saturday,
            MealSlot::Lunch,
            "chicken-and-rice",
            servings(3),
            &[(owner, 1), (manager, 1), (basic, 1)],
            0,
        )
        .await?;
        self.ensure_opt_out_with_own_meal(saturday, MealSlot::Lunch, owner)
            .await?;

        let sunday = self.week_start + Duration::days(6);
        self.ensure_household_meal(
            sunday,
            MealSlot::Lunch,
            "chicken-and-rice",
            servings(4),
            &[(owner, 1), (manager, 1), (basic, 1)],
            2,
        )
        .await?;
        self.ensure_opt_out_with_own_meal(sunday, MealSlot::Lunch, basic)
            .await?;

        let next_wed = self.week_start + Duration::weeks(1) + Duration::days(2);
        self.ensure_household_meal(
            next_wed,
            MealSlot::Dinner,
            "chicken-and-rice",
            servings(3),
            &[(owner, 1), (manager, 1), (basic, 1)],
            0,
        )
        .await?;

        Ok(())
    }

    async fn ensure_opt_out_with_own_meal(
        &mut self,
        date: Date,
        slot: MealSlot,
        member_id: HouseholdMemberId,
    ) -> anyhow::Result<()> {
        let entry_id = meal_id(date, slot);
        let view = self.state.meal_plan.get(entry_id).await?;
        if view.entry.has_opted_out(member_id) {
            return Ok(());
        }
        self.state
            .meal_plan
            .opt_out(entry_id, view.entry.revision, self.actor.id, member_id)
            .await?;
        self.report.household_participants_created =
            self.report.household_participants_created.saturating_sub(1);
        self.ensure_personal_meal(date, slot, member_id, "opted-out")
            .await
    }

    async fn ensure_personal_meal(
        &mut self,
        date: Date,
        slot: MealSlot,
        member_id: HouseholdMemberId,
        key: &str,
    ) -> anyhow::Result<()> {
        let id = MealPlanEntryId::from_uuid(sample_uuid(
            "meal-plan-entry",
            &format!("{date}:{slot}:member:{key}"),
        ));
        if !matches!(
            self.state.meal_plan.get(id).await,
            Err(CoreError::NotFound { .. })
        ) {
            return Ok(());
        }
        self.report.meals_created += 1;
        self.state
            .meal_plan
            .create_unchecked(NewMealPlanEntry {
                id: Some(id),
                scope: MealPlanScope::Member,
                member_id: Some(member_id),
                planned_on: date,
                planned_time: slot_time(slot),
                slot,
                portioning: mmp_core::domain::Portioning::Equal,
                components: components_for(slot),
                participants: None,
                guest_groups: Vec::new(),
                actor_id: self.actor.id,
            })
            .await?;
        Ok(())
    }

    async fn ensure_household_meal(
        &mut self,
        date: Date,
        slot: MealSlot,
        recipe_key: &str,
        prepared: ConsumedAmount,
        allocations: &[(HouseholdMemberId, i64)],
        guest_count: i32,
    ) -> anyhow::Result<()> {
        let id = meal_id(date, slot);
        if !matches!(
            self.state.meal_plan.get(id).await,
            Err(CoreError::NotFound { .. })
        ) {
            return Ok(());
        }
        self.report.meals_created += 1;
        let component_id = mmp_core::domain::MealPlanComponentId::new();
        let participants = allocations
            .iter()
            .map(|(member_id, count)| NewMealParticipant {
                id: None,
                member_id: *member_id,
                allocations: vec![NewMealParticipantAllocation {
                    component_id,
                    allocated: servings(*count),
                }],
            })
            .collect();
        self.state
            .meal_plan
            .create_unchecked(NewMealPlanEntry {
                id: Some(id),
                scope: MealPlanScope::Household,
                member_id: None,
                planned_on: date,
                planned_time: slot_time(slot),
                slot,
                portioning: mmp_core::domain::Portioning::Equal,
                components: vec![NewMealPlanComponent {
                    id: Some(component_id),
                    item: MealItemRef::recipe(recipe_id(recipe_key)),
                    amount: prepared,
                }],
                participants: Some(participants),
                guest_groups: if guest_count > 0 {
                    vec![NewMealGuestGroup {
                        id: None,
                        count: guest_count,
                        allocations: vec![NewMealGuestAllocation {
                            component_id,
                            allocated: servings(1),
                        }],
                    }]
                } else {
                    Vec::new()
                },
                actor_id: self.actor.id,
            })
            .await?;
        self.report.household_participants_created += allocations.len();
        Ok(())
    }

    async fn load_previous_partial_week(&mut self) -> anyhow::Result<()> {
        let week = self.week_start - Duration::weeks(1);
        for day_offset in 0..2 {
            let date = week + Duration::days(day_offset);
            for slot in MealSlot::ALL {
                self.ensure_meal(date, slot, Outcome::Eaten { varied: true })
                    .await?;
            }
        }

        let wednesday = week + Duration::days(2);
        self.ensure_meal(
            wednesday,
            MealSlot::Breakfast,
            Outcome::PartiallyEaten { component_index: 1 },
        )
        .await?;
        self.ensure_meal(wednesday, MealSlot::Lunch, Outcome::Eaten { varied: true })
            .await?;
        self.ensure_meal(wednesday, MealSlot::Dinner, Outcome::NotEaten)
            .await?;

        let thursday = week + Duration::days(3);
        self.ensure_diary_entry(
            thursday,
            MealSlot::Lunch,
            "bakery-lunch",
            "bakery-lunch",
            servings(1),
        )
        .await?;
        self.ensure_diary_entry(
            thursday,
            MealSlot::Snacks,
            "mystery-snack",
            "mystery-snack",
            servings(1),
        )
        .await?;

        let friday = week + Duration::days(4);
        self.ensure_meal(friday, MealSlot::Dinner, Outcome::Planned)
            .await
    }

    async fn load_current_partial_week(&mut self) -> anyhow::Result<()> {
        let week_end = self.week_start + Duration::days(6);

        let mut date = self.week_start;
        while date < self.today {
            for slot in MealSlot::ALL {
                self.ensure_meal(date, slot, Outcome::Eaten { varied: true })
                    .await?;
            }
            date += Duration::days(1);
        }

        self.ensure_meal(
            self.today,
            MealSlot::Breakfast,
            Outcome::Eaten { varied: true },
        )
        .await?;
        self.ensure_meal(self.today, MealSlot::Lunch, Outcome::Eaten { varied: true })
            .await?;
        self.ensure_meal(self.today, MealSlot::Dinner, Outcome::Planned)
            .await?;
        self.ensure_meal(self.today, MealSlot::Snacks, Outcome::Planned)
            .await?;
        self.ensure_timed_snack(
            self.today,
            "morning",
            Time::from_hms(10, 0, 0).unwrap(),
            "banana",
            measured(1, Unit::Item),
        )
        .await?;
        self.ensure_timed_snack(
            self.today,
            "afternoon",
            Time::from_hms(14, 0, 0).unwrap(),
            "mystery-snack",
            measured(1, Unit::Item),
        )
        .await?;

        self.ensure_diary_entry(
            self.today,
            MealSlot::Snacks,
            "extra-snack",
            "greek-yoghurt",
            servings(1),
        )
        .await?;
        self.ensure_recipe_diary_entry(
            self.today,
            MealSlot::Lunch,
            "leftover-porridge",
            "porridge",
            servings(1),
        )
        .await?;

        let mut date = self.today + Duration::days(1);
        while date <= week_end {
            for slot in [MealSlot::Breakfast, MealSlot::Dinner] {
                self.ensure_meal(date, slot, Outcome::Planned).await?;
            }
            date += Duration::days(1);
        }

        let recipe_day = self.today + Duration::days(1);
        if recipe_day <= week_end {
            self.ensure_recipe_meal(recipe_day, MealSlot::Lunch, "chicken-and-rice", servings(1))
                .await?;
        }

        Ok(())
    }

    async fn ensure_timed_snack(
        &mut self,
        date: Date,
        key: &str,
        planned_time: Time,
        product_key: &str,
        amount: ConsumedAmount,
    ) -> anyhow::Result<()> {
        let id = snack_id(date, key);
        if !matches!(
            self.state.meal_plan.get(id).await,
            Err(CoreError::NotFound { .. })
        ) {
            return Ok(());
        }
        self.report.meals_created += 1;
        self.state
            .meal_plan
            .create_unchecked(NewMealPlanEntry {
                id: Some(id),
                scope: MealPlanScope::Member,
                member_id: Some(self.member.id),
                planned_on: date,
                planned_time: Some(planned_time),
                slot: MealSlot::Snacks,
                portioning: mmp_core::domain::Portioning::Equal,
                components: vec![NewMealPlanComponent {
                    id: None,
                    item: MealItemRef::product(product_id(product_key)),
                    amount,
                }],
                participants: None,
                guest_groups: Vec::new(),
                actor_id: self.actor.id,
            })
            .await?;
        Ok(())
    }

    async fn ensure_meal(
        &mut self,
        date: Date,
        slot: MealSlot,
        outcome: Outcome,
    ) -> anyhow::Result<()> {
        let outcome = if date > self.today {
            Outcome::Planned
        } else {
            outcome
        };
        let id = meal_id(date, slot);
        let mut view = match self.state.meal_plan.get(id).await {
            Ok(view) => view,
            Err(CoreError::NotFound { .. }) => {
                self.report.meals_created += 1;
                self.state
                    .meal_plan
                    .create_unchecked(NewMealPlanEntry {
                        id: Some(id),
                        scope: MealPlanScope::Member,
                        member_id: Some(self.member.id),
                        planned_on: date,
                        planned_time: slot_time(slot),
                        slot,
                        portioning: mmp_core::domain::Portioning::Equal,
                        components: components_for(slot),
                        participants: None,
                        guest_groups: Vec::new(),
                        actor_id: self.actor.id,
                    })
                    .await?
            }
            Err(error) => return Err(error.into()),
        };

        let desired = match outcome {
            Outcome::Planned => MealPlanStatus::Planned,
            Outcome::PartiallyEaten { .. } => MealPlanStatus::PartiallyResolved,
            Outcome::Eaten { .. } => MealPlanStatus::Eaten,
            Outcome::NotEaten => MealPlanStatus::NotEaten,
        };
        if view.entry.status() == desired {
            return Ok(());
        }
        if view.entry.status() != MealPlanStatus::Planned {
            view = self
                .state
                .meal_plan
                .reopen(
                    view.entry.id,
                    view.entry.revision,
                    OutcomeActor::own(self.actor.id),
                )
                .await?
                .into_value();
        }

        match outcome {
            Outcome::Planned => {}
            Outcome::PartiallyEaten { component_index } => {
                let component = view
                    .components
                    .get(component_index)
                    .context("sample partial meal component does not exist")?;
                self.state
                    .meal_plan
                    .mark_component_eaten_unchecked(
                        view.entry.id,
                        component.component.id,
                        component.component.revision,
                        ConfirmMealPlanComponent {
                            consumed_on: date,
                            consumed_at: slot_time(slot)
                                .map(|time| PrimitiveDateTime::new(date, time).assume_utc()),
                            amount: component.component.amount,
                            actor_id: self.actor.id,
                            subject_member_id: None,
                        },
                    )
                    .await?;
                self.report.meals_resolved += 1;
            }
            Outcome::NotEaten => {
                self.state
                    .meal_plan
                    .mark_not_eaten_unchecked(
                        view.entry.id,
                        view.entry.revision,
                        OutcomeActor::own(self.actor.id),
                    )
                    .await?;
                self.report.meals_resolved += 1;
            }
            Outcome::Eaten { varied } => {
                let components = view
                    .entry
                    .components
                    .iter()
                    .enumerate()
                    .map(|(index, component)| ActualMealPlanComponent {
                        component_id: component.id,
                        amount: if varied {
                            vary_amount(component.amount, index)
                        } else {
                            component.amount
                        },
                    })
                    .collect();
                let resolved = self
                    .state
                    .meal_plan
                    .mark_eaten_unchecked(
                        view.entry.id,
                        view.entry.revision,
                        ConfirmMealPlanEntry {
                            consumed_on: date,
                            consumed_at: slot_time(slot)
                                .map(|time| PrimitiveDateTime::new(date, time).assume_utc()),
                            components,
                            actor_id: self.actor.id,
                            subject_member_id: None,
                        },
                    )
                    .await?;
                self.report.meals_resolved += 1;
                self.count_stock_effects(&resolved.entry).await?;
            }
        }
        Ok(())
    }

    async fn count_stock_effects(
        &mut self,
        entry: &mmp_core::domain::MealPlanEntry,
    ) -> anyhow::Result<()> {
        for component in &entry.components {
            let effects = self
                .state
                .stock
                .effects_for_source(
                    mmp_core::domain::StockEffectSource::MealPlanComponent,
                    component.id.as_uuid(),
                )
                .await?;
            self.report.stock_effects_applied += effects
                .iter()
                .filter(|effect| effect.state == mmp_core::domain::StockEffectState::Applied)
                .count();
        }
        Ok(())
    }

    async fn ensure_recipe_meal(
        &mut self,
        date: Date,
        slot: MealSlot,
        recipe_key: &str,
        amount: ConsumedAmount,
    ) -> anyhow::Result<()> {
        let id = meal_id(date, slot);
        if !matches!(
            self.state.meal_plan.get(id).await,
            Err(CoreError::NotFound { .. })
        ) {
            return Ok(());
        }
        self.report.meals_created += 1;
        self.state
            .meal_plan
            .create_unchecked(NewMealPlanEntry {
                id: Some(id),
                scope: MealPlanScope::Member,
                member_id: Some(self.member.id),
                planned_on: date,
                planned_time: slot_time(slot),
                slot,
                portioning: mmp_core::domain::Portioning::Equal,
                components: vec![NewMealPlanComponent {
                    id: None,
                    item: MealItemRef::recipe(recipe_id(recipe_key)),
                    amount,
                }],
                participants: None,
                guest_groups: Vec::new(),
                actor_id: self.actor.id,
            })
            .await?;
        Ok(())
    }

    async fn ensure_recipe_diary_entry(
        &mut self,
        date: Date,
        slot: MealSlot,
        key: &str,
        recipe_key: &str,
        amount: ConsumedAmount,
    ) -> anyhow::Result<()> {
        let id = ConsumptionRecordId::from_uuid(sample_uuid(
            "consumption-record",
            &format!("{date}:{key}"),
        ));
        match self.state.diary.get(id).await {
            Ok(_) => return Ok(()),
            Err(CoreError::NotFound { .. }) => {}
            Err(error) => return Err(error.into()),
        }
        self.state
            .diary
            .record_unchecked(NewConsumptionRecord {
                id: Some(id),
                member_id: self.member.id,
                item: MealItemRef::recipe(recipe_id(recipe_key)),
                recorded_by: Some(self.actor.id),
                meal_plan_entry_id: None,
                meal_plan_component_id: None,
                slot,
                amount,
                consumed_on: date,
                consumed_at: Some(
                    PrimitiveDateTime::new(date, Time::from_hms(12, 30, 0).unwrap()).assume_utc(),
                ),
            })
            .await?;
        self.report.diary_entries_created += 1;
        Ok(())
    }

    async fn ensure_diary_entry(
        &mut self,
        date: Date,
        slot: MealSlot,
        key: &str,
        product_key: &str,
        amount: ConsumedAmount,
    ) -> anyhow::Result<()> {
        let id = ConsumptionRecordId::from_uuid(sample_uuid(
            "consumption-record",
            &format!("{date}:{key}"),
        ));
        match self.state.diary.get(id).await {
            Ok(_) => return Ok(()),
            Err(CoreError::NotFound { .. }) => {}
            Err(error) => return Err(error.into()),
        }
        self.state
            .diary
            .record_unchecked(NewConsumptionRecord {
                id: Some(id),
                member_id: self.member.id,
                item: MealItemRef::product(product_id(product_key)),
                recorded_by: Some(self.actor.id),
                meal_plan_entry_id: None,
                meal_plan_component_id: None,
                slot,
                amount,
                consumed_on: date,
                consumed_at: Some(
                    PrimitiveDateTime::new(date, Time::from_hms(15, 30, 0).unwrap()).assume_utc(),
                ),
            })
            .await?;
        self.report.diary_entries_created += 1;
        Ok(())
    }
}

fn product_specs() -> Vec<ProductSpec> {
    vec![
        ProductSpec {
            key: "rolled-oats",
            name: "Sample Jumbo Oats",
            brand: Some("Sample Pantry"),
            ingredient_key: Some("rolled-oats"),
            package_quantity: Some(quantity(1000, Unit::Gram)),
            servings_per_pack: Some(12),
            nutrition: nutrition(100, Unit::Gram, [389, 17, 66, 1, 7, 1, 11, 0, 0]),
        },
        ProductSpec {
            key: "whole-milk",
            name: "Sample Whole Milk",
            brand: Some("Sample Dairy"),
            ingredient_key: Some("whole-milk"),
            package_quantity: Some(quantity(2000, Unit::Millilitre)),
            servings_per_pack: Some(8),
            nutrition: nutrition(100, Unit::Millilitre, [64, 3, 5, 5, 4, 2, 0, 0, 10]),
        },
        ProductSpec {
            key: "whole-milk-value",
            name: "Sample Value Whole Milk",
            brand: Some("Sample Basics"),
            ingredient_key: Some("whole-milk"),
            package_quantity: Some(quantity(1000, Unit::Millilitre)),
            servings_per_pack: Some(4),
            nutrition: nutrition(100, Unit::Millilitre, [61, 3, 5, 5, 3, 2, 0, 0, 11]),
        },
        ProductSpec {
            key: "banana",
            name: "Sample Bananas",
            brand: None,
            ingredient_key: Some("banana"),
            package_quantity: Some(quantity(6, Unit::Item)),
            servings_per_pack: Some(6),
            nutrition: nutrition(1, Unit::Item, [105, 1, 27, 14, 0, 0, 3, 0, 0]),
        },
        ProductSpec {
            key: "chicken-breast",
            name: "Sample Chicken Breast Fillets",
            brand: Some("Sample Fresh"),
            ingredient_key: Some("chicken-breast"),
            package_quantity: Some(quantity(600, Unit::Gram)),
            servings_per_pack: Some(4),
            nutrition: nutrition(100, Unit::Gram, [165, 31, 0, 0, 4, 1, 0, 1, 85]),
        },
        ProductSpec {
            key: "basmati-rice",
            name: "Sample Cooked Basmati Rice",
            brand: Some("Sample Pantry"),
            ingredient_key: Some("basmati-rice"),
            package_quantity: Some(quantity(500, Unit::Gram)),
            servings_per_pack: Some(5),
            nutrition: nutrition(100, Unit::Gram, [130, 3, 28, 0, 0, 0, 0, 0, 0]),
        },
        ProductSpec {
            key: "salmon-fillet",
            name: "Sample Salmon Fillets",
            brand: Some("Sample Fresh"),
            ingredient_key: Some("salmon-fillet"),
            package_quantity: Some(quantity(400, Unit::Gram)),
            servings_per_pack: Some(2),
            nutrition: nutrition(100, Unit::Gram, [208, 20, 0, 0, 13, 3, 0, 0, 55]),
        },
        ProductSpec {
            key: "potato",
            name: "Sample White Potatoes",
            brand: None,
            ingredient_key: Some("potato"),
            package_quantity: Some(quantity(2500, Unit::Gram)),
            servings_per_pack: None,
            nutrition: nutrition(100, Unit::Gram, [77, 2, 17, 1, 0, 0, 2, 0, 0]),
        },
        ProductSpec {
            key: "broccoli",
            name: "Sample Broccoli",
            brand: None,
            ingredient_key: Some("broccoli"),
            package_quantity: Some(quantity(500, Unit::Gram)),
            servings_per_pack: None,
            nutrition: nutrition(100, Unit::Gram, [34, 3, 7, 2, 0, 0, 3, 0, 0]),
        },
        ProductSpec {
            key: "greek-yoghurt",
            name: "Sample Greek Yoghurt",
            brand: Some("Sample Dairy"),
            ingredient_key: Some("greek-yoghurt"),
            package_quantity: Some(quantity(500, Unit::Gram)),
            servings_per_pack: Some(4),
            nutrition: nutrition(100, Unit::Gram, [97, 9, 4, 4, 5, 3, 0, 0, 15]),
        },
        ProductSpec {
            key: "apple",
            name: "Sample Apples",
            brand: None,
            ingredient_key: Some("apple"),
            package_quantity: Some(quantity(6, Unit::Item)),
            servings_per_pack: Some(6),
            nutrition: nutrition(1, Unit::Item, [95, 0, 25, 19, 0, 0, 4, 0, 0]),
        },
        ProductSpec {
            key: "bakery-lunch",
            name: "Sample Bakery Lunch",
            brand: Some("Sample Bakery"),
            ingredient_key: None,
            package_quantity: Some(quantity(1, Unit::Item)),
            servings_per_pack: Some(1),
            nutrition: NutritionFacts {
                basis: Some(quantity(1, Unit::Item)),
                energy_kcal: Some(decimal(540)),
                carbohydrate_g: Some(decimal(62)),
                fat_g: Some(decimal(24)),
                ..Default::default()
            },
        },
        ProductSpec {
            key: "mystery-snack",
            name: "Sample Mystery Snack",
            brand: None,
            ingredient_key: None,
            package_quantity: Some(quantity(1, Unit::Item)),
            servings_per_pack: Some(1),
            nutrition: NutritionFacts::default(),
        },
    ]
}

fn components_for(slot: MealSlot) -> Vec<NewMealPlanComponent> {
    let values = match slot {
        MealSlot::Breakfast => vec![
            ("rolled-oats", measured(80, Unit::Gram)),
            ("whole-milk", measured(250, Unit::Millilitre)),
            ("banana", measured(1, Unit::Item)),
        ],
        MealSlot::Lunch => vec![
            ("chicken-breast", measured(150, Unit::Gram)),
            ("basmati-rice", measured(150, Unit::Gram)),
            ("broccoli", measured(100, Unit::Gram)),
        ],
        MealSlot::Dinner => vec![
            ("salmon-fillet", measured(150, Unit::Gram)),
            ("potato", measured(300, Unit::Gram)),
            ("broccoli", measured(150, Unit::Gram)),
        ],
        MealSlot::Snacks => vec![
            ("greek-yoghurt", servings(1)),
            ("apple", measured(1, Unit::Item)),
        ],
    };
    values
        .into_iter()
        .map(|(key, amount)| NewMealPlanComponent {
            id: None,
            item: MealItemRef::product(product_id(key)),
            amount,
        })
        .collect()
}

fn vary_amount(amount: ConsumedAmount, index: usize) -> ConsumedAmount {
    let factor = if index.is_multiple_of(2) {
        Decimal::new(9, 1)
    } else {
        Decimal::new(11, 1)
    };
    match amount {
        ConsumedAmount::Measure(quantity) => {
            ConsumedAmount::Measure(Quantity::new(quantity.amount * factor, quantity.unit))
        }
        ConsumedAmount::Servings(value) => ConsumedAmount::Servings(value * factor),
        ConsumedAmount::Packs(value) => ConsumedAmount::Packs(value * factor),
    }
}

fn sample_uuid(resource: &str, key: &str) -> Uuid {
    Uuid::new_v5(&SAMPLE_NAMESPACE, format!("{resource}:{key}").as_bytes())
}

fn product_id(key: &str) -> ProductId {
    ProductId::from_uuid(sample_uuid("product", key))
}

fn recipe_id(key: &str) -> RecipeId {
    RecipeId::from_uuid(sample_uuid("recipe", key))
}

enum RecipeLineSpec {
    Ingredient(&'static str),
    Product(&'static str),
    Unresolved(&'static str),
}

impl RecipeLineSpec {
    fn requirement(&self) -> RecipeRequirement {
        match self {
            RecipeLineSpec::Ingredient(key) => RecipeRequirement::Ingredient {
                ingredient_id: IngredientId::seeded(key),
            },
            RecipeLineSpec::Product(key) => RecipeRequirement::Product {
                product_id: product_id(key),
            },
            RecipeLineSpec::Unresolved(text) => RecipeRequirement::Unresolved {
                text: (*text).to_owned(),
            },
        }
    }
}

struct RecipeSpec {
    key: &'static str,
    name: &'static str,
    servings: i32,
    description: &'static str,
    preparation_minutes: i32,
    cooking_minutes: i32,
    notes: &'static str,
    components: Vec<(RecipeLineSpec, ConsumedAmount)>,
    instructions: Vec<&'static str>,
    meal_categories: Vec<MealCategory>,
    country_categories: Vec<&'static str>,
    tags: Vec<&'static str>,
    photo: bool,
}

fn recipe_specs() -> Vec<RecipeSpec> {
    vec![
        RecipeSpec {
            key: "porridge",
            name: "Morning Porridge",
            servings: 2,
            description: "Creamy oats with banana for a warm start to the day.",
            preparation_minutes: 5,
            cooking_minutes: 10,
            notes: "Add the banana just before serving.",
            components: vec![
                (
                    RecipeLineSpec::Ingredient("rolled-oats"),
                    ConsumedAmount::Measure(quantity(100, Unit::Gram)),
                ),
                (
                    RecipeLineSpec::Ingredient("whole-milk"),
                    ConsumedAmount::Measure(quantity(400, Unit::Millilitre)),
                ),
                (
                    RecipeLineSpec::Ingredient("banana"),
                    ConsumedAmount::Measure(quantity(1, Unit::Item)),
                ),
                (
                    RecipeLineSpec::Ingredient("cinnamon"),
                    ConsumedAmount::Measure(quantity(1, Unit::Teaspoon)),
                ),
            ],
            instructions: vec![
                "Add the oats and milk to a saucepan.",
                "Cook gently until creamy, stirring often.",
                "Slice the banana over the porridge, dust with cinnamon and serve.",
            ],
            meal_categories: vec![MealCategory::Breakfast],
            country_categories: vec!["GB"],
            tags: vec!["Quick", "Vegetarian"],
            photo: false,
        },
        RecipeSpec {
            key: "chicken-and-rice",
            name: "Chicken and Rice",
            servings: 4,
            description: "Tender chicken in a rich tomato sauce with fluffy basmati rice.",
            preparation_minutes: 10,
            cooking_minutes: 30,
            notes: "Rest the chicken for five minutes before serving.",
            components: vec![
                (
                    RecipeLineSpec::Product("chicken-breast"),
                    ConsumedAmount::Measure(quantity(600, Unit::Gram)),
                ),
                (
                    RecipeLineSpec::Ingredient("basmati-rice"),
                    ConsumedAmount::Measure(quantity(300, Unit::Gram)),
                ),
            ],
            instructions: vec![
                "Season the chicken and brown it in a hot pan.",
                "Add the sauce and simmer until the chicken is cooked through.",
                "Cook the basmati rice until tender.",
                "Rest the chicken, then serve with the rice.",
            ],
            meal_categories: vec![MealCategory::Dinner],
            country_categories: vec!["IN"],
            tags: vec!["Family favourite", "High protein"],
            photo: true,
        },
        RecipeSpec {
            key: "imported-rice-bowl",
            name: "Imported Rice Bowl",
            servings: 2,
            description: "A quick bowl imported from a friend's collection, still being tidied up.",
            preparation_minutes: 5,
            cooking_minutes: 15,
            notes: "Match the imported ingredient to your catalogue when you get a moment.",
            components: vec![
                (
                    RecipeLineSpec::Ingredient("basmati-rice"),
                    ConsumedAmount::Measure(quantity(200, Unit::Gram)),
                ),
                (
                    RecipeLineSpec::Unresolved("Jasmin Rice"),
                    ConsumedAmount::Measure(quantity(50, Unit::Gram)),
                ),
            ],
            instructions: vec![
                "Rinse the rice until the water runs clear.",
                "Simmer until tender, then fluff and serve.",
            ],
            meal_categories: vec![MealCategory::Lunch],
            country_categories: vec!["TH"],
            tags: vec!["Quick"],
            photo: false,
        },
    ]
}

fn meal_id(date: Date, slot: MealSlot) -> MealPlanEntryId {
    MealPlanEntryId::from_uuid(sample_uuid("meal-plan-entry", &format!("{date}:{slot}")))
}

fn snack_id(date: Date, key: &str) -> MealPlanEntryId {
    MealPlanEntryId::from_uuid(sample_uuid(
        "meal-plan-entry",
        &format!("{date}:snacks:{key}"),
    ))
}

fn slot_time(slot: MealSlot) -> Option<Time> {
    let (hour, minute) = match slot {
        MealSlot::Breakfast => (7, 30),
        MealSlot::Lunch => (12, 30),
        MealSlot::Dinner => (18, 30),
        MealSlot::Snacks => return None,
    };
    Some(Time::from_hms(hour, minute, 0).unwrap())
}

fn quantity(amount: i64, unit: Unit) -> Quantity {
    Quantity::new(decimal(amount), unit)
}

fn measured(amount: i64, unit: Unit) -> ConsumedAmount {
    ConsumedAmount::Measure(quantity(amount, unit))
}

fn servings(amount: i64) -> ConsumedAmount {
    ConsumedAmount::Servings(decimal(amount))
}

fn decimal(value: i64) -> Decimal {
    Decimal::new(value, 0)
}

fn nutrition(amount: i64, unit: Unit, values: [i64; 9]) -> NutritionFacts {
    NutritionFacts {
        basis: Some(quantity(amount, unit)),
        energy_kcal: Some(decimal(values[0])),
        protein_g: Some(decimal(values[1])),
        carbohydrate_g: Some(decimal(values[2])),
        sugar_g: Some(decimal(values[3])),
        fat_g: Some(decimal(values[4])),
        saturated_fat_g: Some(decimal(values[5])),
        fibre_g: Some(decimal(values[6])),
        salt_g: Some(decimal(values[7])),
        cholesterol_mg: Some(decimal(values[8])),
        ..Default::default()
    }
}

fn nutrition_goals(values: [i64; 9]) -> NutritionGoals {
    NutritionGoals {
        energy_kcal: Some(decimal(values[0])),
        protein_g: Some(decimal(values[1])),
        carbohydrate_g: Some(decimal(values[2])),
        sugar_g: Some(decimal(values[3])),
        fat_g: Some(decimal(values[4])),
        saturated_fat_g: Some(decimal(values[5])),
        fibre_g: Some(decimal(values[6])),
        salt_g: Some(decimal(values[7])),
        cholesterol_mg: Some(decimal(values[8])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn sample_identifiers_are_stable_and_separate_by_resource() {
        assert_eq!(product_id("apple"), product_id("apple"));
        assert_ne!(
            product_id("apple").as_uuid(),
            sample_uuid("ingredient", "apple")
        );
    }

    #[test]
    fn meal_identifiers_follow_the_absolute_date() {
        assert_eq!(
            meal_id(date!(2026 - 08 - 24), MealSlot::Breakfast),
            meal_id(date!(2026 - 08 - 24), MealSlot::Breakfast)
        );
        assert_ne!(
            meal_id(date!(2026 - 08 - 24), MealSlot::Breakfast),
            meal_id(date!(2026 - 08 - 25), MealSlot::Breakfast)
        );
    }

    #[test]
    fn the_product_set_includes_known_partial_and_unknown_nutrition() {
        let products = product_specs();
        assert!(products.iter().any(|product| {
            product
                .nutrition
                .named_values()
                .all(|(_, value)| value.is_some())
        }));
        assert!(products.iter().any(|product| {
            !product.nutrition.is_unknown()
                && product
                    .nutrition
                    .named_values()
                    .any(|(_, value)| value.is_none())
        }));
        assert!(
            products
                .iter()
                .any(|product| product.nutrition.is_unknown())
        );
    }
}
