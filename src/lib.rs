//! # trade-aggregator
//!
//! Agrégation de données de marché brutes (tape + book) en order flow agressif et
//! profils de liquidité passifs — temps réel et replay, **déterministe** (event-time).
//! La crate **agrège et expose** ; elle **n'interprète pas** (pilier P2).
//!
//! Voir `docs/` pour la vision, le domaine, l'architecture et la roadmap.
//!
//! Cette version couvre la tranche **T0 — walking skeleton** :
//! `trades → SymbolAggregator → barres temporelles → on_bar_close`.

pub mod aggregator;
pub mod bar;
pub mod canonical;
pub mod error;
pub mod extension;
pub mod period;

/// Mapping DataBento (DBN → modèle canonique), isolé derrière la feature `databento`
/// (fiches `CAN-8`/`CAN-11`/`CAN-12`).
#[cfg(feature = "databento")]
pub mod databento;

pub use aggregator::{Builder, SymbolAggregator};
pub use bar::{Bar, Ohlcv};
pub use canonical::{AggressorSide, Granularity, Instrument, MarketEvent, Px, Qty, Trade, Ts};
pub use error::ConfigError;
pub use extension::Subscriber;
pub use period::{Boundary, Period, TimePeriod};
