# Tests documentés — T1 Cœur agressif

> Phase 7, étape 2. Critères d'acceptation par use-case. **U** = unitaire, **I** = intégration.
> Rédigés lot par lot (A d'abord).

## Lot A — Order flow

- **`UC-T1-1`** (U) — `BarComponent` : un type implémentant `on_trade`/`on_close` se compile et s'utilise via le trait.
- **`UC-T1-3`** Footprint (U) — après 3 trades (Buy@100×2, Sell@100×5, Buy@101×1), les cellules valent `{100:(2,5), 101:(1,0)}`. Un côté `Unknown` n'est attribué ni Buy ni Sell.
- **`UC-T1-4`** VolumeProfile + POC (U) — volumes par prix corrects ; `poc()` = prix de volume max (ex. 100 si 100 a le plus de volume).
- **`UC-T1-5`** Value Area (U) — sur un profil connu, la value area (~70 %) couvre les niveaux attendus autour du POC ; le seuil est paramétrable.
- **`UC-T1-6`** Delta (U) — `Σ Buy − Σ Sell` (ex. Buy 3 + Sell 5 → −2) ; `Unknown` compte 0.
- **`UC-T1-7`** CVD (U) — cumul des deltas de barres successives (ex. −2 puis +5 → −2 puis 3).

## Lot B — Périodes

- **`UC-T1-13`** AlignedTime (U) — fenêtres alignées sur l'horloge ; première barre **partielle** si démarrage en cours de fenêtre (`AGG-B5`).
- **`UC-T1-8`** Tick (U) — `n` trades par barre (ferme à l'arrivée du (n+1)ᵉ).
- **`UC-T1-9`** Volume (U) — ferme dès volume cumulé ≥ seuil (trade de franchissement inclus).
- **`UC-T1-10`** Dollar (U) — idem sur `Σ price·size`.
- **`UC-T1-11`** Range (U) — amplitude `high−low` ≤ range ; le trade qui dépasse ouvre une barre.
- **`UC-T1-12`** Renko (U) — nouvelle brique quand `|price − open| ≥ brick`.

## Lot C — Extension

- **`UC-T1-15`** on_bar_update (U) — un `on_bar_update` est émis à **chaque trade** intégré dans la barre en formation (méthode du trait `Subscriber`, défaut vide → rétro-compatible). `EXT-2`/`AGG-B3`.
- **`UC-T1-17`** channel (U) — `ChannelSink` pousse les barres fermées dans un `mpsc::Sender`. `EXT-4`.
- **`UC-T1-18`** pull (U) — le `Receiver` du channel fournit la consommation pull (itérateur `into_iter`). `EXT-5`.
- **`UC-T1-1`/closure** (U) — `FnSubscriber(|p,b| …)`.

> **`UC-T1-16` (EXT-3 / TR-2 — dispatch monomorphisé « zéro dyn dans le hot path »)
> reporté en T4** (perf/benchmarks) : c'est une optimisation transverse qui nécessite de
> généraliser `Period`/`Subscriber` ; à faire avec des benchmarks, pas à l'aveugle. Le
> dispatch `dyn` actuel est correct et sans allocation par trade (`TR-1` tenu).
