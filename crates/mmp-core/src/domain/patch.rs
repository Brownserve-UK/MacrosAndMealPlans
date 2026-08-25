use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Patch<T> {
    #[default]
    Unchanged,
    Set(T),
    Clear,
}

impl<T> Patch<T> {
    pub fn is_unchanged(&self) -> bool {
        matches!(self, Patch::Unchanged)
    }

    pub fn apply(self, current: Option<T>) -> Option<T> {
        match self {
            Patch::Unchanged => current,
            Patch::Set(value) => Some(value),
            Patch::Clear => None,
        }
    }

    pub fn as_ref(&self) -> Patch<&T> {
        match self {
            Patch::Unchanged => Patch::Unchanged,
            Patch::Set(value) => Patch::Set(value),
            Patch::Clear => Patch::Clear,
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Patch<U> {
        match self {
            Patch::Unchanged => Patch::Unchanged,
            Patch::Set(value) => Patch::Set(f(value)),
            Patch::Clear => Patch::Clear,
        }
    }
}

impl<'de, T> Deserialize<'de> for Patch<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<T>::deserialize(deserializer)? {
            Some(value) => Ok(Patch::Set(value)),
            None => Ok(Patch::Clear),
        }
    }
}

#[cfg(test)]
#[path = "patch_tests.rs"]
mod tests;
