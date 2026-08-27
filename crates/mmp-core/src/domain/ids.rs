use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

macro_rules! entity_id {
    ($name:ident, $resource:literal) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub const RESOURCE: &'static str = $resource;

            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub const fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }

            pub fn seeded(seed_key: &str) -> Self {
                Self(seeded_uuid($resource, seed_key))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(Uuid::from_str(s)?))
            }
        }

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }
    };
}

entity_id!(IngredientId, "ingredient");
entity_id!(ProductId, "product");
entity_id!(UserId, "user");
entity_id!(HouseholdMemberId, "household_member");
entity_id!(ConsumptionRecordId, "consumption_record");
entity_id!(MealPlanEntryId, "meal_plan_entry");
entity_id!(MealPlanComponentId, "meal_plan_component");
entity_id!(NutritionTargetId, "nutrition_target");
entity_id!(RecipeId, "recipe");
entity_id!(RecipeComponentId, "recipe_component");
entity_id!(RecipeInstructionId, "recipe_instruction");

// This should give a stable UUID for a seeded resource
// so a seeded item remains identifiable
const SEED_NAMESPACE: Uuid = Uuid::from_u128(0x6d6d_7073_6565_6400_9b1e_4a2c_8f37_d510);

fn seeded_uuid(resource: &str, seed_key: &str) -> Uuid {
    Uuid::new_v5(&SEED_NAMESPACE, format!("{resource}:{seed_key}").as_bytes())
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct Revision(i64);

impl Revision {
    pub const INITIAL: Revision = Revision(1);

    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(&self) -> i64 {
        self.0
    }

    pub const fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
#[path = "ids_tests.rs"]
mod tests;
