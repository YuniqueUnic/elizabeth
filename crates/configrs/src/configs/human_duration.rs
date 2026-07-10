use std::{fmt, str::FromStr, time::Duration};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{Error as _, Unexpected, Visitor},
};

/// A configuration duration that accepts humantime strings or integer seconds.
///
/// Human-readable serializers always emit the humantime string form so saved
/// configuration remains easy to review and edit.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HumanDuration(Duration);

impl HumanDuration {
    pub const fn as_secs(&self) -> u64 {
        self.0.as_secs()
    }

    pub const fn into_inner(self) -> Duration {
        self.0
    }
}

impl From<Duration> for HumanDuration {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl From<HumanDuration> for Duration {
    fn from(value: HumanDuration) -> Self {
        value.into_inner()
    }
}

impl FromStr for HumanDuration {
    type Err = humantime::DurationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        humantime::parse_duration(value).map(Self)
    }
}

impl Serialize for HumanDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&humantime::format_duration(self.0).to_string())
        } else {
            serializer.serialize_u64(self.as_secs())
        }
    }
}

impl<'de> Deserialize<'de> for HumanDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct HumanDurationVisitor;

        impl Visitor<'_> for HumanDurationVisitor {
            type Value = HumanDuration;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a humantime duration string or non-negative integer seconds")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Duration::from_secs(value).into())
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let seconds = u64::try_from(value).map_err(|_| {
                    E::invalid_value(Unexpected::Signed(value), &"non-negative integer seconds")
                })?;
                Ok(Duration::from_secs(seconds).into())
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(E::custom)
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_any(HumanDurationVisitor)
        } else {
            deserializer.deserialize_u64(HumanDurationVisitor)
        }
    }
}
