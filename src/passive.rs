//! Côté **passif** : reconstruction du carnet (`OrderBook`) et agrégation (nœud `passive/`).
//!
//! T2 lot A : `OrderBook` (reconstruction L2 par niveau de prix) + `PassiveAggregator`
//! (maintient le book). Les profils de liquidité périodiques arrivent au lot B.

use std::collections::BTreeMap;

use crate::canonical::{BookAction, BookSide, BookUpdate, Px, Qty, Ts};

/// Anomalie d'intégrité du carnet (fiche `OB-10` / `TR-7`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookError {
    /// Annulation/réduction sous zéro à un niveau (quantité bornée à 0).
    NegativeLevel { side: BookSide, price: Px },
}

/// Carnet d'ordres reconstruit, agrégé **par niveau de prix** (L2).
///
/// Le L3 (market-by-order) se dérive vers ce L2 en amont (mapping), via le suivi des
/// `order_id`. Ici on tient `prix → quantité` par côté.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OrderBook {
    bids: BTreeMap<Px, Qty>,
    asks: BTreeMap<Px, Qty>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Réinitialise le book (snapshot/clear, fiche `OB-5`).
    pub fn clear(&mut self) {
        self.bids.clear();
        self.asks.clear();
    }

    fn side_mut(&mut self, side: BookSide) -> &mut BTreeMap<Px, Qty> {
        match side {
            BookSide::Bid => &mut self.bids,
            BookSide::Ask => &mut self.asks,
        }
    }

    /// Applique un `BookUpdate` (fiches `OB-1/2/3/7`). Renvoie `Err` sur anomalie
    /// d'intégrité (quantité bornée à 0), sans paniquer.
    pub fn apply(&mut self, u: &BookUpdate) -> Result<(), BookError> {
        let levels = self.side_mut(u.side);
        match u.action {
            BookAction::Add => {
                *levels.entry(u.price).or_insert(0) += u.size;
                Ok(())
            }
            BookAction::Modify => {
                if u.size == 0 {
                    levels.remove(&u.price);
                } else {
                    levels.insert(u.price, u.size);
                }
                Ok(())
            }
            BookAction::Cancel => {
                if let Some(q) = levels.get_mut(&u.price) {
                    if u.size >= *q {
                        let underflow = u.size > *q;
                        levels.remove(&u.price);
                        if underflow {
                            return Err(BookError::NegativeLevel {
                                side: u.side,
                                price: u.price,
                            });
                        }
                    } else {
                        *q -= u.size;
                    }
                    Ok(())
                } else {
                    Err(BookError::NegativeLevel {
                        side: u.side,
                        price: u.price,
                    })
                }
            }
        }
    }

    /// Meilleur bid (prix le plus haut côté Bid) — fiche `OB-9`.
    pub fn best_bid(&self) -> Option<(Px, Qty)> {
        self.bids.iter().next_back().map(|(&p, &q)| (p, q))
    }

    /// Meilleur ask (prix le plus bas côté Ask) — fiche `OB-9`.
    pub fn best_ask(&self) -> Option<(Px, Qty)> {
        self.asks.iter().next().map(|(&p, &q)| (p, q))
    }

    /// `n` premiers niveaux d'un côté, du meilleur au moins bon — fiche `OB-9`.
    pub fn depth(&self, side: BookSide, n: usize) -> Vec<(Px, Qty)> {
        match side {
            BookSide::Bid => self
                .bids
                .iter()
                .rev()
                .take(n)
                .map(|(&p, &q)| (p, q))
                .collect(),
            BookSide::Ask => self.asks.iter().take(n).map(|(&p, &q)| (p, q)).collect(),
        }
    }

    /// Le carnet est-il **croisé** (meilleur bid ≥ meilleur ask) ? — fiche `OB-10`.
    pub fn is_crossed(&self) -> bool {
        match (self.best_bid(), self.best_ask()) {
            (Some((b, _)), Some((a, _))) => b >= a,
            _ => false,
        }
    }

    /// Fixe directement la quantité d'un niveau (0 = retire). Utile pour reconstruire
    /// depuis un snapshot par niveaux (MBP).
    pub fn set_level(&mut self, side: BookSide, price: Px, qty: Qty) {
        let levels = self.side_mut(side);
        if qty == 0 {
            levels.remove(&price);
        } else {
            levels.insert(price, qty);
        }
    }

    /// Quantité totale d'un côté (somme des niveaux).
    pub fn total_qty(&self, side: BookSide) -> Qty {
        match side {
            BookSide::Bid => self.bids.values().copied().sum(),
            BookSide::Ask => self.asks.values().copied().sum(),
        }
    }
}

/// Profil de liquidité périodique (fiches `LP-1…6`) : résumé de l'état du carnet sur une
/// fenêtre alignée sur l'horloge (mêmes bornes que le côté agressif `AlignedTimePeriod`).
#[derive(Debug, Clone, PartialEq)]
pub struct LiquidityProfile {
    pub start: Ts,
    pub end: Ts,
    /// Snapshots du carnet à l'ouverture et à la clôture de la fenêtre (`LP-2`).
    pub open: OrderBook,
    pub close: OrderBook,
    /// Churn : volumes ajoutés / annulés sur la fenêtre (`LP-3`).
    pub add_volume: Qty,
    pub cancel_volume: Qty,
    /// Quantité **moyenne pondérée par le temps** par côté (`LP-1` côté agrégé / `LP-4`).
    pub tw_bid: f64,
    pub tw_ask: f64,
    /// Fenêtre fermée par un flush de fin de flux (incomplète).
    pub partial: bool,
}

impl LiquidityProfile {
    /// Déséquilibre bid/ask pondéré-temps ∈ [−1, 1] (`LP-5`). `>0` = plus de bid.
    pub fn imbalance(&self) -> f64 {
        let tot = self.tw_bid + self.tw_ask;
        if tot == 0.0 {
            0.0
        } else {
            (self.tw_bid - self.tw_ask) / tot
        }
    }
}

/// État d'accumulation d'une fenêtre passive en cours.
struct Window {
    start: Ts,
    end: Ts,
    open: OrderBook,
    add_volume: Qty,
    cancel_volume: Qty,
    last_ts: Ts,
    acc_bid: f64,
    acc_ask: f64,
}

/// Agrégateur passif : maintient l'`OrderBook` (fiche `PAS-1`) et, si une fenêtre est
/// configurée, produit des `LiquidityProfile` périodiques alignés (fiches `LP-*`).
pub struct PassiveAggregator {
    book: OrderBook,
    window_ns: Option<i64>,
    window: Option<Window>,
    closed: Vec<LiquidityProfile>,
}

impl Default for PassiveAggregator {
    fn default() -> Self {
        PassiveAggregator {
            book: OrderBook::new(),
            window_ns: None,
            window: None,
            closed: Vec::new(),
        }
    }
}

impl PassiveAggregator {
    /// Carnet seul, sans profils périodiques.
    pub fn new() -> Self {
        Self::default()
    }

    /// Avec profils périodiques de `window_ns` (fenêtres alignées sur l'horloge).
    pub fn with_window(window_ns: i64) -> Self {
        assert!(window_ns > 0, "window_ns doit être > 0");
        PassiveAggregator {
            window_ns: Some(window_ns),
            ..Self::default()
        }
    }

    fn aligned_window(&self, ts: Ts, w: i64) -> (Ts, Ts) {
        let start = ts.div_euclid(w) * w;
        (start, start + w)
    }

    /// Intègre les totaux courants du book sur `[last_ts, up_to]`.
    fn integrate(win: &mut Window, book: &OrderBook, up_to: Ts) {
        let dt = (up_to - win.last_ts) as f64;
        if dt > 0.0 {
            win.acc_bid += book.total_qty(BookSide::Bid) as f64 * dt;
            win.acc_ask += book.total_qty(BookSide::Ask) as f64 * dt;
            win.last_ts = up_to;
        }
    }

    fn finalize(&mut self, end: Ts, partial: bool) {
        if let Some(win) = self.window.take() {
            let duration = (end - win.start).max(1) as f64;
            self.closed.push(LiquidityProfile {
                start: win.start,
                end,
                open: win.open,
                close: self.book.clone(),
                add_volume: win.add_volume,
                cancel_volume: win.cancel_volume,
                tw_bid: win.acc_bid / duration,
                tw_ask: win.acc_ask / duration,
                partial,
            });
        }
    }

    /// Applique un `BookUpdate` au carnet et alimente le profil de la fenêtre courante.
    pub fn apply(&mut self, u: &BookUpdate) -> Result<(), BookError> {
        if let Some(w) = self.window_ns {
            // Fermer les fenêtres traversées, ouvrir celle de `u.ts`.
            let crosses = self
                .window
                .as_ref()
                .map(|win| u.ts >= win.end)
                .unwrap_or(true);
            if crosses {
                if let Some(win) = self.window.as_mut() {
                    let end = win.end;
                    PassiveAggregator::integrate(win, &self.book, end);
                    self.finalize(end, false);
                }
                let (start, end) = self.aligned_window(u.ts, w);
                self.window = Some(Window {
                    start,
                    end,
                    open: self.book.clone(),
                    add_volume: 0,
                    cancel_volume: 0,
                    last_ts: start,
                    acc_bid: 0.0,
                    acc_ask: 0.0,
                });
            }
            // Intègre jusqu'à l'instant de l'update, puis comptabilise le churn.
            if let Some(win) = self.window.as_mut() {
                PassiveAggregator::integrate(win, &self.book, u.ts);
                match u.action {
                    BookAction::Add => win.add_volume += u.size,
                    BookAction::Cancel => win.cancel_volume += u.size,
                    BookAction::Modify => {}
                }
            }
        }
        self.book.apply(u)
    }

    /// Ferme la fenêtre en cours (flush de fin de flux) — fiche `SYM-11`/`LP-6`.
    pub fn finish(&mut self) {
        if let Some(win) = self.window.as_mut() {
            let last = win.last_ts.max(win.start);
            PassiveAggregator::integrate(win, &self.book, last);
            let end = win.end;
            self.finalize(end, true);
        }
    }

    /// Récupère et vide les profils fermés (consommation pull, fiche `EXT-6`).
    pub fn drain_profiles(&mut self) -> Vec<LiquidityProfile> {
        std::mem::take(&mut self.closed)
    }

    /// Accès en lecture au carnet courant (fiche `EXT-6` — état interrogeable).
    pub fn book(&self) -> &OrderBook {
        &self.book
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upd(action: BookAction, side: BookSide, price: Px, size: Qty) -> BookUpdate {
        BookUpdate {
            ts: 0,
            action,
            side,
            price,
            size,
            order_id: None,
            instrument_id: 1,
        }
    }

    // UC-T2-2 / UC-T2-3
    #[test]
    fn add_cancel_modify_and_queries() {
        let mut b = OrderBook::new();
        b.apply(&upd(BookAction::Add, BookSide::Bid, 100, 5))
            .unwrap();
        b.apply(&upd(BookAction::Add, BookSide::Bid, 99, 3))
            .unwrap();
        b.apply(&upd(BookAction::Add, BookSide::Ask, 101, 4))
            .unwrap();
        assert_eq!(b.best_bid(), Some((100, 5)));
        assert_eq!(b.best_ask(), Some((101, 4)));

        // Modify met la quantité du niveau.
        b.apply(&upd(BookAction::Modify, BookSide::Bid, 100, 2))
            .unwrap();
        assert_eq!(b.best_bid(), Some((100, 2)));

        // Cancel partiel puis total.
        b.apply(&upd(BookAction::Cancel, BookSide::Bid, 100, 1))
            .unwrap();
        assert_eq!(b.best_bid(), Some((100, 1)));
        b.apply(&upd(BookAction::Cancel, BookSide::Bid, 100, 1))
            .unwrap();
        assert_eq!(b.best_bid(), Some((99, 3)));

        assert_eq!(b.depth(BookSide::Bid, 5), vec![(99, 3)]);
    }

    // UC-T2-5 : intégrité (cancel sous zéro).
    #[test]
    fn cancel_below_zero_is_an_error() {
        let mut b = OrderBook::new();
        b.apply(&upd(BookAction::Add, BookSide::Ask, 101, 2))
            .unwrap();
        let err = b.apply(&upd(BookAction::Cancel, BookSide::Ask, 101, 5));
        assert_eq!(
            err,
            Err(BookError::NegativeLevel {
                side: BookSide::Ask,
                price: 101
            })
        );
        assert_eq!(b.best_ask(), None, "le niveau est retiré (borné à 0)");
    }

    #[test]
    fn crossed_book_is_detected() {
        let mut b = OrderBook::new();
        b.apply(&upd(BookAction::Add, BookSide::Bid, 101, 1))
            .unwrap();
        b.apply(&upd(BookAction::Add, BookSide::Ask, 100, 1))
            .unwrap();
        assert!(b.is_crossed());
    }

    // UC-T2-4 : clear (resync).
    #[test]
    fn clear_resets_book() {
        let mut b = OrderBook::new();
        b.apply(&upd(BookAction::Add, BookSide::Bid, 100, 5))
            .unwrap();
        b.clear();
        assert_eq!(b.best_bid(), None);
    }
}
