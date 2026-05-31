//! Modèle d'entrée **canonique** (pilier P3 source-agnostic).
//!
//! Concepts : voir `docs/domain/glossaire.md`. T0 ne manipule que des `Trade`
//! (le `BookUpdate` arrive en tranche T2).

/// Horodatage en **nanosecondes depuis l'epoch UTC** (fiche `TR-4`).
/// Le temps vient *toujours* des données (event-time), jamais de l'horloge système.
pub type Ts = i64;

/// Prix en entier (échelle fixe propre à l'instrument). Garder un entier évite les
/// imprécisions flottantes dans le hot path.
pub type Px = i64;

/// Quantité (volume) d'un trade.
pub type Qty = u64;

/// Côté qui **initie** le trade (fiche `CAN-3`). Distinct du côté du book (`Bid`/`Ask`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggressorSide {
    /// L'agresseur achète (lève l'ask).
    Buy,
    /// L'agresseur vend (frappe le bid).
    Sell,
    /// Côté non fourni par la source (DataBento `None`) — fiche `UC-T0-2`.
    Unknown,
}

/// Une transaction exécutée (fiche `CAN-1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trade {
    pub ts: Ts,
    pub price: Px,
    pub size: Qty,
    pub aggressor: AggressorSide,
    /// Identifiant d'instrument (les fichiers DataBento mêlent plusieurs échéances).
    pub instrument_id: u32,
}

/// Côté du **carnet** (liquidité passive) — distinct de `AggressorSide` (fiche `CAN-2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookSide {
    /// Acheteurs passifs.
    Bid,
    /// Vendeurs passifs.
    Ask,
}

/// Action sur le carnet (fiche `CAN-2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookAction {
    /// Ajoute de la liquidité à un niveau (ou un ordre).
    Add,
    /// Retire de la liquidité.
    Cancel,
    /// Modifie la quantité (et/ou le prix) à un niveau / d'un ordre.
    Modify,
}

/// Événement modifiant le carnet (fiche `CAN-2`).
///
/// `order_id` présent en L3 (market-by-order), absent en L2 (market-by-price).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookUpdate {
    pub ts: Ts,
    pub action: BookAction,
    pub side: BookSide,
    pub price: Px,
    /// Quantité (nouvelle quantité au niveau pour `Modify`, quantité ajoutée/retirée sinon).
    pub size: Qty,
    pub order_id: Option<u64>,
    pub instrument_id: u32,
}

/// Événement de marché horodaté en entrée (fiche `CAN-4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketEvent {
    /// Consommation de liquidité (tape) → côté agressif.
    Trade(Trade),
    /// Modification du carnet → côté passif (T2).
    BookUpdate(BookUpdate),
}

impl MarketEvent {
    /// Timestamp event-time de l'événement.
    pub fn ts(&self) -> Ts {
        match self {
            MarketEvent::Trade(t) => t.ts,
            MarketEvent::BookUpdate(b) => b.ts,
        }
    }
}

/// Définition d'instrument (fiche `CAN-5`). Minimal en T0 ; enrichi plus tard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instrument {
    pub id: u32,
    /// Pas de cotation (en unités de `Px`).
    pub tick_size: Px,
}

/// Richesse de la donnée d'entrée (fiche `CAN-6`).
///
/// Hiérarchie **ordonnée** : `L1 < L2 < L3` (info croissante). On dérive vers le bas,
/// jamais vers le haut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Granularity {
    /// Top of book / BBO (+ last trade).
    L1,
    /// Market-by-price (profondeur par niveau).
    L2,
    /// Market-by-order (par ordre individuel).
    L3,
}
