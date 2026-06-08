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
use crate::bar::Bar;
use crate::canonical::{AggressorSide, BookAction, BookSide, BookUpdate, MarketEvent, Trade, Ts};
use crate::extension::ChannelSink;
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

/// Helper **DX** (issue #22) : rejoue un fichier de trades dans un `SymbolAggregator`
/// **déjà configuré** et **collecte les barres** fermées, dans l'ordre event-time
/// (`finish()` inclus).
///
/// Variante *builder-friendly* (cf. issue) : l'appelant configure période, lentilles,
/// passif… et a déjà géré l'erreur de configuration (`ConfigError`) au `build()` — ce qui
/// évite de la mélanger à `dbn::Error`. Câble en interne un `ChannelSink` et draine le
/// `Receiver` : équivaut au câblage manuel, sans la plomberie répétée.
///
/// En présence de **plusieurs périodes**, les barres de toutes les périodes sont
/// retournées dans leur ordre de clôture (le label de période est ignoré ici ; utiliser
/// directement un `ChannelSink` si l'on veut discriminer par label).
pub fn replay_to_bars<P: AsRef<Path>>(
    path: P,
    mut agg: SymbolAggregator,
    limit: Option<usize>,
) -> Result<Vec<Bar>, dbn::Error> {
    let (tx, rx) = std::sync::mpsc::channel();
    agg.subscribe(Box::new(ChannelSink::new(tx)));
    replay_trades_file(path, &mut agg, limit)?;
    // Libère le `Sender` détenu par l'agrégateur pour clore l'itération du `Receiver`.
    drop(agg);
    Ok(rx.into_iter().map(|(_label, bar)| bar).collect())
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

/// Élément de carnet dans un flux fusionné : **delta** MBO ou **snapshot** MBP-10
/// (fiche `UC-T5-1`). Permet d'unifier les deux schémas de carnet sous un seul `ts`.
enum BookItem {
    Update(BookUpdate),
    Snapshot { ts: Ts, book: OrderBook },
}

impl BookItem {
    fn ts(&self) -> Ts {
        match self {
            BookItem::Update(u) => u.ts,
            BookItem::Snapshot { ts, .. } => *ts,
        }
    }
}

/// Prochain trade de `instrument_id` dans le flux (saute les autres échéances).
fn next_trade<D: DecodeRecordRef>(
    decoder: &mut D,
    instrument_id: u32,
) -> Result<Option<Trade>, dbn::Error> {
    while let Some(rec) = decoder.decode_record_ref()? {
        if let Some(t) = rec.get::<dbn::TradeMsg>() {
            if t.hd.instrument_id != instrument_id {
                continue;
            }
            return Ok(Some(trade_from_dbn(t)));
        }
    }
    Ok(None)
}

/// Prochain élément de carnet de `instrument_id` : `Mbp10Msg` → snapshot, `MboMsg` →
/// delta. Saute les autres échéances et les actions MBO non liées au book.
fn next_book_item<D: DecodeRecordRef>(
    decoder: &mut D,
    instrument_id: u32,
) -> Result<Option<BookItem>, dbn::Error> {
    while let Some(rec) = decoder.decode_record_ref()? {
        if let Some(m) = rec.get::<dbn::Mbp10Msg>() {
            if m.hd.instrument_id != instrument_id {
                continue;
            }
            return Ok(Some(BookItem::Snapshot {
                ts: m.hd.ts_event as i64,
                book: book_from_mbp10(m),
            }));
        }
        if let Some(mb) = rec.get::<dbn::MboMsg>() {
            if mb.hd.instrument_id != instrument_id {
                continue;
            }
            if let Some(bu) = book_update_from_mbo(mb) {
                return Ok(Some(BookItem::Update(bu)));
            }
            // Action MBO non liée au maintien du book (T/F/R/N) → élément suivant.
        }
    }
    Ok(None)
}

fn apply_book_item(agg: &mut SymbolAggregator, item: BookItem) {
    match item {
        BookItem::Update(u) => agg.process(&MarketEvent::BookUpdate(u)),
        BookItem::Snapshot { ts, book } => agg.ingest_book_snapshot(ts, book),
    }
}

/// Rejoue **en flux fusionné event-time** un fichier de trades et un fichier de carnet
/// (MBP-10 **ou** MBO) dans un **seul** `SymbolAggregator` (fiches `UC-T5-1..4`).
///
/// k-way merge (k=2) par `ts` croissant. **Départage déterministe** à `ts` égal :
/// l'événement **carnet est appliqué avant le trade** (`b.ts() <= t.ts`). À toute clôture
/// de barre déclenchée par un trade à `t`, `agg.book()` reflète donc les mises à jour de
/// carnet jusqu'à `t` inclus.
///
/// Les snapshots MBP-10 sont ingérés via `ingest_book_snapshot`, les deltas MBO via
/// `process(BookUpdate)`. Un seul `finish()` est appelé à la fin. Renvoie le nombre
/// d'événements poussés (trades + carnet) ; `limit` borne ce total.
pub fn replay_merged<P: AsRef<Path>>(
    trades: P,
    book: P,
    agg: &mut SymbolAggregator,
    limit: Option<usize>,
) -> Result<usize, dbn::Error> {
    let instrument_id = agg.instrument().id;
    let mut td = DbnDecoder::from_zstd_file(trades)?;
    let mut bd = DbnDecoder::from_zstd_file(book)?;

    let mut next_t = next_trade(&mut td, instrument_id)?;
    let mut next_b = next_book_item(&mut bd, instrument_id)?;
    let mut n = 0usize;

    loop {
        if let Some(max) = limit
            && n >= max
        {
            break;
        }
        // Décide d'abord (emprunts relâchés), agit ensuite (mutations).
        let take_book = match (&next_t, &next_b) {
            (None, None) => break,
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (Some(t), Some(b)) => b.ts() <= t.ts, // carnet avant trade à ts égal
        };
        if take_book {
            apply_book_item(agg, next_b.take().unwrap());
            next_b = next_book_item(&mut bd, instrument_id)?;
        } else {
            agg.process(&MarketEvent::Trade(next_t.take().unwrap()));
            next_t = next_trade(&mut td, instrument_id)?;
        }
        n += 1;
    }
    agg.finish();
    Ok(n)
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
