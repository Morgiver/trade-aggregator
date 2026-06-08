//! Point d'extension réactif (pilier P5, fiches `EXT-*`).
//!
//! La crate **expose** ; elle n'interprète pas. T1 lot C : notification intra-barre
//! (`on_bar_update`), adaptateurs **closure** et **channel** (push), et consommation
//! **pull** via le `Receiver` du channel (qui est un itérateur).

use std::sync::mpsc::Sender;

use crate::bar::Bar;
use crate::passive::OrderBook;

/// Reçoit les barres. L'implémenteur calcule ce qu'il veut.
pub trait Subscriber {
    /// Appelé quand une barre se ferme (fiche `EXT-1`).
    fn on_bar_close(&mut self, period: &str, bar: &Bar);

    /// Variante de `on_bar_close` recevant aussi le **carnet** échantillonné au `ts` de
    /// clôture de la barre (fiche `EXT-7`, issue #18) — `None` si le côté passif est
    /// inactif. Permet d'aligner les scalaires passifs (top-of-book, depth) sur la barre
    /// agressive plutôt que sur une fenêtre passive indépendante.
    ///
    /// Par défaut : délègue à `on_bar_close` (rétro-compatible — un abonné qui n'a besoin
    /// que de la barre n'implémente que `on_bar_close`). On expose le carnet **brut** ;
    /// microprice/imbalance restent au consommateur (non-goal).
    fn on_bar_close_with_book(&mut self, period: &str, bar: &Bar, _book: Option<&OrderBook>) {
        self.on_bar_close(period, bar);
    }

    /// Appelé à chaque trade intégré dans la barre **en formation** (fiches `EXT-2`/`AGG-B3`).
    ///
    /// Par défaut : rien (rétro-compatible — un abonné T0 n'a pas à l'implémenter).
    /// Note : la barre en formation porte son `OHLCV` courant ; son `OrderFlow` n'est
    /// renseigné qu'à la clôture (le snapshot par trade serait coûteux).
    fn on_bar_update(&mut self, _period: &str, _bar: &Bar) {}
}

/// Adaptateur **closure** (fiche `EXT-1`) : `|period, bar| { … }` sur fermeture.
pub struct FnSubscriber<F>(pub F);

impl<F: FnMut(&str, &Bar)> Subscriber for FnSubscriber<F> {
    fn on_bar_close(&mut self, period: &str, bar: &Bar) {
        (self.0)(period, bar);
    }
}

/// Adaptateur **channel** (fiche `EXT-4`) : pousse les barres fermées dans un
/// `std::sync::mpsc::Sender`. Le `Receiver` correspondant fournit la consommation **pull**
/// (fiche `EXT-5`) — c'est un itérateur (`recv` / `try_iter` / `into_iter`).
pub struct ChannelSink {
    tx: Sender<(String, Bar)>,
}

impl ChannelSink {
    pub fn new(tx: Sender<(String, Bar)>) -> Self {
        ChannelSink { tx }
    }
}

impl Subscriber for ChannelSink {
    fn on_bar_close(&mut self, period: &str, bar: &Bar) {
        // Si le récepteur est lâché, on ignore (le producteur ne doit pas paniquer).
        let _ = self.tx.send((period.to_string(), bar.clone()));
    }
}
