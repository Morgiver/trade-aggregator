//! Erreurs typées (fiche `TR-6` : fail-fast à la construction, pas de panic).

use crate::canonical::Granularity;

/// Erreur de configuration, levée **à la construction** d'un `SymbolAggregator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    /// Une agrégation exige une granularité supérieure à celle déclarée (fiche `SYM-8`).
    IncompatibleGranularity {
        required: Granularity,
        declared: Granularity,
    },
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConfigError::IncompatibleGranularity { required, declared } => write!(
                f,
                "agrégation incompatible : requiert {required:?}, granularité déclarée {declared:?}"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}
