# Use-cases — T1 Cœur agressif

> Phase 7, étape 1. S'appuie sur T0 (mergée). Chaque UC référence ses fiches.
> Grosse tranche → **sous-découpée en 3 lots** qui s'enchaînent (chacun vert, intégrable).

## Lot A — Order flow (sur les barres existantes)

### `UC-T1-1` — Trait `BarComponent`
Une lentille implémente `on_trade(&Trade)` + `on_close()`. *Fiches* : `OF-0`.

### `UC-T1-2` — Composer des lentilles sur une période
À la config d'une `Period`, choisir les lentilles voulues ; chaque trade les alimente. *Fiches* : `OF-COMP`.

### `UC-T1-3` — Footprint
Volume par `(prix, côté)` dans la barre. *Fiches* : `FP-1`, `FP-2`.

### `UC-T1-4` — VolumeProfile + POC
Distribution volume par prix ; `POC` = niveau max à la clôture. *Fiches* : `VP-1`, `VP-2`.

### `UC-T1-5` — Value Area
Plage ~70 % autour du POC (seuil paramétrable). *Fiches* : `VP-3`.

### `UC-T1-6` — Delta
`Σ buy − Σ sell` de la barre. *Fiches* : `DC-1`.

### `UC-T1-7` — Cumulative Delta (CVD)
Cumul inter-barres, porté par l'agrégateur. *Fiches* : `DC-2`.

## Lot B — Types de périodes

### `UC-T1-8` — `TickPeriod` (n trades) — *fiche* `AGG-P4`
### `UC-T1-9` — `VolumePeriod` (n volume) — *fiche* `AGG-P5`
### `UC-T1-10` — `DollarPeriod` (n notional = Σ price·size) — *fiche* `AGG-P6`
### `UC-T1-11` — `RangePeriod` (range de prix fixe) — *fiche* `AGG-P7`
### `UC-T1-12` — `RenkoPeriod` (briques) — *fiche* `AGG-P8`
### `UC-T1-13` — `AlignedTimePeriod` (bornée sur l'horloge) — *fiche* `AGG-P2`
### `UC-T1-14` — Première barre **partielle** (période démarrée en cours) — *fiche* `AGG-B5`

## Lot C — Point d'extension complet

### `UC-T1-15` — `on_bar_update` (barre en formation)
Le subscriber est notifié à chaque trade intégré, pas seulement à la clôture. *Fiches* : `AGG-B3`, `EXT-2`.

### `UC-T1-16` — Dispatch **générique zero-cost**
Abonnement monomorphisé, sans `Box<dyn>` ni allocation dans le hot path. *Fiches* : `EXT-3`, `TR-1`, `TR-2`.

### `UC-T1-17` — Adaptateur **channel** (push, multi-consommateurs)
Émettre les barres vers un `std::sync::mpsc` / `crossbeam`. *Fiches* : `EXT-4`.

### `UC-T1-18` — Adaptateur **Iterator/Stream** (pull)
Consommer les barres en pull, composable. *Fiches* : `EXT-5`.

### `UC-T1-19` — Erreurs de mapping aux frontières
Le décodage source faillible renvoie un `Result`, hors du hot path. *Fiches* : `TR-8`.

---

## Couverture des fiches T1
Périodes `AGG-P2/P4/P5/P6/P7/P8` · `AGG-B3/B5` · order flow `OF-0/COMP`, `FP-1/2`,
`VP-1/2/3`, `DC-1/2` · extension `EXT-2/3/4/5` · transverse `TR-1/2/8` — **toutes couvertes.**

## Proposition de séquencement
Lot **A** (order flow) → Lot **B** (périodes) → Lot **C** (extension), commits verts à
chaque lot, **une seule PR T1**. (On peut aussi faire 3 PR si tu préfères des revues plus
fines.)
