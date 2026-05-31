# Use-cases — T3 Différenciation

> Phase 7, étape 1. S'appuie sur T0+T1+T2. Sous-découpé en 3 lots.

## Lot A — Barres information-driven (López de Prado, simplifiées à seuil fixe)

### `UC-T3-1` — `TickImbalancePeriod` (AGG-P10)
Déséquilibre signé de ticks (Buy +1, Sell −1) ; ferme quand `|cumul| ≥ seuil`.

### `UC-T3-2` — `VolumeImbalancePeriod` (AGG-P11)
Déséquilibre signé de volume (Buy +size, Sell −size) ; ferme quand `|cumul| ≥ seuil`.

### `UC-T3-3` — `DollarImbalancePeriod` (AGG-P12)
Idem sur le notional signé (`±price·size`).

### `UC-T3-4` — `RunPeriod` (AGG-P13)
Ferme quand une **série** (run) directionnelle atteint le seuil (max des runs Buy / Sell).

> Note : version **seuil fixe** (les seuils dynamiques façon EMA d'AFML = raffinement T4+).
> `Unknown` (côté agresseur absent) compte 0.

## Lot B — TPO / Market Profile (lentille order flow)

### `UC-T3-5` — Brackets (TPO-1)
Le temps de la barre est découpé en *brackets* (sous-période paramétrable).

### `UC-T3-6` — Profil temps×prix (TPO-2)
Pour chaque niveau de prix, le nombre de brackets l'ayant touché.

### `UC-T3-7` — POC / Value Area temps (TPO-3)
POC (prix le plus visité), Value Area ~70 % des TPO.

### `UC-T3-8` — Single prints (TPO-4) + Initial Balance (TPO-5)
Niveaux touchés par un seul bracket ; range des `n` premiers brackets.

> Intégrée comme `LensKind::Tpo { bracket_ns }` dans le côté agressif (s'attache à la `Bar`).

## Lot C — Finitions

### `UC-T3-9` — `PointFigurePeriod` (AGG-P9)
### `UC-T3-10` — Détection de désordre temporel (TR-5)
Un event dont le `ts` recule est signalé (compteur / drapeau), pas silencieux.

---

## Couverture
`AGG-P9/P10/P11/P12/P13` · `TPO-1…5` · `TR-5` — fiches T3.
