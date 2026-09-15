macro_rules! str_enum {
    ($name:ident, $unknown:ident, $label:literal) => {
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.code())
            }
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $unknown(pub String);

        impl std::fmt::Display for $unknown {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "`{}` is not a known {}", self.0, $label)
            }
        }

        impl std::error::Error for $unknown {}

        impl std::str::FromStr for $name {
            type Err = $unknown;

            fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
                Self::ALL
                    .into_iter()
                    .find(|candidate| candidate.code() == value)
                    .ok_or_else(|| $unknown(value.to_owned()))
            }
        }
    };
}

pub(crate) use str_enum;
