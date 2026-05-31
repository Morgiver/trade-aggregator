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
use crate::canonical::{AggressorSide, MarketEvent, Trade};

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
