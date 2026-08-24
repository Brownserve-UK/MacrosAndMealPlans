use mmp_core::domain::{CatalogueOrigin, Provenance, Quantity, Unit};
use mmp_core::ports::Paginated;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod iso_date {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::Date;
    use time::format_description::FormatItem;
    use time::macros::format_description;

    const FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");

    pub fn parse(raw: &str) -> Result<Date, time::error::Parse> {
        Date::parse(raw, FORMAT)
    }

    pub fn serialize<S: Serializer>(date: &Date, serializer: S) -> Result<S::Ok, S::Error> {
        let formatted = date.format(FORMAT).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Date, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Date::parse(&raw, FORMAT).map_err(serde::de::Error::custom)
    }

    pub mod option {
        use serde::{Deserialize, Deserializer, Serializer};
        use time::Date;

        pub fn serialize<S: Serializer>(
            date: &Option<Date>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            match date {
                Some(date) => super::serialize(date, serializer),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<Date>, D::Error> {
            let raw = Option::<String>::deserialize(deserializer)?;
            raw.map(|raw| Date::parse(&raw, super::FORMAT).map_err(serde::de::Error::custom))
                .transpose()
        }
    }
}

pub mod iso_time {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::Time;
    use time::format_description::FormatItem;
    use time::macros::format_description;

    const FORMAT: &[FormatItem<'_>] = format_description!("[hour]:[minute]");

    pub fn parse(raw: &str) -> Result<Time, time::error::Parse> {
        Time::parse(raw, FORMAT)
    }

    pub fn serialize<S: Serializer>(time: &Time, serializer: S) -> Result<S::Ok, S::Error> {
        let formatted = time.format(FORMAT).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Time, D::Error> {
        let raw = String::deserialize(deserializer)?;
        parse(&raw).map_err(serde::de::Error::custom)
    }

    pub mod option {
        use serde::{Deserialize, Deserializer, Serializer};
        use time::Time;

        pub fn serialize<S: Serializer>(
            time: &Option<Time>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            match time {
                Some(time) => super::serialize(time, serializer),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<Time>, D::Error> {
            let raw = Option::<String>::deserialize(deserializer)?;
            raw.map(|raw| super::parse(&raw).map_err(serde::de::Error::custom))
                .transpose()
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ProvenanceDto {
    pub origin: CatalogueOrigin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_external_id: Option<String>,
    pub locally_modified: bool,
}

impl From<Provenance> for ProvenanceDto {
    fn from(value: Provenance) -> Self {
        Self {
            origin: value.origin,
            seed_key: value.seed_key,
            source_provider: value.source_provider,
            source_external_id: value.source_external_id,
            locally_modified: value.locally_modified,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct QuantityDto {
    #[serde(with = "rust_decimal::serde::float")]
    #[schema(value_type = f64, example = 1.0)]
    pub amount: Decimal,
    #[schema(example = "l")]
    pub unit: Unit,
}

impl From<Quantity> for QuantityDto {
    fn from(value: Quantity) -> Self {
        Self {
            amount: value.amount,
            unit: value.unit,
        }
    }
}

impl From<QuantityDto> for Quantity {
    fn from(value: QuantityDto) -> Self {
        Quantity::new(value.amount, value.unit)
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PageMeta {
    #[schema(example = 1)]
    pub page: u32,
    #[schema(example = 25)]
    pub per_page: u32,
    #[schema(example = 137)]
    pub total: i64,
    #[schema(example = 6)]
    pub total_pages: u32,
}

impl PageMeta {
    pub fn of<T>(page: &Paginated<T>) -> Self {
        Self {
            page: page.page,
            per_page: page.per_page,
            total: page.total,
            total_pages: page.total_pages(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortDirectionDto {
    Asc,
    Desc,
}

impl From<SortDirectionDto> for mmp_core::ports::SortDirection {
    fn from(value: SortDirectionDto) -> Self {
        match value {
            SortDirectionDto::Asc => Self::Ascending,
            SortDirectionDto::Desc => Self::Descending,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UnitDto {
    #[schema(example = "g")]
    pub code: &'static str,
    #[schema(example = "gram")]
    pub label: &'static str,
    #[schema(example = "mass")]
    pub dimension: &'static str,
    pub convertible: bool,
}

impl UnitDto {
    pub fn all() -> Vec<UnitDto> {
        Unit::ALL
            .into_iter()
            .map(|unit| UnitDto {
                code: unit.code(),
                label: unit.label(),
                dimension: match unit.dimension() {
                    mmp_core::domain::Dimension::Mass => "mass",
                    mmp_core::domain::Dimension::Volume => "volume",
                    mmp_core::domain::Dimension::Count => "count",
                },
                convertible: unit.base_factor().is_some(),
            })
            .collect()
    }
}
