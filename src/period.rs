//! La `Period` : règle qui décide quand une `Bar` se ferme (fiches `AGG-P0`, `AGG-P1`).

use crate::canonical::{Granularity, Trade, Ts};

/// Résultat de l'examen d'un trade par une `Period`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boundary {
    /// Le trade appartient à la barre courante.
    Continue,
    /// Le trade ouvre une nouvelle barre `[start, end)` : fermer la courante d'abord.
    CloseAndOpen { start: Ts, end: Ts },
}

/// Contrat commun des règles de période (fiche `AGG-P0`).
pub trait Period {
    /// Examine un trade et indique s'il faut fermer/ouvrir une barre.
    fn on_trade(&mut self, t: &Trade) -> Boundary;
    /// Granularité minimale requise (fiche `CAN-7`). Les périodes sur le tape se
    /// contentent de `L1`.
    fn min_granularity(&self) -> Granularity {
        Granularity::L1
    }
    /// Libellé pour la sortie (fiche `EXT-1`).
    fn label(&self) -> String;
}

/// Barres temporelles de durée fixe (fiche `AGG-P1`).
///
/// Fenêtres `[base + k·interval, base + (k+1)·interval)` où `base` = ts du **premier**
/// trade vu. Déterministe : ne dépend que des `ts`.
pub struct TimePeriod {
    interval_ns: i64,
    base: Option<Ts>,
    current_end: Ts,
}

impl TimePeriod {
    /// Crée une période de `interval_ns` nanosecondes (doit être > 0).
    pub fn new(interval_ns: i64) -> Self {
        assert!(interval_ns > 0, "interval_ns doit être > 0");
        TimePeriod {
            interval_ns,
            base: None,
            current_end: 0,
        }
    }

    /// Borne haute de la fenêtre contenant `ts`, relative à `base`.
    fn window_end_for(&self, base: Ts, ts: Ts) -> Ts {
        let k = (ts - base).div_euclid(self.interval_ns);
        base + (k + 1) * self.interval_ns
    }
}

impl Period for TimePeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        match self.base {
            None => {
                // Premier trade : ancre la grille et ouvre la première fenêtre.
                let base = t.ts;
                self.base = Some(base);
                let end = self.window_end_for(base, t.ts);
                self.current_end = end;
                Boundary::CloseAndOpen {
                    start: end - self.interval_ns,
                    end,
                }
            }
            Some(base) => {
                if t.ts < self.current_end {
                    Boundary::Continue
                } else {
                    let end = self.window_end_for(base, t.ts);
                    let start = end - self.interval_ns;
                    self.current_end = end;
                    Boundary::CloseAndOpen { start, end }
                }
            }
        }
    }

    fn label(&self) -> String {
        format!("time:{}ns", self.interval_ns)
    }
}
