//! La `Period` : règle qui décide quand une `Bar` se ferme (fiches `AGG-P0`, `AGG-P1`).

use crate::canonical::{AggressorSide, Granularity, Px, Qty, Trade, Ts};

/// Signe agressif d'un trade : `+1` Buy, `−1` Sell, `0` inconnu.
fn signed(t: &Trade) -> i64 {
    match t.aggressor {
        AggressorSide::Buy => 1,
        AggressorSide::Sell => -1,
        AggressorSide::Unknown => 0,
    }
}

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

/// Barres de **range de prix** (fiche `AGG-P7`) : l'amplitude `high − low` d'une barre
/// reste ≤ `range`. Le trade qui ferait dépasser le range ouvre une nouvelle barre.
pub struct RangePeriod {
    range: i64,
    lo: Px,
    hi: Px,
    open: bool,
}

impl RangePeriod {
    pub fn new(range: i64) -> Self {
        assert!(range > 0, "range doit être > 0");
        RangePeriod {
            range,
            lo: 0,
            hi: 0,
            open: false,
        }
    }
}

impl Period for RangePeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        if !self.open {
            self.open = true;
            self.lo = t.price;
            self.hi = t.price;
            return Boundary::CloseAndOpen {
                start: t.ts,
                end: t.ts,
                partial: false,
            };
        }
        let new_hi = self.hi.max(t.price);
        let new_lo = self.lo.min(t.price);
        if new_hi - new_lo > self.range {
            // Le trade dépasse le range → il ouvre une nouvelle barre.
            self.lo = t.price;
            self.hi = t.price;
            Boundary::CloseAndOpen {
                start: t.ts,
                end: t.ts,
                partial: false,
            }
        } else {
            self.hi = new_hi;
            self.lo = new_lo;
            Boundary::Continue
        }
    }

    fn label(&self) -> String {
        format!("range:{}", self.range)
    }
}

/// Barres **Renko** simplifiées (fiche `AGG-P8`) : nouvelle brique quand le prix s'écarte
/// d'au moins `brick` du prix d'ouverture de la barre courante.
///
/// *Note* : version simplifiée (référence = prix d'ouverture de la barre), à affiner
/// ultérieurement (grille de briques, multi-briques par saut) si le besoin émerge.
pub struct RenkoPeriod {
    brick: i64,
    reference: Px,
    open: bool,
}

impl RenkoPeriod {
    pub fn new(brick: i64) -> Self {
        assert!(brick > 0, "brick doit être > 0");
        RenkoPeriod {
            brick,
            reference: 0,
            open: false,
        }
    }
}

impl Period for RenkoPeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        if !self.open || (t.price - self.reference).abs() >= self.brick {
            self.open = true;
            self.reference = t.price;
            Boundary::CloseAndOpen {
                start: t.ts,
                end: t.ts,
                partial: false,
            }
        } else {
            Boundary::Continue
        }
    }

    fn label(&self) -> String {
        format!("renko:{}", self.brick)
    }
}

/// Barres **Renko sur grille de briques** (issue #21, raffinement de `RenkoPeriod`).
///
/// La référence est **alignée sur une grille** de niveaux multiples de `brick` (et non sur
/// le prix d'ouverture arbitraire de la barre) → bornes de briques **déterministes**,
/// indépendantes de l'endroit où le flux démarre. Une barre se ferme quand le prix s'écarte
/// d'au moins `brick` de la référence ; la nouvelle référence est **re-snappée sur la
/// grille**, ce qui gère proprement les **sauts multi-briques** (un trade franchissant
/// plusieurs briques d'un coup atterrit sur le bon niveau de grille).
///
/// **Borne d'excursion explicite** : au sein d'une barre, `|price − reference| < brick`,
/// donc l'amplitude des prix est bornée par `excursion_bound() = 2·brick − 1` (corps +
/// mèche max) → **largeur de footprint fixe garantie** (cf. `Footprint::window`).
pub struct RenkoBrickPeriod {
    brick: i64,
    reference: Px,
    open: bool,
}

impl RenkoBrickPeriod {
    pub fn new(brick: i64) -> Self {
        assert!(brick > 0, "brick doit être > 0");
        RenkoBrickPeriod {
            brick,
            reference: 0,
            open: false,
        }
    }

    /// Niveau de grille (multiple de `brick`) au plus près sous `price`.
    fn snap(&self, price: Px) -> Px {
        price.div_euclid(self.brick) * self.brick
    }

    /// Référence de grille courante (multiple de `brick`).
    pub fn reference(&self) -> Px {
        self.reference
    }

    /// Borne d'excursion des prix d'une barre : `2·brick − 1` (corps + mèche max).
    pub fn excursion_bound(&self) -> i64 {
        2 * self.brick - 1
    }
}

impl Period for RenkoBrickPeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        if !self.open || (t.price - self.reference).abs() >= self.brick {
            self.open = true;
            // Re-snappe sur la grille → sauts multi-briques gérés proprement.
            self.reference = self.snap(t.price);
            Boundary::CloseAndOpen {
                start: t.ts,
                end: t.ts,
                partial: false,
            }
        } else {
            Boundary::Continue
        }
    }

    fn label(&self) -> String {
        format!("renko-grid:{}", self.brick)
    }
}

/// Borne « decide-before-add » pour les périodes à seuil signé : ferme si la barre
/// courante a déjà atteint le seuil, sinon continue.
macro_rules! threshold_boundary {
    ($self:ident, $t:ident, $acc:expr, $reset:expr, $reached:expr) => {{
        if !$self.open || $reached {
            $self.open = true;
            $acc = $reset;
            Boundary::CloseAndOpen {
                start: $t.ts,
                end: $t.ts,
                partial: false,
            }
        } else {
            $acc += $reset;
            Boundary::Continue
        }
    }};
}

/// Barres **tick imbalance** (fiche `AGG-P10`) : ferme quand `|Σ signe| ≥ seuil`.
pub struct TickImbalancePeriod {
    threshold: i64,
    acc: i64,
    open: bool,
}
impl TickImbalancePeriod {
    pub fn new(threshold: i64) -> Self {
        assert!(threshold > 0);
        TickImbalancePeriod {
            threshold,
            acc: 0,
            open: false,
        }
    }
}
impl Period for TickImbalancePeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        let reached = self.acc.abs() >= self.threshold;
        threshold_boundary!(self, t, self.acc, signed(t), reached)
    }
    fn label(&self) -> String {
        format!("tick-imbalance:{}", self.threshold)
    }
}

/// Barres **volume imbalance** (fiche `AGG-P11`) : `Σ signe·size`.
pub struct VolumeImbalancePeriod {
    threshold: i64,
    acc: i64,
    open: bool,
}
impl VolumeImbalancePeriod {
    pub fn new(threshold: i64) -> Self {
        assert!(threshold > 0);
        VolumeImbalancePeriod {
            threshold,
            acc: 0,
            open: false,
        }
    }
}
impl Period for VolumeImbalancePeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        let reached = self.acc.abs() >= self.threshold;
        threshold_boundary!(self, t, self.acc, signed(t) * t.size as i64, reached)
    }
    fn label(&self) -> String {
        format!("volume-imbalance:{}", self.threshold)
    }
}

/// Barres **dollar imbalance** (fiche `AGG-P12`) : `Σ signe·price·size` (notional).
pub struct DollarImbalancePeriod {
    threshold: i128,
    acc: i128,
    open: bool,
}
impl DollarImbalancePeriod {
    pub fn new(threshold: i128) -> Self {
        assert!(threshold > 0);
        DollarImbalancePeriod {
            threshold,
            acc: 0,
            open: false,
        }
    }
}
impl Period for DollarImbalancePeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        let reached = self.acc.abs() >= self.threshold;
        let contrib = signed(t) as i128 * t.price as i128 * t.size as i128;
        threshold_boundary!(self, t, self.acc, contrib, reached)
    }
    fn label(&self) -> String {
        format!("dollar-imbalance:{}", self.threshold)
    }
}

/// Barres **run** (fiche `AGG-P13`, simplifiées) : ferme quand une série directionnelle
/// consécutive (même côté) atteint `threshold` trades.
pub struct RunPeriod {
    threshold: u64,
    run: u64,
    dir: i64,
    open: bool,
}
impl RunPeriod {
    pub fn new(threshold: u64) -> Self {
        assert!(threshold > 0);
        RunPeriod {
            threshold,
            run: 0,
            dir: 0,
            open: false,
        }
    }
    fn start(&mut self, s: i64) {
        self.dir = s;
        self.run = if s != 0 { 1 } else { 0 };
    }
}
impl Period for RunPeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        let s = signed(t);
        if !self.open || self.run >= self.threshold {
            self.open = true;
            self.start(s);
            Boundary::CloseAndOpen {
                start: t.ts,
                end: t.ts,
                partial: false,
            }
        } else {
            if s != 0 && s == self.dir {
                self.run += 1;
            } else {
                self.start(s);
            }
            Boundary::Continue
        }
    }
    fn label(&self) -> String {
        format!("run:{}", self.threshold)
    }
}

/// Barres **Point & Figure** (fiche `AGG-P9`, simplifiées) : chaque barre = une colonne
/// directionnelle (X montante / O descendante). La barre se ferme au **renversement** du
/// prix d'au moins `reversal × box_size` contre la tendance de la colonne.
///
/// *Note* : version simplifiée (pas de grille de box stricte ni de filtrage à la box) —
/// raffinement ultérieur si besoin.
pub struct PointFigurePeriod {
    box_size: i64,
    reversal: i64,
    dir: i64,
    extreme: Px,
    open: bool,
}
impl PointFigurePeriod {
    pub fn new(box_size: i64, reversal: i64) -> Self {
        assert!(box_size > 0 && reversal > 0);
        PointFigurePeriod {
            box_size,
            reversal,
            dir: 0,
            extreme: 0,
            open: false,
        }
    }
    fn open_at(&mut self, t: &Trade, dir: i64) -> Boundary {
        self.open = true;
        self.dir = dir;
        self.extreme = t.price;
        Boundary::CloseAndOpen {
            start: t.ts,
            end: t.ts,
            partial: false,
        }
    }
}
impl Period for PointFigurePeriod {
    fn on_trade(&mut self, t: &Trade) -> Boundary {
        if !self.open {
            return self.open_at(t, 0);
        }
        let rev = self.reversal * self.box_size;
        match self.dir {
            // Direction non encore établie : on l'établit au 1er mouvement ≥ box.
            0 => {
                if (t.price - self.extreme).abs() >= self.box_size {
                    self.dir = (t.price - self.extreme).signum();
                    self.extreme = t.price;
                }
                Boundary::Continue
            }
            d if d > 0 => {
                if t.price > self.extreme {
                    self.extreme = t.price;
                    Boundary::Continue
                } else if self.extreme - t.price >= rev {
                    self.open_at(t, -1) // renversement → nouvelle colonne (O)
                } else {
                    Boundary::Continue
                }
            }
            _ => {
                if t.price < self.extreme {
                    self.extreme = t.price;
                    Boundary::Continue
                } else if t.price - self.extreme >= rev {
                    self.open_at(t, 1) // renversement → nouvelle colonne (X)
                } else {
                    Boundary::Continue
                }
            }
        }
    }
    fn label(&self) -> String {
        format!("pnf:{}:{}", self.box_size, self.reversal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // AGG-P7 : barres de range (amplitude ≤ range).
    #[test]
    fn range_period_closes_when_span_exceeds_range() {
        let mut p = RangePeriod::new(5);
        assert!(closes(p.on_trade(&trv(0, 100, 1)))); // ouvre, lo=hi=100
        assert!(!closes(p.on_trade(&trv(1, 103, 1)))); // span 3 ≤ 5
        assert!(!closes(p.on_trade(&trv(2, 105, 1)))); // span 5 ≤ 5
        assert!(closes(p.on_trade(&trv(3, 106, 1)))); // span 6 > 5 → nouvelle barre
    }

    // AGG-P8 : barres Renko (écart ≥ brick depuis l'ouverture).
    #[test]
    fn renko_period_closes_on_brick_move() {
        let mut p = RenkoPeriod::new(10);
        assert!(closes(p.on_trade(&trv(0, 100, 1)))); // ref=100
        assert!(!closes(p.on_trade(&trv(1, 105, 1)))); // |+5| < 10
        assert!(closes(p.on_trade(&trv(2, 110, 1)))); // |+10| ≥ 10 → nouvelle brique
        assert!(closes(p.on_trade(&trv(3, 100, 1)))); // |−10| ≥ 10 → nouvelle brique
    }

    // UC-T5-11 : Renko grille — référence alignée + sauts multi-briques + borne d'excursion.
    #[test]
    fn renko_brick_grid_aligned_and_multibrick() {
        let mut p = RenkoBrickPeriod::new(10);
        // Ouverture : prix 103 → référence snappée sur la grille = 100.
        assert!(closes(p.on_trade(&trv(0, 103, 1))));
        assert_eq!(p.reference(), 100);
        assert!(!closes(p.on_trade(&trv(1, 108, 1)))); // |8| < 10 → continue
        assert!(!closes(p.on_trade(&trv(2, 95, 1)))); // |−5| < 10 → continue
        // Saut multi-briques : 134 → ferme, référence re-snappée sur 130.
        assert!(closes(p.on_trade(&trv(3, 134, 1))));
        assert_eq!(p.reference(), 130);
        // Borne d'excursion explicite = 2·brick − 1.
        assert_eq!(p.excursion_bound(), 19);
        assert_eq!(p.label(), "renko-grid:10");
    }

    fn tside(price: i64, size: u64, side: AggressorSide) -> Trade {
        Trade {
            ts: 0,
            price,
            size,
            aggressor: side,
            instrument_id: 1,
        }
    }

    // AGG-P10 : tick imbalance (Σ signe).
    #[test]
    fn tick_imbalance_closes_on_threshold() {
        use AggressorSide::{Buy, Sell};
        let mut p = TickImbalancePeriod::new(2);
        assert!(closes(p.on_trade(&tside(100, 1, Buy)))); // acc +1, ouvre
        assert!(!closes(p.on_trade(&tside(100, 1, Buy)))); // acc +2 (≥2)
        assert!(closes(p.on_trade(&tside(100, 1, Sell)))); // seuil atteint → ferme/ouvre
    }

    // AGG-P11 : volume imbalance (Σ signe·size).
    #[test]
    fn volume_imbalance_closes_on_threshold() {
        use AggressorSide::{Buy, Sell};
        let mut p = VolumeImbalancePeriod::new(10);
        assert!(closes(p.on_trade(&tside(100, 4, Sell)))); // acc -4, ouvre
        assert!(!closes(p.on_trade(&tside(100, 7, Sell)))); // acc -11 (|.|≥10)
        assert!(closes(p.on_trade(&tside(100, 1, Buy)))); // ferme/ouvre
    }

    // AGG-P12 : dollar imbalance (Σ signe·price·size).
    #[test]
    fn dollar_imbalance_closes_on_threshold() {
        use AggressorSide::Buy;
        let mut p = DollarImbalancePeriod::new(1_000);
        assert!(closes(p.on_trade(&tside(100, 4, Buy)))); // +400, ouvre
        assert!(!closes(p.on_trade(&tside(100, 8, Buy)))); // +1200 (≥1000)
        assert!(closes(p.on_trade(&tside(100, 1, Buy)))); // ferme/ouvre
    }

    // AGG-P13 : run bars (série directionnelle).
    #[test]
    fn run_period_closes_on_run_length() {
        use AggressorSide::{Buy, Sell};
        let mut p = RunPeriod::new(3);
        assert!(closes(p.on_trade(&tside(100, 1, Buy)))); // run 1, ouvre
        assert!(!closes(p.on_trade(&tside(100, 1, Buy)))); // run 2
        assert!(!closes(p.on_trade(&tside(100, 1, Buy)))); // run 3 (atteint)
        assert!(closes(p.on_trade(&tside(100, 1, Sell)))); // ferme/ouvre
    }

    // AGG-P9 : Point & Figure (renversement de colonne).
    #[test]
    fn point_figure_reverses_column() {
        let mut p = PointFigurePeriod::new(10, 3); // box 10, reversal 3 → seuil 30
        assert!(closes(p.on_trade(&trv(0, 100, 1)))); // ouvre colonne
        assert!(!closes(p.on_trade(&trv(1, 115, 1)))); // établit dir = +1
        assert!(!closes(p.on_trade(&trv(2, 130, 1)))); // prolonge la colonne X
        assert!(closes(p.on_trade(&trv(3, 95, 1)))); // 130-95=35 ≥ 30 → renversement
    }
}
