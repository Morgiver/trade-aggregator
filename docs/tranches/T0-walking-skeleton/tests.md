# Tests documentés — T0 Walking skeleton

> Phase 7, étape 2. **Critères d'acceptation** par use-case (cf. [`use-cases.md`](use-cases.md)).
> Niveaux : **U** = test unitaire (synthétique, en mémoire, CI) · **I** = intégration
> (fixture `.dbn` synthétique committée) · **R** = réel optionnel (gated `TRADE_AGG_DATA_DIR`).

## Mapping d'entrée
- **`UC-T0-1`** (I, feature `databento`) — un `dbn::TradeMsg` connu → `Trade` aux champs
  attendus (ts, price, size, side). Réel (R) : un fichier `*.trades.dbn.zst` se lit en une
  suite de `Trade` non vide.
- **`UC-T0-2`** (U) — un trade de côté `None`/non spécifié → `AggressorSide::Unknown` (pas de panic, pas d'erreur).

## Construction & fail-fast
- **`UC-T0-3`** (U) — `SymbolAggregator::builder(instrument, L1).with_time_period(60s).build()` → `Ok`.
- **`UC-T0-4`** (U) — enregistrer une agrégation de `min_granularity = L3` sur un agrégateur déclaré `L1` → `Err(IncompatibleGranularity { required: L3, declared: L1 })` **à la construction**.

## Agrégation temporelle
- **`UC-T0-5`** (U) — pousser 3 trades dans la même fenêtre → 1 barre en formation, `OHLCV` correct (open = 1ᵉʳ, close = dernier, high/low = extrêmes, volume = somme).
- **`UC-T0-6`** (U) — pousser un trade au-delà de la borne → la barre précédente est **fermée** (`on_bar_close`) avec son `OHLCV` figé, une nouvelle barre s'ouvre ; les bornes sont `[start, start+interval)`.
- **`UC-T0-7`** (U) — après le dernier trade, `finish()` ferme la barre en formation, marquée **partielle**, et l'émet.

## Sortie & déterminisme
- **`UC-T0-8`** (U) — un `Subscriber` enregistré reçoit, **dans l'ordre**, chaque barre fermée (mêmes barres que celles attendues).
- **`UC-T0-9`** (U + R) — rejouer **deux fois** la même séquence → **suites de barres strictement identiques** (déterminisme event-time).

## Invariants transverses (vérifiés au passage)
- Aucune horloge système utilisée (`TR-3`) : le résultat ne dépend que des `ts` d'entrée.
- Timestamps = entiers ns (`TR-4`).
- Les erreurs de construction sont des `Result` typés, pas des panics (`TR-6`).
