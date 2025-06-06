//! Utility functions and helpers for configuration

use serde::{Deserialize, Deserializer, Serializer};
use std::time::Duration;

/// Serde helper module for Duration serialization as seconds
pub mod serde_duration {
    use super::*;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(seconds))
    }
}

/// Serde helper module for optional Duration serialization
pub mod serde_duration_option {
    use super::*;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => serializer.serialize_some(&d.as_secs()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds: Option<u64> = Option::deserialize(deserializer)?;
        Ok(seconds.map(Duration::from_secs))
    }
}

/// Default functions for serde
pub fn default_true() -> bool {
    true
}

pub fn default_false() -> bool {
    false
}
