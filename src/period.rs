//! La `Period` : règle qui décide quand une `Bar` se ferme (fiches `AGG-P0`, `AGG-P1`).

use crate::canonical::{Granularity, Qty, Trade, Ts};

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

/// Barres par **nombre de trades** (fiche `AGG-P4`) : `n` trades par barre.
pub struct TickPeriod {
    n: u64,
    count: u64,
}

impl TickPeriod {
    pub fn new(n: u64) -> Self {
        assert!(n > 0, "n doit être > 0");
        TickPeriod { n, count: 0 }
    }
}

impl Period for TickPeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        if self.count == 0 || self.count >= self.n {
            self.count = 1;
            Boundary::CloseAndOpen {
                start: t.ts,
                end: t.ts,
                partial: false,
            }
        } else {
            self.count += 1;
            Boundary::Continue
        }
    }

    fn label(&self) -> String {
        format!("tick:{}", self.n)
    }
}

/// Barres par **volume échangé** (fiche `AGG-P5`) : ferme dès que le volume cumulé
/// atteint `threshold` (la barre inclut le trade qui franchit le seuil).
pub struct VolumePeriod {
    threshold: Qty,
    acc: Qty,
    open: bool,
}

impl VolumePeriod {
    pub fn new(threshold: Qty) -> Self {
        assert!(threshold > 0, "threshold doit être > 0");
        VolumePeriod {
            threshold,
            acc: 0,
            open: false,
        }
    }
}

impl Period for VolumePeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        if !self.open || self.acc >= self.threshold {
            self.open = true;
            self.acc = t.size;
            Boundary::CloseAndOpen {
                start: t.ts,
                end: t.ts,
                partial: false,
            }
        } else {
            self.acc += t.size;
            Boundary::Continue
        }
    }

    fn label(&self) -> String {
        format!("volume:{}", self.threshold)
    }
}

/// Barres par **valeur échangée / notional** (fiche `AGG-P6`) : `Σ price·size`.
pub struct DollarPeriod {
    threshold: i128,
    acc: i128,
    open: bool,
}

impl DollarPeriod {
    pub fn new(threshold: i128) -> Self {
        assert!(threshold > 0, "threshold doit être > 0");
        DollarPeriod {
            threshold,
            acc: 0,
            open: false,
        }
    }

    fn notional(t: &Trade) -> i128 {
        (t.price as i128) * (t.size as i128)
    }
}

impl Period for DollarPeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        if !self.open || self.acc >= self.threshold {
            self.open = true;
            self.acc = Self::notional(t);
            Boundary::CloseAndOpen {
                start: t.ts,
                end: t.ts,
                partial: false,
            }
        } else {
            self.acc += Self::notional(t);
            Boundary::Continue
        }
    }

    fn label(&self) -> String {
        format!("dollar:{}", self.threshold)
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

    fn trv(ts: Ts, price: i64, size: u64) -> Trade {
        Trade {
            ts,
            price,
            size,
            aggressor: AggressorSide::Buy,
            instrument_id: 1,
        }
    }

    fn closes(b: Boundary) -> bool {
        matches!(b, Boundary::CloseAndOpen { .. })
    }

    // AGG-P4 : barres de n trades.
    #[test]
    fn tick_period_n_trades_per_bar() {
        let mut p = TickPeriod::new(3);
        assert!(closes(p.on_trade(&trv(0, 100, 1)))); // ouvre barre 1
        assert!(!closes(p.on_trade(&trv(1, 100, 1)))); // 2e
        assert!(!closes(p.on_trade(&trv(2, 100, 1)))); // 3e (barre pleine)
        assert!(closes(p.on_trade(&trv(3, 100, 1)))); // ferme barre 1, ouvre barre 2
    }

    // AGG-P5 : barres de volume (inclut le trade qui franchit le seuil).
    #[test]
    fn volume_period_closes_after_threshold() {
        let mut p = VolumePeriod::new(10);
        assert!(closes(p.on_trade(&trv(0, 100, 4)))); // acc 4
        assert!(!closes(p.on_trade(&trv(1, 100, 4)))); // acc 8
        assert!(!closes(p.on_trade(&trv(2, 100, 5)))); // acc 13 ≥ 10 (inclus)
        assert!(closes(p.on_trade(&trv(3, 100, 1)))); // trade suivant → ferme/ouvre
    }

    // AGG-P6 : barres de notional (Σ price·size).
    #[test]
    fn dollar_period_closes_after_notional() {
        let mut p = DollarPeriod::new(1_000);
        assert!(closes(p.on_trade(&trv(0, 100, 4)))); // 400
        assert!(!closes(p.on_trade(&trv(1, 100, 5)))); // 900
        assert!(!closes(p.on_trade(&trv(2, 100, 2)))); // 1100 ≥ 1000
        assert!(closes(p.on_trade(&trv(3, 100, 1)))); // suivant → ferme/ouvre
    }
}
