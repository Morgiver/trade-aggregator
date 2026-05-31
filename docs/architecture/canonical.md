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

### `CAN-1` — Modèle `MarketEvent` · **P0**
Type horodaté `Trade | BookUpdate` couvrant tape et book.
**Critère** : un flux DataBento se décode en une séquence de `MarketEvent` sans perte.

### `CAN-2` — `Instrument` definition · **P0**
Tick size, price increment, lot/contract size, multiplicateur, devise.
**Critère** : les prix d'un symbole se calent sur son tick size ; le notional est calculable.

### `CAN-3` — `Granularity` (L1/L2/L3) + capacités · **P0**
Déclarée à la construction ; expose les agrégations possibles.
**Critère** : demander une agrégation non supportée par la granularité → erreur typée.

### `CAN-4` — Mapping DataBento (`dbn`) · **P0** (trades) / **P1** (book)
Module isolé/feature-gated ; `Ask/Bid` agresseur → `AggressorSide::Sell/Buy`.
**Critère** : la crate compile sans la feature `databento` ; avec, un fichier DBN se mappe.
