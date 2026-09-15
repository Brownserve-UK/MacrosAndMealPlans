use super::str_enum::str_enum;

str_enum!(CatalogueOrigin, UnknownOrigin, "catalogue origin");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum CatalogueOrigin {
    Seeded,
    Local,
    External,
}

impl CatalogueOrigin {
    pub const ALL: [CatalogueOrigin; 3] = [
        CatalogueOrigin::Seeded,
        CatalogueOrigin::Local,
        CatalogueOrigin::External,
    ];

    pub const fn code(&self) -> &'static str {
        match self {
            CatalogueOrigin::Seeded => "seeded",
            CatalogueOrigin::Local => "local",
            CatalogueOrigin::External => "external",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Provenance {
    pub origin: CatalogueOrigin,
    pub seed_key: Option<String>,
    pub source_provider: Option<String>,
    pub source_external_id: Option<String>,
    pub locally_modified: bool,
}

impl Provenance {
    pub fn local() -> Self {
        Self {
            origin: CatalogueOrigin::Local,
            seed_key: None,
            source_provider: None,
            source_external_id: None,
            locally_modified: false,
        }
    }

    pub fn seeded(seed_key: impl Into<String>) -> Self {
        Self {
            origin: CatalogueOrigin::Seeded,
            seed_key: Some(seed_key.into()),
            source_provider: None,
            source_external_id: None,
            locally_modified: false,
        }
    }

    pub fn accepts_seed_refresh(&self) -> bool {
        self.origin == CatalogueOrigin::Seeded && !self.locally_modified
    }
}

impl Default for Provenance {
    fn default() -> Self {
        Self::local()
    }
}

#[cfg(test)]
#[path = "provenance_tests.rs"]
mod tests;
