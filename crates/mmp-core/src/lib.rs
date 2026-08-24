// Domain model for M&MP
pub mod domain;
pub mod error;
pub mod ports;
pub mod services;

#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use error::{CoreError, RepositoryError, Result, ValidationError, ValidationErrors};
