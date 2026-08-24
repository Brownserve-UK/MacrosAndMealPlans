use crate::domain::{Ingredient, IngredientId, Provenance, Revision, Unit, validate_name};
use crate::error::{Result, ValidationErrors};
use crate::services::CatalogueService;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SeedIngredient {
    pub seed_key: String,
    pub name: String,
    pub default_unit: Unit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SeedReport {
    pub created: usize,
    pub updated: usize,
    pub preserved: usize,
    pub conflicted: usize,
}

impl SeedReport {
    pub fn total(&self) -> usize {
        self.created + self.updated + self.preserved + self.conflicted
    }
}

impl CatalogueService {
    pub async fn apply_seed_ingredients(&self, seeds: &[SeedIngredient]) -> Result<SeedReport> {
        let mut report = SeedReport::default();

        for seed in seeds {
            let mut errors = ValidationErrors::new();
            validate_name("name", &seed.name, &mut errors);
            if seed.seed_key.trim().is_empty() {
                errors.push("seed_key", "Required");
            }
            errors.into_result()?;

            match self.find_ingredient_by_seed_key(&seed.seed_key).await? {
                None => match self.create_seeded_ingredient(seed).await {
                    Ok(()) => report.created += 1,
                    Err(crate::CoreError::Duplicate { .. }) => report.conflicted += 1,
                    Err(other) => return Err(other),
                },
                Some(existing) if existing.provenance.accepts_seed_refresh() => {
                    self.refresh_seeded_ingredient(existing, seed).await?;
                    report.updated += 1;
                }
                Some(_) => report.preserved += 1,
            }
        }

        Ok(report)
    }

    async fn create_seeded_ingredient(&self, seed: &SeedIngredient) -> Result<()> {
        let now = self.now();
        let ingredient = Ingredient {
            id: IngredientId::seeded(&seed.seed_key),
            name: seed.name.trim().to_owned(),
            default_unit: seed.default_unit,
            provenance: Provenance::seeded(&seed.seed_key),
            revision: Revision::INITIAL,
            created_at: now,
            updated_at: now,
            archived_at: None,
        };
        self.insert_ingredient(&ingredient).await
    }

    async fn refresh_seeded_ingredient(
        &self,
        mut existing: Ingredient,
        seed: &SeedIngredient,
    ) -> Result<()> {
        let expected = existing.revision;
        existing.name = seed.name.trim().to_owned();
        existing.default_unit = seed.default_unit;
        existing.revision = expected.next();
        existing.updated_at = self.now();
        self.commit_seeded_ingredient(&existing, expected).await
    }
}
