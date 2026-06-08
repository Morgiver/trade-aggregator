//! Lentilles **order flow** attachées à une `Bar` (nœud `aggressor/orderflow`).
//!
//! Lot A de la tranche T1. Chaque lentille est un **accumulateur composable**
//! (`BarComponent`), câblé dans `SymbolAggregator` via `LensKind` (composabilité) et
//! exposé à la clôture dans `OrderFlow` attaché à la `Bar`.

use std::collections::{BTreeMap, BTreeSet};

use crate::canonical::{AggressorSide, Px, Qty, Trade, Ts};

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

    /// Fenêtre **à largeur fixe** indexée par offset de tick autour d'une ancre (issue #20).
    ///
    /// Renvoie `2*half_width + 1` cellules `(buy, sell)`, du prix `anchor - half_width·tick`
    /// (indice `0`) au prix `anchor + half_width·tick` (dernier indice) ; l'ancre est à
    /// l'indice `half_width`. Les cellules absentes valent `(0, 0)`.
    ///
    /// Matérialise l'indexation par tick **une fois pour toutes** (le crate connaît la
    /// sémantique des cellules) → évite à chaque consommateur le bug d'« un tick ». Sur une
    /// période **bornée en prix** (`Range(R)` → `R+1` niveaux, `Renko` → grille bornée),
    /// un `half_width` couvrant la borne ne perd aucune cellule. On renvoie le **brut** ;
    /// la normalisation/imbalance reste au consommateur (non-goal).
    pub fn window(&self, anchor: Px, tick_size: Px, half_width: usize) -> Vec<(Qty, Qty)> {
        assert!(tick_size > 0, "tick_size doit être > 0");
        let n = 2 * half_width + 1;
        let start = anchor - (half_width as i64) * tick_size;
        (0..n)
            .map(|i| self.cell(start + (i as i64) * tick_size))
            .collect()
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

/// **Nombre de trades** par côté agresseur (issue #19) : `(buy_count, sell_count)`.
///
/// `Unknown` est ignoré (comme `Delta`). Couplé au volume, `volume / nombre_de_trades`
/// donne la **taille moyenne de trade** (signal de microstructure : sweeps, icebergs) —
/// calcul laissé au consommateur (on n'expose que les comptes bruts).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TradeCount {
    buy: u64,
    sell: u64,
}

impl TradeCount {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn buy(&self) -> u64 {
        self.buy
    }
    pub fn sell(&self) -> u64 {
        self.sell
    }
    pub fn total(&self) -> u64 {
        self.buy + self.sell
    }
    /// `(buy_count, sell_count)`.
    pub fn pair(&self) -> (u64, u64) {
        (self.buy, self.sell)
    }
}

impl BarComponent for TradeCount {
    fn on_trade(&mut self, t: &Trade) {
        match t.aggressor {
            AggressorSide::Buy => self.buy += 1,
            AggressorSide::Sell => self.sell += 1,
            AggressorSide::Unknown => {}
        }
    }
}

/// **VWAP** de barre (issue #19) : `Σ(price·size) / Σ size`, **tous trades** (ancre de
/// prix côté-agnostique, agrégation pure). `None` si volume nul.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Vwap {
    /// `Σ price·size` (i128 : borne large contre l'overflow).
    num: i128,
    /// `Σ size`.
    den: u128,
}

impl Vwap {
    pub fn new() -> Self {
        Self::default()
    }
    /// VWAP courant ; `None` si aucun volume agrégé.
    pub fn value(&self) -> Option<f64> {
        if self.den == 0 {
            None
        } else {
            Some(self.num as f64 / self.den as f64)
        }
    }
}

impl BarComponent for Vwap {
    fn on_trade(&mut self, t: &Trade) {
        self.num += t.price as i128 * t.size as i128;
        self.den += t.size as u128;
    }
}

/// **TPO / Market Profile** (fiches `TPO-1…5`) : distribution du **temps** par niveau de
/// prix sur la barre, via des *brackets* (sous-périodes de durée `bracket_ns`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tpo {
    bracket_ns: i64,
    ib_brackets: u32,
    bar_start: Option<Ts>,
    /// `prix → ensemble des brackets l'ayant touché`.
    by_price: BTreeMap<Px, BTreeSet<u32>>,
}

impl Tpo {
    /// `bracket_ns` = durée d'un bracket ; `ib_brackets` = nb de brackets de l'Initial Balance.
    pub fn new(bracket_ns: i64, ib_brackets: u32) -> Self {
        assert!(bracket_ns > 0, "bracket_ns doit être > 0");
        Tpo {
            bracket_ns,
            ib_brackets,
            bar_start: None,
            by_price: BTreeMap::new(),
        }
    }

    fn count(&self, price: Px) -> usize {
        self.by_price.get(&price).map(|s| s.len()).unwrap_or(0)
    }

    fn total(&self) -> usize {
        self.by_price.values().map(|s| s.len()).sum()
    }

    /// `POC` temps : prix le plus visité (le plus bas en cas d'égalité) — `TPO-3`.
    pub fn poc(&self) -> Option<Px> {
        self.by_price
            .iter()
            .max_by(|a, b| a.1.len().cmp(&b.1.len()).then(b.0.cmp(a.0)))
            .map(|(&p, _)| p)
    }

    /// `Value Area` temps (~`pct` des TPO autour du POC) — `TPO-3`.
    pub fn value_area(&self, pct: f64) -> Option<(Px, Px)> {
        let poc = self.poc()?;
        let total = self.total();
        if total == 0 {
            return None;
        }
        let target = (total as f64 * pct).ceil() as usize;
        let prices: Vec<Px> = self.by_price.keys().copied().collect();
        let i0 = prices.iter().position(|&p| p == poc).unwrap();
        let (mut lo, mut hi, mut acc) = (i0, i0, self.count(poc));
        while acc < target && (lo > 0 || hi < prices.len() - 1) {
            let below = if lo > 0 {
                self.count(prices[lo - 1])
            } else {
                0
            };
            let above = if hi < prices.len() - 1 {
                self.count(prices[hi + 1])
            } else {
                0
            };
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

    /// `Single prints` : prix touchés par **un seul** bracket — `TPO-4`.
    pub fn single_prints(&self) -> Vec<Px> {
        self.by_price
            .iter()
            .filter(|(_, s)| s.len() == 1)
            .map(|(&p, _)| p)
            .collect()
    }

    /// `Initial Balance` : fourchette de prix des `ib_brackets` premiers brackets — `TPO-5`.
    pub fn initial_balance(&self) -> Option<(Px, Px)> {
        let ib: Vec<Px> = self
            .by_price
            .iter()
            .filter(|(_, s)| s.iter().any(|&b| b < self.ib_brackets))
            .map(|(&p, _)| p)
            .collect();
        match (ib.iter().min(), ib.iter().max()) {
            (Some(&lo), Some(&hi)) => Some((lo, hi)),
            _ => None,
        }
    }
}

impl BarComponent for Tpo {
    fn on_trade(&mut self, t: &Trade) {
        let start = *self.bar_start.get_or_insert(t.ts);
        let idx = ((t.ts - start) / self.bracket_ns) as u32;
        self.by_price.entry(t.price).or_default().insert(idx);
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
    /// Profil TPO / Market Profile (`TPO-*`).
    pub tpo: Option<Tpo>,
    /// Nombre de trades `(buy, sell)` (issue #19, `Unknown` ignoré).
    pub trade_count: Option<(u64, u64)>,
    /// VWAP de la barre (issue #19).
    pub vwap: Option<f64>,
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
    /// TPO / Market Profile (`bracket_ns` = durée d'un bracket ; `ib_brackets` = IB).
    Tpo {
        bracket_ns: i64,
        ib_brackets: u32,
    },
    /// Nombre de trades `(buy, sell)` (issue #19).
    TradeCount,
    /// VWAP de barre (issue #19).
    Vwap,
}

/// Instance vivante d'une lentille pour la barre en cours.
pub(crate) enum LensInstance {
    Footprint(Footprint),
    VolumeProfile(VolumeProfile),
    Delta(Delta),
    Tpo(Tpo),
    TradeCount(TradeCount),
    Vwap(Vwap),
}

impl LensInstance {
    pub(crate) fn from_kind(kind: LensKind) -> Self {
        match kind {
            LensKind::Footprint => LensInstance::Footprint(Footprint::new()),
            LensKind::VolumeProfile { value_area_pct } => {
                LensInstance::VolumeProfile(VolumeProfile::new(value_area_pct))
            }
            LensKind::Delta => LensInstance::Delta(Delta::new()),
            LensKind::Tpo {
                bracket_ns,
                ib_brackets,
            } => LensInstance::Tpo(Tpo::new(bracket_ns, ib_brackets)),
            LensKind::TradeCount => LensInstance::TradeCount(TradeCount::new()),
            LensKind::Vwap => LensInstance::Vwap(Vwap::new()),
        }
    }

    pub(crate) fn on_trade(&mut self, t: &Trade) {
        match self {
            LensInstance::Footprint(c) => c.on_trade(t),
            LensInstance::VolumeProfile(c) => c.on_trade(t),
            LensInstance::Delta(c) => c.on_trade(t),
            LensInstance::Tpo(c) => c.on_trade(t),
            LensInstance::TradeCount(c) => c.on_trade(t),
            LensInstance::Vwap(c) => c.on_trade(t),
        }
    }

    /// Variante **lecture-seule** de `snapshot_into` (issue #31) : verse l'état **courant**
    /// de la lentille dans `of` **sans muter** la lentille ni clôturer la barre. Renvoie le
    /// delta de la barre courante si c'en est un (pour calculer le CVD courant). Utilisée
    /// pour interroger l'order flow d'une barre **en formation** — le coût (clones) n'est
    /// payé qu'à l'appel, donc le hot path reste intact.
    pub(crate) fn snapshot_ref(&self, of: &mut OrderFlow) -> Option<i64> {
        match self {
            LensInstance::Footprint(c) => {
                of.footprint = Some(c.clone());
                None
            }
            LensInstance::VolumeProfile(c) => {
                of.volume_profile = Some(c.clone());
                None
            }
            LensInstance::Delta(c) => {
                of.delta = Some(c.value());
                Some(c.value())
            }
            LensInstance::Tpo(c) => {
                of.tpo = Some(c.clone());
                None
            }
            LensInstance::TradeCount(c) => {
                of.trade_count = Some(c.pair());
                None
            }
            LensInstance::Vwap(c) => {
                of.vwap = c.value();
                None
            }
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
            LensInstance::Tpo(c) => {
                c.on_close();
                of.tpo = Some(c.clone());
                None
            }
            LensInstance::TradeCount(c) => {
                c.on_close();
                of.trade_count = Some(c.pair());
                None
            }
            LensInstance::Vwap(c) => {
                c.on_close();
                of.vwap = c.value();
                None
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

    // UC-T5-9 : fenêtre footprint à largeur fixe, indexée par offset de tick.
    #[test]
    fn footprint_window_fixed_width_and_offset() {
        let mut fp = Footprint::new();
        feed(
            &mut fp,
            &[
                t(100, 2, AggressorSide::Buy),
                t(100, 5, AggressorSide::Sell),
                t(102, 1, AggressorSide::Buy),
            ],
        );
        // anchor=100, tick=2, half_width=2 → prix 96,98,100,102,104 (5 cellules).
        let w = fp.window(100, 2, 2);
        assert_eq!(w.len(), 5, "2*half_width + 1");
        assert_eq!(w[2], (2, 5), "ancre à l'indice half_width");
        assert_eq!(w[3], (1, 0), "102");
        assert_eq!(w[0], (0, 0), "96 absent → (0,0)");
        assert_eq!(w[1], (0, 0), "98 absent → (0,0)");
        assert_eq!(w[4], (0, 0), "104 absent → (0,0)");
    }

    #[test]
    fn footprint_window_half_width_zero_is_single_cell() {
        let mut fp = Footprint::new();
        feed(&mut fp, &[t(100, 3, AggressorSide::Buy)]);
        assert_eq!(fp.window(100, 1, 0), vec![(3, 0)]);
        assert_eq!(fp.window(101, 1, 0), vec![(0, 0)]);
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

    // UC-T5-7 : TradeCount (buy/sell, Unknown ignoré).
    #[test]
    fn trade_count_buy_sell_unknown() {
        let mut tc = TradeCount::new();
        feed(
            &mut tc,
            &[
                t(100, 3, AggressorSide::Buy),
                t(101, 1, AggressorSide::Buy),
                t(100, 5, AggressorSide::Sell),
                t(100, 9, AggressorSide::Unknown), // ignoré
            ],
        );
        assert_eq!(tc.pair(), (2, 1));
        assert_eq!(tc.total(), 3); // Unknown non compté
    }

    // UC-T5-8 : VWAP = Σ(price·size)/Σ size, tous trades, cohérent avec [low, high].
    #[test]
    fn vwap_value_all_trades() {
        let mut v = Vwap::new();
        // (100·2 + 110·3 + 105·5) / (2+3+5) = (200+330+525)/10 = 1055/10 = 105.5
        feed(
            &mut v,
            &[
                t(100, 2, AggressorSide::Buy),
                t(110, 3, AggressorSide::Sell),
                t(105, 5, AggressorSide::Unknown), // inclus (côté-agnostique)
            ],
        );
        let vwap = v.value().unwrap();
        assert!((vwap - 105.5).abs() < 1e-9);
        assert!((100.0..=110.0).contains(&vwap), "VWAP ∈ [low, high]");
    }

    #[test]
    fn vwap_empty_is_none() {
        assert_eq!(Vwap::new().value(), None);
    }

    // UC-T3-5..8 : TPO / Market Profile.
    #[test]
    fn tpo_profile() {
        // bracket 0 : t0,t5 @100 ; bracket 1 : t12 @101, t15 @100 ; bracket 2 : t25 @102
        let trades = [(0i64, 100i64), (5, 100), (12, 101), (15, 100), (25, 102)];
        let mut tpo = Tpo::new(10, 2);
        for (ts, price) in trades {
            tpo.on_trade(&Trade {
                ts,
                price,
                size: 1,
                aggressor: AggressorSide::Buy,
                instrument_id: 1,
            });
        }
        tpo.on_close();
        assert_eq!(tpo.poc(), Some(100)); // 100 touché par brackets {0,1}
        assert_eq!(tpo.single_prints(), vec![101, 102]); // touchés par 1 bracket
        assert_eq!(tpo.value_area(0.70), Some((100, 101)));
        assert_eq!(tpo.initial_balance(), Some((100, 101))); // brackets 0 et 1
    }
}
