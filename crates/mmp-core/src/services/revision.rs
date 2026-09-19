use std::fmt::Display;

use crate::domain::Revision;
use crate::error::{CoreError, Result};
use crate::ports::UpdateOutcome;

pub(super) fn require_revision(
    resource: &'static str,
    id: impl Display,
    expected: Revision,
    actual: Revision,
) -> Result<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(CoreError::RevisionMismatch {
            resource,
            id: id.to_string(),
            expected,
            actual,
        })
    }
}

pub(super) fn commit_outcome(
    resource: &'static str,
    id: impl Display,
    expected: Revision,
    outcome: UpdateOutcome,
) -> Result<()> {
    match outcome {
        UpdateOutcome::Updated => Ok(()),
        UpdateOutcome::RevisionMismatch { actual } => Err(CoreError::RevisionMismatch {
            resource,
            id: id.to_string(),
            expected,
            actual,
        }),
        UpdateOutcome::NotFound => Err(CoreError::not_found(resource, id.to_string())),
    }
}
