//! Lentilles **order flow** attachées à une `Bar` (nœud `aggressor/orderflow`).
//!
//! Lot A de la tranche T1. Chaque lentille est un **accumulateur composable**
//! (`BarComponent`), câblé dans `SymbolAggregator` via `LensKind` (composabilité) et
//! exposé à la clôture dans `OrderFlow` attaché à la `Bar`.

use std::collections::BTreeMap;

use crate::canonical::{AggressorSide, Px, Qty, Trade};

/// Contrat commun d'une lentille order flow (fiche `OF-0`).
pub trait BarComponent {
    /// Intègre un trade (hot path).
    fn on_trade(&mut self, t: &Trade);
    /// Finalise à la fermeture de la barre (par défaut : rien).
    fn on_close(&mut self) {}
}

/// **Footprint** (fiche `FP-1`/`FP-2`) : volume par `(prix, côté)` dans la barre.
///
/// Un trade de côté `Unknown` n'est attribué ni au Bid ni à l'Ask (sa quantité n'entre
/// pas dans les cellules) — il reste comptabilisé ailleurs (OHLCV, profil de volume).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Footprint {
    /// `prix → (volume acheteur agressif, volume vendeur agressif)`.
    cells: BTreeMap<Px, (Qty, Qty)>,
}

impl Footprint {
    pub fn new() -> Self {
        Self::default()
    }
    /// Cellule `(buy, sell)` à un prix.
    pub fn cell(&self, price: Px) -> (Qty, Qty) {
        self.cells.get(&price).copied().unwrap_or((0, 0))
    }
    /// Itère les cellules par prix croissant.
    pub fn iter(&self) -> impl Iterator<Item = (Px, (Qty, Qty))> + '_ {
        self.cells.iter().map(|(&p, &c)| (p, c))
    }
}

impl BarComponent for Footprint {
    fn on_trade(&mut self, t: &Trade) {
        let entry = self.cells.entry(t.price).or_insert((0, 0));
        match t.aggressor {
            AggressorSide::Buy => entry.0 += t.size,
            AggressorSide::Sell => entry.1 += t.size,
            AggressorSide::Unknown => {}
        }
    }
}

/// **Profil de volume** + `POC` + `Value Area` (fiches `VP-1`/`VP-2`/`VP-3`).
#[derive(Debug, Clone, PartialEq)]
pub struct VolumeProfile {
    by_price: BTreeMap<Px, Qty>,
    value_area_pct: f64,
}

impl VolumeProfile {
    /// Profil avec un seuil de value area (ex. `0.70`).
    pub fn new(value_area_pct: f64) -> Self {
        assert!(
            value_area_pct > 0.0 && value_area_pct <= 1.0,
            "value_area_pct ∈ ]0,1]"
        );
        VolumeProfile {
            by_price: BTreeMap::new(),
            value_area_pct,
        }
    }

    /// Volume total agrégé.
    pub fn total_volume(&self) -> Qty {
        self.by_price.values().copied().sum()
    }

    /// `POC` = niveau de prix de volume maximal (le plus bas en cas d'égalité).
    pub fn poc(&self) -> Option<Px> {
        self.by_price
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            .map(|(&p, _)| p)
    }

    /// `Value Area` = plage `(bas, haut)` autour du POC concentrant ≥ `value_area_pct`
    /// du volume. Étend itérativement vers le voisin de plus gros volume.
    pub fn value_area(&self) -> Option<(Px, Px)> {
        let poc = self.poc()?;
        let total = self.total_volume();
        if total == 0 {
            return None;
        }
        let target = (total as f64 * self.value_area_pct).ceil() as u128;

        let prices: Vec<Px> = self.by_price.keys().copied().collect();
        let poc_idx = prices.iter().position(|&p| p == poc).unwrap();
        let vol = |i: usize| self.by_price[&prices[i]] as u128;

        let mut acc = vol(poc_idx);
        let (mut lo, mut hi) = (poc_idx, poc_idx);
        while acc < target && (lo > 0 || hi < prices.len() - 1) {
            let below = if lo > 0 { vol(lo - 1) } else { 0 };
            let above = if hi < prices.len() - 1 {
                vol(hi + 1)
            } else {
                0
            };
            // Étend du côté au plus gros volume ; en cas d'égalité, vers le haut.
            if hi < prices.len() - 1 && (above >= below || lo == 0) {
                hi += 1;
                acc += above;
            } else {
                lo -= 1;
                acc += below;
            }
        }
        Some((prices[lo], prices[hi]))
    }
}

impl BarComponent for VolumeProfile {
    fn on_trade(&mut self, t: &Trade) {
        *self.by_price.entry(t.price).or_insert(0) += t.size;
    }
}

/// **Delta** d'une barre (fiche `DC-1`) : `Σ Buy − Σ Sell`. `Unknown` compte 0.
#[derive(Debug, Default, Clone, Copy)]
pub struct Delta {
    value: i64,
}

impl Delta {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn value(&self) -> i64 {
        self.value
    }
}

impl BarComponent for Delta {
    fn on_trade(&mut self, t: &Trade) {
        match t.aggressor {
            AggressorSide::Buy => self.value += t.size as i64,
            AggressorSide::Sell => self.value -= t.size as i64,
            AggressorSide::Unknown => {}
        }
    }
}

/// **Cumulative Delta** (fiche `DC-2`) : état **inter-barres** (porté par l'agrégateur,
/// pas par la barre). On lui pousse le delta de chaque barre fermée.
#[derive(Debug, Default, Clone, Copy)]
pub struct Cvd {
    cumulative: i64,
}

impl Cvd {
    pub fn new() -> Self {
        Self::default()
    }
    /// Ajoute le delta d'une barre et renvoie le cumul courant.
    pub fn push_bar_delta(&mut self, bar_delta: i64) -> i64 {
        self.cumulative += bar_delta;
        self.cumulative
    }
    pub fn value(&self) -> i64 {
        self.cumulative
    }
}

/// Résultats order flow attachés à une `Bar` fermée (snapshot des lentilles actives).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderFlow {
    pub footprint: Option<Footprint>,
    pub volume_profile: Option<VolumeProfile>,
    /// Delta de la barre (`DC-1`).
    pub delta: Option<i64>,
    /// Cumulative delta à la clôture de cette barre (`DC-2`, état inter-barres).
    pub cvd: Option<i64>,
}

/// Choix de lentilles à activer sur une période (fiche `OF-COMP`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LensKind {
    Footprint,
    VolumeProfile {
        value_area_pct: f64,
    },
    /// Delta de barre (+ CVD cumulé automatiquement au niveau de la période).
    Delta,
}

/// Instance vivante d'une lentille pour la barre en cours.
pub(crate) enum LensInstance {
    Footprint(Footprint),
    VolumeProfile(VolumeProfile),
    Delta(Delta),
}

impl LensInstance {
    pub(crate) fn from_kind(kind: LensKind) -> Self {
        match kind {
            LensKind::Footprint => LensInstance::Footprint(Footprint::new()),
            LensKind::VolumeProfile { value_area_pct } => {
                LensInstance::VolumeProfile(VolumeProfile::new(value_area_pct))
            }
            LensKind::Delta => LensInstance::Delta(Delta::new()),
        }
    }

    pub(crate) fn on_trade(&mut self, t: &Trade) {
        match self {
            LensInstance::Footprint(c) => c.on_trade(t),
            LensInstance::VolumeProfile(c) => c.on_trade(t),
            LensInstance::Delta(c) => c.on_trade(t),
        }
    }

    /// Verse le résultat de la lentille dans `of` ; renvoie le delta si c'en est un.
    pub(crate) fn snapshot_into(&mut self, of: &mut OrderFlow) -> Option<i64> {
        match self {
            LensInstance::Footprint(c) => {
                c.on_close();
                of.footprint = Some(c.clone());
                None
            }
            LensInstance::VolumeProfile(c) => {
                c.on_close();
                of.volume_profile = Some(c.clone());
                None
            }
            LensInstance::Delta(c) => {
                c.on_close();
                of.delta = Some(c.value());
                Some(c.value())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(price: Px, size: Qty, side: AggressorSide) -> Trade {
        Trade {
            ts: 0,
            price,
            size,
            aggressor: side,
            instrument_id: 1,
        }
    }

    fn feed<C: BarComponent>(c: &mut C, trades: &[Trade]) {
        for tr in trades {
            c.on_trade(tr);
        }
        c.on_close();
    }

    // UC-T1-3
    #[test]
    fn footprint_cells_by_price_and_side() {
        let mut fp = Footprint::new();
        feed(
            &mut fp,
            &[
                t(100, 2, AggressorSide::Buy),
                t(100, 5, AggressorSide::Sell),
                t(101, 1, AggressorSide::Buy),
                t(101, 9, AggressorSide::Unknown), // ignoré côté
            ],
        );
        assert_eq!(fp.cell(100), (2, 5));
        assert_eq!(fp.cell(101), (1, 0));
    }

    // UC-T1-4
    #[test]
    fn volume_profile_poc() {
        let mut vp = VolumeProfile::new(0.70);
        feed(
            &mut vp,
            &[
                t(100, 2, AggressorSide::Buy),
                t(100, 5, AggressorSide::Sell),
                t(101, 1, AggressorSide::Buy),
            ],
        );
        assert_eq!(vp.total_volume(), 8);
        assert_eq!(vp.poc(), Some(100)); // 100 → 7, 101 → 1
    }

    // UC-T1-5
    #[test]
    fn volume_profile_value_area() {
        let mut vp = VolumeProfile::new(0.70);
        // Volumes : 98:1, 99:3, 100:10, 101:4, 102:1  (total 19, cible ⌈13.3⌉ = 14)
        feed(
            &mut vp,
            &[
                t(98, 1, AggressorSide::Buy),
                t(99, 3, AggressorSide::Buy),
                t(100, 10, AggressorSide::Buy),
                t(101, 4, AggressorSide::Buy),
                t(102, 1, AggressorSide::Buy),
            ],
        );
        assert_eq!(vp.poc(), Some(100));
        // POC(10) + 101(4) = 14 ≥ 14 → [100,101].
        assert_eq!(vp.value_area(), Some((100, 101)));
    }

    // UC-T1-6
    #[test]
    fn delta_buy_minus_sell() {
        let mut d = Delta::new();
        feed(
            &mut d,
            &[
                t(100, 3, AggressorSide::Buy),
                t(100, 5, AggressorSide::Sell),
                t(100, 9, AggressorSide::Unknown),
            ],
        );
        assert_eq!(d.value(), -2);
    }

    // UC-T1-7
    #[test]
    fn cvd_accumulates_across_bars() {
        let mut cvd = Cvd::new();
        assert_eq!(cvd.push_bar_delta(-2), -2);
        assert_eq!(cvd.push_bar_delta(5), 3);
        assert_eq!(cvd.value(), 3);
    }
}
