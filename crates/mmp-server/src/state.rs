use std::sync::Arc;

use mmp_core::services::{CatalogueService, DiaryService, HouseholdService};

use crate::auth::AuthProvider;

#[derive(Clone)]
pub struct AppState {
    pub catalogue: CatalogueService,
    pub household: Arc<HouseholdService>,
    pub diary: DiaryService,
    pub auth: Arc<dyn AuthProvider>,
}

impl AppState {
    pub fn new(
        catalogue: CatalogueService,
        household: Arc<HouseholdService>,
        diary: DiaryService,
        auth: Arc<dyn AuthProvider>,
    ) -> Self {
        Self {
            catalogue,
            household,
            diary,
            auth,
        }
    }
}
