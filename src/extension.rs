//! Point d'extension réactif (pilier P5, fiche `EXT-1`).
//!
//! T0 : le minimum — un `Subscriber` notifié à chaque barre fermée (`on_bar_close`).
//! Les variantes push/pull complètes (channels, `Stream`, `on_bar_update`) arrivent en T1.

use crate::bar::Bar;

/// Reçoit les barres fermées. L'implémenteur calcule ce qu'il veut : la crate **expose**,
/// elle n'interprète pas.
pub trait Subscriber {
    /// Appelé quand une barre se ferme, avec le libellé de la période qui l'a produite.
    fn on_bar_close(&mut self, period: &str, bar: &Bar);
}
