//! Mapping **DataBento** (DBN → modèle canonique), isolé derrière la feature `databento`.
//!
//! Fiches `CAN-8` (mapping trades), `CAN-11` (mapping `AggressorSide`), `CAN-12`
//! (isolation : le cœur ne dépend pas de `dbn`). La crate compile **sans** cette feature.
//!
//! DataBento encode le côté **agresseur** par `Bid` (acheteur agresseur) / `Ask`
//! (vendeur agresseur) / `None`. On mappe vers `AggressorSide::{Buy, Sell, Unknown}`.

use std::path::Path;

use dbn::decode::{DbnDecoder, DecodeRecordRef};

use crate::aggregator::SymbolAggregator;
use crate::canonical::{AggressorSide, BookAction, BookSide, BookUpdate, MarketEvent, Trade};
use crate::passive::OrderBook;

/// Convertit le côté DBN (octet `B`/`A`/`N`) en `AggressorSide` (fiche `CAN-11`).
fn aggressor_from_dbn(side: std::os::raw::c_char) -> AggressorSide {
    match side as u8 {
        b'B' => AggressorSide::Buy,  // Bid = acheteur agresseur
        b'A' => AggressorSide::Sell, // Ask = vendeur agresseur
        _ => AggressorSide::Unknown, // 'N' ou inconnu
    }
}

/// Convertit un `dbn::TradeMsg` en `Trade` canonique (fiche `CAN-8`).
pub fn trade_from_dbn(msg: &dbn::TradeMsg) -> Trade {
    Trade {
        ts: msg.hd.ts_event as i64,
        price: msg.price,
        size: msg.size as u64,
        aggressor: aggressor_from_dbn(msg.side),
        instrument_id: msg.hd.instrument_id,
    }
}

/// Lit le premier `instrument_id` rencontré dans un fichier de trades DBN
/// (utilitaire : les fichiers DataBento mêlent plusieurs échéances).
pub fn first_instrument_id<P: AsRef<Path>>(path: P) -> Result<Option<u32>, dbn::Error> {
    let mut decoder = DbnDecoder::from_zstd_file(path)?;
    while let Some(rec) = decoder.decode_record_ref()? {
        if let Some(t) = rec.get::<dbn::TradeMsg>() {
            return Ok(Some(t.hd.instrument_id));
        }
    }
    Ok(None)
}

/// Rejoue un fichier `*.trades.dbn.zst` dans un `SymbolAggregator` (fiche `UC-T0-1`).
///
/// Ne traite que les `TradeMsg`. `limit` borne le nombre de trades lus (utile pour les
/// tests sur de gros fichiers). Appelle `finish()` à la fin. Renvoie le nombre de trades
/// poussés.
pub fn replay_trades_file<P: AsRef<Path>>(
    path: P,
    agg: &mut SymbolAggregator,
    limit: Option<usize>,
) -> Result<usize, dbn::Error> {
    let mut decoder = DbnDecoder::from_zstd_file(path)?;
    let mut n = 0usize;
    while let Some(rec) = decoder.decode_record_ref()? {
        if let Some(t) = rec.get::<dbn::TradeMsg>() {
            agg.process(&MarketEvent::Trade(trade_from_dbn(t)));
            n += 1;
            if let Some(max) = limit
                && n >= max
            {
                break;
            }
        }
    }
    agg.finish();
    Ok(n)
}

/// Reconstruit un `OrderBook` (L2) à partir d'un message **MBP-10** (fiche `CAN-13`).
///
/// Un message MBP donne les meilleurs niveaux du carnet à l'instant `t` (snapshot) :
/// on reconstruit donc le book directement depuis ses niveaux (les niveaux vides,
/// quantité 0, sont ignorés).
pub fn book_from_mbp10(msg: &dbn::Mbp10Msg) -> OrderBook {
    let mut book = OrderBook::new();
    for lvl in msg.levels.iter() {
        if lvl.bid_sz > 0 {
            book.set_level(BookSide::Bid, lvl.bid_px, lvl.bid_sz as u64);
        }
        if lvl.ask_sz > 0 {
            book.set_level(BookSide::Ask, lvl.ask_px, lvl.ask_sz as u64);
        }
    }
    book
}

/// Mappe un message **MBO** en `BookUpdate` canonique (fiche `CAN-9`).
///
/// `A`dd / `C`ancel / `M`odify → `Add`/`Cancel`/`Modify` ; côté `B`id/`A`sk.
/// Renvoie `None` pour les actions non liées au maintien du book (`T`rade, `F`ill,
/// clea`R`, `N`one).
///
/// ⚠️ Reconstruction L3→L2 *fidèle* (suivi par `order_id`, gestion fine des `Modify`
/// multi-ordres par niveau) : raffinement ultérieur. Ici on transmet l'`order_id` brut.
pub fn book_update_from_mbo(msg: &dbn::MboMsg) -> Option<BookUpdate> {
    let action = match msg.action as u8 {
        b'A' => BookAction::Add,
        b'C' => BookAction::Cancel,
        b'M' => BookAction::Modify,
        _ => return None,
    };
    let side = match msg.side as u8 {
        b'B' => BookSide::Bid,
        b'A' => BookSide::Ask,
        _ => return None,
    };
    Some(BookUpdate {
        ts: msg.hd.ts_event as i64,
        action,
        side,
        price: msg.price,
        size: msg.size as u64,
        order_id: Some(msg.order_id),
        instrument_id: msg.hd.instrument_id,
    })
}

/// Rejoue un fichier `*.mbp-10.dbn.zst` en reconstruisant le book à chaque message.
/// Renvoie `(messages lus, messages où le book était croisé)`. `limit` borne la lecture.
pub fn replay_mbp10_file<P: AsRef<Path>>(
    path: P,
    instrument_id: u32,
    limit: Option<usize>,
) -> Result<(usize, usize), dbn::Error> {
    let mut decoder = DbnDecoder::from_zstd_file(path)?;
    let (mut n, mut crossed) = (0usize, 0usize);
    while let Some(rec) = decoder.decode_record_ref()? {
        if let Some(m) = rec.get::<dbn::Mbp10Msg>() {
            if m.hd.instrument_id != instrument_id {
                continue;
            }
            let book = book_from_mbp10(m);
            n += 1;
            if book.is_crossed() {
                crossed += 1;
            }
            if let Some(max) = limit
                && n >= max
            {
                break;
            }
        }
    }
    Ok((n, crossed))
}

/// Premier `instrument_id` rencontré dans un fichier MBP-10.
pub fn first_mbp10_instrument_id<P: AsRef<Path>>(path: P) -> Result<Option<u32>, dbn::Error> {
    let mut decoder = DbnDecoder::from_zstd_file(path)?;
    while let Some(rec) = decoder.decode_record_ref()? {
        if let Some(m) = rec.get::<dbn::Mbp10Msg>() {
            return Ok(Some(m.hd.instrument_id));
        }
    }
    Ok(None)
}
