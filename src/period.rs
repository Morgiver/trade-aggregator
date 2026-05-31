//! La `Period` : règle qui décide quand une `Bar` se ferme (fiches `AGG-P0`, `AGG-P1`).

use crate::canonical::{Granularity, Trade, Ts};

/// Résultat de l'examen d'un trade par une `Period`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boundary {
    /// Le trade appartient à la barre courante.
    Continue,
    /// Le trade ouvre une nouvelle barre `[start, end)` : fermer la courante d'abord.
    /// `partial` = la nouvelle barre démarre incomplète (entrée en cours de fenêtre,
    /// fiche `AGG-B5`).
    CloseAndOpen { start: Ts, end: Ts, partial: bool },
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
                    partial: false,
                }
            }
            Some(base) => {
                if t.ts < self.current_end {
                    Boundary::Continue
                } else {
                    let end = self.window_end_for(base, t.ts);
                    let start = end - self.interval_ns;
                    self.current_end = end;
                    Boundary::CloseAndOpen {
                        start,
                        end,
                        partial: false,
                    }
                }
            }
        }
    }

    fn label(&self) -> String {
        format!("time:{}ns", self.interval_ns)
    }
}

/// Barres temporelles **alignées sur l'horloge** : fenêtres
/// `[k·interval, (k+1)·interval)` (multiples de l'epoch), fiche `AGG-P2`.
///
/// La **première** barre est marquée *partielle* (`AGG-B5`) si le flux démarre en cours
/// de fenêtre (premier trade non aligné sur une borne).
pub struct AlignedTimePeriod {
    interval_ns: i64,
    current_end: Option<Ts>,
}

impl AlignedTimePeriod {
    pub fn new(interval_ns: i64) -> Self {
        assert!(interval_ns > 0, "interval_ns doit être > 0");
        AlignedTimePeriod {
            interval_ns,
            current_end: None,
        }
    }

    fn window(&self, ts: Ts) -> (Ts, Ts) {
        let start = ts.div_euclid(self.interval_ns) * self.interval_ns;
        (start, start + self.interval_ns)
    }
}

impl Period for AlignedTimePeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        let (start, end) = self.window(t.ts);
        match self.current_end {
            None => {
                self.current_end = Some(end);
                // Partielle si le flux n'a pas démarré pile sur la borne de fenêtre.
                Boundary::CloseAndOpen {
                    start,
                    end,
                    partial: t.ts != start,
                }
            }
            Some(ce) if t.ts < ce => Boundary::Continue,
            Some(_) => {
                self.current_end = Some(end);
                Boundary::CloseAndOpen {
                    start,
                    end,
                    partial: false,
                }
            }
        }
    }

    fn label(&self) -> String {
        format!("aligned-time:{}ns", self.interval_ns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::AggressorSide;

    fn tr(ts: Ts) -> Trade {
        Trade {
            ts,
            price: 100,
            size: 1,
            aggressor: AggressorSide::Buy,
            instrument_id: 1,
        }
    }

    // AGG-P2 + AGG-B5 : fenêtres alignées sur l'horloge, première barre partielle.
    #[test]
    fn aligned_time_period_windows_and_partial_first_bar() {
        let mut p = AlignedTimePeriod::new(100);
        // Premier trade à ts=150 → fenêtre [100,200), barre partielle (démarrage en cours).
        assert_eq!(
            p.on_trade(&tr(150)),
            Boundary::CloseAndOpen {
                start: 100,
                end: 200,
                partial: true
            }
        );
        // ts=180 reste dans la fenêtre.
        assert_eq!(p.on_trade(&tr(180)), Boundary::Continue);
        // ts=230 → nouvelle fenêtre [200,300), complète.
        assert_eq!(
            p.on_trade(&tr(230)),
            Boundary::CloseAndOpen {
                start: 200,
                end: 300,
                partial: false
            }
        );
    }

    #[test]
    fn aligned_first_bar_on_boundary_is_not_partial() {
        let mut p = AlignedTimePeriod::new(100);
        assert_eq!(
            p.on_trade(&tr(200)),
            Boundary::CloseAndOpen {
                start: 200,
                end: 300,
                partial: false
            }
        );
    }
}
