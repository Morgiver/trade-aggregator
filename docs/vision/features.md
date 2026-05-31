# Features — trade-aggregator

> Regroupées **par thème** et **priorisées** — *pas* atomisées en fiches (ça vient en
> Phase 5). La priorité alimente `strategie.md`. Sources :
> [`../ideation/idea.md`](../ideation/idea.md), [`piliers.md`](piliers.md),
> [`scope.md`](scope.md).

## Légende de priorité

| | Niveau | Sens |
|---|---|---|
| **P0** | Fondation | walking skeleton — tout en dépend |
| **P1** | Cœur de valeur | la proposition de valeur principale |
| **P2** | Enrichissement | profondeur & différenciation |
| **P3** | Later | au-delà du périmètre initial |

---

## A — Entrée & modèle d'événements canonique  *(pilier P3 source-agnostic)*
- **Modèle d'événements canonique** (trade, delta de book : add/cancel/modify/fill…) — **P0**
- **Instrument definition** (tick size, price increment, lot/contract size, multiplicateur, devise) — **P0**
- **Déclaration de granularité** (L1/L2/L3) + notion de **capacités** — **P0**
- **Mapping DataBento** (`dbn`) isolé/feature-gated : trades d'abord — **P0** ; book/MBO — **P1**

## B — Orchestration : `SymbolAggregator`  *(pilier P1 dualité)*
- **Routage** event → Aggressor / Passive + **fan-out** vers N agrégations — **P0**
- **Boucle `process(event)`** unique, live = replay (event-time) — **P0**
- **Configuration des agrégations à la création** + **fail-fast** si incompatible avec la granularité — **P0**

## C — Agrégation agressive : périodes sur le tape  *(piliers P1, P2)*
- **Barres temporelles** (timeframes, aligned, sessions) + OHLCV — **P0**
- **Tick bars**, **volume bars** — **P1**
- **Dollar / notional bars** — **P1**
- **Range bars**, **Renko** — **P1** ; **Point & Figure** — **P2**
- **Imbalance bars / run bars** (López de Prado) — **P2** *(différenciation forte)*
- **Barres hybrides** (temps OU volume, premier atteint) — **P2**

## D — Order flow (sur les périodes agressives)  *(pilier P2 agréger)*
- **Footprint** (volume bid/ask par niveau de prix) — **P1**
- **Delta** + **Cumulative Delta (CVD)** — **P1**
- **POC**, **Value Area (VAH/VAL)** — **P1**
- **TPO / Market Profile** (lentille temps) — **P2**

## E — Agrégation passive : le book  *(pilier P1 dualité)*
- **Reconstruction du carnet** (book builder depuis MBO, maison, base guide DataBento) — **P1**
- **Profils de liquidité périodiques** : profil pondéré-temps, snapshots open/close — **P2**
- **Activité du book** : churn add/cancel, depth max/min, déséquilibre bid/ask moyen — **P2**

## F — Exposition / point d'extension  *(pilier P5)*
- **Canal de sortie minimal** : `on_bar_close` (callback / trait) — **P0**
- **Point d'extension complet** : push **et** pull (callbacks, channels, `Stream`) — **P1**
- **`on_bar_update`** (barre en formation, intra-barre) — **P1**
- **Alignement temporel** garanti des deux côtés (Aggressor ⟷ Passive) — **P1**
- **État interrogeable** (snapshot pull à la demande) — **P2**

## G — Performance & robustesse  *(transverse)*
- **Hot path zero-alloc**, structures pensées temps réel — **P1** (garanties de base dès le skeleton)
- **Cas limites** : events désordonnés, gaps, resynchronisation du book — **P2**

---

## Lecture rapide par priorité

- **P0 (skeleton)** : format canonique + instrument + mapping trades DataBento →
  `SymbolAggregator` → barres temporelles → sortie `on_bar_close`. *Une tranche verticale
  de bout en bout, en replay.*
- **P1 (cœur)** : autres périodes (tick/volume/dollar/range/Renko), **footprint + delta/CVD
  + POC/VA**, **reconstruction du book**, point d'extension complet, hot path soigné.
- **P2 (profondeur)** : **imbalance/run bars**, **TPO**, profils de liquidité passifs,
  robustesse cas limites.
- **P3 (later)** : variantes de profils/périodes additionnelles, ergonomie avancée.
