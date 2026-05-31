//! Côté **passif** : reconstruction du carnet (`OrderBook`) et agrégation (nœud `passive/`).
//!
//! T2 lot A : `OrderBook` (reconstruction L2 par niveau de prix) + `PassiveAggregator`
//! (maintient le book). Les profils de liquidité périodiques arrivent au lot B.

use std::collections::BTreeMap;

use crate::canonical::{BookAction, BookSide, BookUpdate, Px, Qty};

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
#[derive(Debug, Default, Clone)]
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
}

/// Agrégateur passif : maintient l'`OrderBook` (fiche `PAS-1`). Les profils périodiques
/// (lot B) viendront s'appuyer dessus.
#[derive(Debug, Default)]
pub struct PassiveAggregator {
    book: OrderBook,
}

impl PassiveAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applique un `BookUpdate` au carnet maintenu.
    pub fn apply(&mut self, u: &BookUpdate) -> Result<(), BookError> {
        self.book.apply(u)
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
