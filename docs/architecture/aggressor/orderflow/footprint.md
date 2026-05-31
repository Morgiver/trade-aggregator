# orderflow/footprint — Footprint

> Feuille. Parent : [`README.md`](README.md). Concept :
> [`../../../domain/glossaire.md`](../../../domain/glossaire.md).

**Rôle** : pour une `Bar`, répartir le **volume échangé par niveau de prix et par côté**
(`Bid` vs `Ask`). Lentille **volume**.

## Accumulation
- `on_trade` : `cells[trade.price][trade.aggressor_side] += trade.size`
  (Buy → colonne Ask consommée / Sell → colonne Bid consommée, selon convention à fixer
  en Phase 7).
- Structure : `prix → (volume_buy, volume_sell)`.

## Sorties
- La grille footprint de la `Bar` (volume Buy/Sell par prix).
- Base de calcul du `Delta` (cf. [`delta-cvd.md`](delta-cvd.md)) et des imbalances
  diagonales (donnée brute — l'interprétation reste au consommateur).

## Granularité
N'a besoin que des `Trade` (+ `AggressorSide`). Indépendant du book.
