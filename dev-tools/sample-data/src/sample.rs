use std::fmt;
use std::str::FromStr;

use anyhow::{Context, bail};
use mmp_core::CoreError;
use mmp_core::domain::{
    AccessScope, ActualMealPlanComponent, ConfirmMealPlanComponent, ConfirmMealPlanEntry,
    ConsumedAmount, ConsumptionRecordId, HouseholdMember, HouseholdMemberId, IngredientId,
    MealCategory, MealItemRef, MealPlanEntryId, MealPlanStatus, MealSlot, NewConsumptionRecord,
    NewHouseholdMember, NewMealPlanComponent, NewMealPlanEntry, NewNutritionTarget, NewProduct,
    NewRecipe, NewRecipeComponent, NewRecipeInstruction, NewUser, NutritionFacts, NutritionGoals,
    Patch, ProductId, Provenance, Quantity, RecipeId, RecipePatch, Role, Unit, User, UserId,
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
    pub meals_created: usize,
    pub meals_resolved: usize,
    pub diary_entries_created: usize,
}

struct Loader<'a> {
    state: &'a AppState,
    actor: User,
    member: HouseholdMember,
    week_start: Date,
    report: Report,
}

#[derive(Clone, Copy)]
enum Outcome {
    Planned,
    PartiallyEaten { component_index: usize },
    Eaten { varied: bool },
    NotEaten,
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
        report: Report::default(),
    };

    loader.load_accounts().await?;
    loader.load_products().await?;
    loader.load_recipes().await?;
    loader.load_targets().await?;

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
                            .map(|(product_key, amount)| NewRecipeComponent {
                                id: None,
                                product_id: product_id(product_key),
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
        self.load_current_partial_week().await
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
        let monday = self.week_start;
        for slot in MealSlot::ALL {
            self.ensure_meal(monday, slot, Outcome::Eaten { varied: true })
                .await?;
        }

        let tuesday = self.week_start + Duration::days(1);
        self.ensure_meal(
            tuesday,
            MealSlot::Breakfast,
            Outcome::Eaten { varied: true },
        )
        .await?;
        self.ensure_meal(tuesday, MealSlot::Lunch, Outcome::Eaten { varied: true })
            .await?;
        self.ensure_meal(tuesday, MealSlot::Dinner, Outcome::Planned)
            .await?;
        self.ensure_diary_entry(
            tuesday,
            MealSlot::Snacks,
            "extra-snack",
            "greek-yoghurt",
            servings(1),
        )
        .await?;

        let wednesday = self.week_start + Duration::days(2);
        self.ensure_meal(wednesday, MealSlot::Breakfast, Outcome::Planned)
            .await?;
        self.ensure_recipe_meal(wednesday, MealSlot::Lunch, "chicken-and-rice", servings(1))
            .await?;
        self.ensure_meal(wednesday, MealSlot::Dinner, Outcome::Planned)
            .await?;
        self.ensure_recipe_diary_entry(
            tuesday,
            MealSlot::Lunch,
            "leftover-porridge",
            "porridge",
            servings(1),
        )
        .await?;

        for day_offset in 3..7 {
            let date = self.week_start + Duration::days(day_offset);
            for slot in [MealSlot::Breakfast, MealSlot::Dinner] {
                self.ensure_meal(date, slot, Outcome::Planned).await?;
            }
        }
        Ok(())
    }

    async fn ensure_meal(
        &mut self,
        date: Date,
        slot: MealSlot,
        outcome: Outcome,
    ) -> anyhow::Result<()> {
        let id = meal_id(date, slot);
        let mut view = match self.state.meal_plan.get(id).await {
            Ok(view) => view,
            Err(CoreError::NotFound { .. }) => {
                self.report.meals_created += 1;
                self.state
                    .meal_plan
                    .create_unchecked(NewMealPlanEntry {
                        id: Some(id),
                        member_id: self.member.id,
                        planned_on: date,
                        planned_time: slot_time(slot),
                        slot,
                        components: components_for(slot),
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
        if view.entry.status == desired {
            return Ok(());
        }
        if view.entry.status != MealPlanStatus::Planned {
            view = self
                .state
                .meal_plan
                .reopen(view.entry.id, view.entry.revision, self.actor.id)
                .await?;
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
                        },
                    )
                    .await?;
                self.report.meals_resolved += 1;
            }
            Outcome::NotEaten => {
                self.state
                    .meal_plan
                    .mark_not_eaten_unchecked(view.entry.id, view.entry.revision, self.actor.id)
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
                self.state
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
                        },
                    )
                    .await?;
                self.report.meals_resolved += 1;
            }
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
                member_id: self.member.id,
                planned_on: date,
                planned_time: slot_time(slot),
                slot,
                components: vec![NewMealPlanComponent {
                    id: None,
                    item: MealItemRef::recipe(recipe_id(recipe_key)),
                    amount,
                }],
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

struct RecipeSpec {
    key: &'static str,
    name: &'static str,
    servings: i32,
    description: &'static str,
    preparation_minutes: i32,
    cooking_minutes: i32,
    notes: &'static str,
    components: Vec<(&'static str, ConsumedAmount)>,
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
                    "rolled-oats",
                    ConsumedAmount::Measure(quantity(100, Unit::Gram)),
                ),
                (
                    "whole-milk",
                    ConsumedAmount::Measure(quantity(400, Unit::Millilitre)),
                ),
                ("banana", ConsumedAmount::Measure(quantity(1, Unit::Item))),
            ],
            instructions: vec![
                "Add the oats and milk to a saucepan.",
                "Cook gently until creamy, stirring often.",
                "Slice the banana over the porridge and serve.",
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
                    "chicken-breast",
                    ConsumedAmount::Measure(quantity(600, Unit::Gram)),
                ),
                (
                    "basmati-rice",
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
    ]
}

fn meal_id(date: Date, slot: MealSlot) -> MealPlanEntryId {
    MealPlanEntryId::from_uuid(sample_uuid("meal-plan-entry", &format!("{date}:{slot}")))
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
