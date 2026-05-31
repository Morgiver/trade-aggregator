# Use-cases — T2 Côté passif

> Phase 7, étape 1. S'appuie sur T0+T1. Sous-découpé en 3 lots.
> Chaque UC référence ses fiches (cf. [`../../architecture/passive/`](../../architecture/passive/README.md)).

## Lot A — `BookUpdate` + reconstruction de l'`OrderBook`

### `UC-T2-1` — Type `BookUpdate` + variante `MarketEvent::BookUpdate`
`BookUpdate { ts, action: Add|Cancel|Modify, side: Bid|Ask, price, size, order_id? }`. *Fiches* : `CAN-2`, `CAN-4`.

### `UC-T2-2` — Maintenir le book **L2** (par niveau de prix)
Add ajoute/incrémente, Cancel retire/décrémente, Modify ajuste. *Fiches* : `OB-1/2/3/7`.

### `UC-T2-3` — Requêtes book
Best bid/ask, profondeur N niveaux, snapshot. *Fiches* : `OB-9`.

### `UC-T2-4` — Snapshot / clear + resynchronisation
Un snapshot réinitialise le book ; un trou de séquence déclenche un resync. *Fiches* : `OB-5/6`, `TR-7`.

### `UC-T2-5` — Intégrité
Book croisé (bid≥ask) / quantité négative détectés, pas ignorés. *Fiches* : `OB-10`.

### `UC-T2-6` — Routage `BookUpdate` → `PassiveAggregator`
`SymbolAggregator.process` route les book updates vers le côté passif (les trades restent côté agressif). *Fiches* : `SYM-3`, `PAS-1`.

### `UC-T2-7` — Fail-fast L1
Un `PassiveAggregator` est refusé à la construction si la granularité déclarée est L1. *Fiches* : `PAS-3`.

## Lot B — Profils de liquidité

### `UC-T2-8` — Profil pondéré-temps — *fiche* `LP-1`
### `UC-T2-9` — Snapshots ouverture/clôture — *fiche* `LP-2`
### `UC-T2-10` — Churn (add/cancel) — *fiche* `LP-3`
### `UC-T2-11` — Depth (max/min, cumul) + déséquilibre bid/ask — *fiches* `LP-4/5`
### `UC-T2-12` — Émission alignée sur le côté agressif (bornes de `Period`) — *fiches* `LP-6`, `SYM-9`, `PAS-2`, `EXT-7`
### `UC-T2-13` — État interrogeable (snapshot pull du book/profil) — *fiche* `EXT-6`

## Lot C — Mapping DataBento book

### `UC-T2-14` — Mapping MBO (`mbo`) → `BookUpdate` — *fiche* `CAN-9`
### `UC-T2-15` — Mapping MBP-1/10 (L2) → `BookUpdate` — *fiche* `CAN-13`
### `UC-T2-16` — Mapping `definition` → `Instrument` — *fiche* `CAN-10`
### `UC-T2-17` — Replay réel d'un fichier MBO (gated `TRADE_AGG_DATA_DIR`) — validation bout-en-bout

---

## Couverture des fiches T2
`CAN-2/4/9/10/13` · `SYM-3/7/9` · `PAS-1/2/3` · `OB-1…10` · `LP-1…6` · `EXT-6/7` · `TR-7` — **toutes couvertes**.

## Note de conception
Le `PassiveAggregator` réutilise le même mécanisme de `Period` (bornes alignées avec le
côté agressif) ; le book maintenu est échantillonné par fenêtre pour produire les profils.
L3 (MBO par ordre) dérive le L2 ; en l'absence d'`order_id` (L2/MBP), on tient les niveaux agrégés.
