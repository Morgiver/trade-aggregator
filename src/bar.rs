//! La `Bar` : unité agrégée produite par une `Period` (fiches `AGG-B1`, `AGG-B2`).

use crate::canonical::{Px, Qty, Trade, Ts};

/// Open / High / Low / Close / Volume d'une barre (fiche `AGG-B2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ohlcv {
    pub open: Px,
    pub high: Px,
    pub low: Px,
    pub close: Px,
    pub volume: Qty,
}

/// Une barre agrégée. États : *en formation* puis *fermée* (fiche `AGG-B1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    /// Borne basse (incluse) de la fenêtre, event-time.
    pub start: Ts,
    /// Borne haute (exclue) de la fenêtre.
    pub end: Ts,
    pub ohlcv: Ohlcv,
    /// `true` si la barre a été fermée par un *flush* de fin de flux (fiche `SYM-11`),
    /// donc potentiellement incomplète.
    pub partial: bool,
}

impl Bar {
    /// Démarre une barre `[start, end)` à partir de son premier trade.
    pub(crate) fn open(start: Ts, end: Ts, first: &Trade) -> Self {
        Bar {
            start,
            end,
            ohlcv: Ohlcv {
                open: first.price,
                high: first.price,
                low: first.price,
                close: first.price,
                volume: first.size,
            },
            partial: false,
        }
    }

    /// Intègre un trade dans la barre en formation.
    pub(crate) fn add(&mut self, t: &Trade) {
        let o = &mut self.ohlcv;
        if t.price > o.high {
            o.high = t.price;
        }
        if t.price < o.low {
            o.low = t.price;
        }
        o.close = t.price;
        o.volume += t.size;
    }
}
