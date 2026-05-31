# canonical — Modèle d'entrée canonique

> Feuille. Parent : [`README.md`](README.md). Concepts :
> [`../domain/glossaire.md`](../domain/glossaire.md).
>
> **Rôle** : le **contrat d'entrée** unique de la crate. Tout ce qui entre est un
> `MarketEvent` canonique. Pilier **P3** (source-agnostic).

## Contenu
- **`MarketEvent`** : `Trade` | `BookUpdate`, horodaté.
  - `Trade { ts, price, size, aggressor_side }`
  - `BookUpdate { ts, action: Add|Cancel|Modify|Fill, side: Bid|Ask, price, size, order_id? }`
- **`Instrument`** : tick size, price increment, lot/contract size, multiplicateur, devise.
- **`Granularity`** : `L1 | L2 | L3` (déclarée à la construction).
- **`AggressorSide`** : `Buy | Sell` (≠ `Bid`/`Ask`).

## Mapping DataBento (isolé / feature-gated)
- Module séparé qui traduit les records `dbn` (MBO / Trades / definition) → `MarketEvent`
  canonique. DataBento `Ask`/`Bid` (agresseur) → `AggressorSide::Sell`/`Buy`.
- **Le cœur ne dépend pas de `dbn`** : derrière une feature, remplaçable, et la crate
  compile sans.

## Frontière
- Les **adapters par venue** (Binance/Bybit/Coinbase) sont **hors scope** : ils
  produiraient ce `MarketEvent` depuis l'extérieur (projet compagnon).
- Aucune logique d'agrégation ici : juste le **vocabulaire d'entrée** typé.

---

## Fiches features (Phase 5)

> Atomisation des features du thème A ([`../vision/features.md`](../vision/features.md))
> rattachées à ce nœud. `ID · priorité · critère`.

- **`CAN-1`** — Type `Trade` (ts, price, size, aggressor_side) · **P0** · *un trade brut se représente sans perte.*
- **`CAN-2`** — Type `BookUpdate` (ts, action, side, price, size, order_id?) · **P0** · *un event de book se représente sans perte.*
- **`CAN-3`** — `AggressorSide { Buy, Sell }` · **P0** · *distinct de Bid/Ask ; valeur « inconnu » possible.*
- **`CAN-4`** — Enum `MarketEvent` horodaté (`Trade | BookUpdate`) · **P0** · *séquence unique ordonnable par ts.*
- **`CAN-5`** — `Instrument` (tick size, price increment, lot/contract size, multiplicateur, devise) · **P0** · *prix calés sur le tick ; notional calculable.*
- **`CAN-6`** — Enum `Granularity { L1, L2, L3 }` · **P0** · *déclarée à la construction.*
- **`CAN-7`** — Table de capacités (granularité → agrégations permises) · **P0** · *expose si une agrégation est supportée.*
- **`CAN-8`** — Mapping DataBento : schéma `trades` → `Trade` · **P0** · *fichier trades DBN → `Trade`.*
- **`CAN-9`** — Mapping DataBento : schéma `mbo` → `BookUpdate` · **P1** · *fichier MBO DBN → `BookUpdate`.*
- **`CAN-10`** — Mapping DataBento : `definition` → `Instrument` · **P1** · *définition DBN → `Instrument`.*
- **`CAN-11`** — Mapping `AggressorSide` (DBN `Ask/Bid` → `Sell/Buy`) · **P0** · *convention agresseur correcte.*
- **`CAN-12`** — Isolation feature-gate `databento` · **P0** · *la crate compile sans la feature ; le cœur ne dépend pas de `dbn`.*
