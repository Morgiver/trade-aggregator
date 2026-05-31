# Piliers — trade-aggregator

> Les axes structurants qui tiennent la cohérence du produit. Tout choix d'archi, de
> feature ou de scope doit pouvoir se rattacher à l'un d'eux. Sources :
> [`../ideation/idea.md`](../ideation/idea.md), [`scope.md`](scope.md).

## P1 — Dualité Aggressor / Passive

Tout, dans le domaine, est soit **consommation** de liquidité (le *tape* → agressif),
soit **fourniture** de liquidité (le *book* → passif). C'est l'ossature du modèle :
`SymbolAggregator` (racine, porte l'instrument) orchestre un `AggressorAggregator` et un
`PassiveAggregator`, chacun hébergeant **N agrégations en parallèle** (multi-charts) à la
**granularité déclarée**.
→ *Conséquence* : pas de troisième catégorie fourre-tout ; chaque donnée produite se
range d'un côté ou de l'autre.

## P2 — Agréger, pas interpréter

La crate **produit de la donnée structurée** ; elle n'en tire **aucune conclusion**
(indicateurs, signaux, absorption…). C'est le garde-fou anti-creep érigé en principe.
→ *Conséquence* : à chaque feature candidate, on demande « est-ce de l'agrégation ou de
l'interprétation ? » — la seconde est hors scope, point.

## P3 — Source-agnostic, un format canonique

Le cœur ne connaît qu'**un seul modèle d'événements normalisé**. Aucune dépendance à une
venue ; le mapping DataBento est un module **isolé** (format de référence + test), les
adapters par exchange vivent **dehors**.
→ *Conséquence* : ajouter une source ne touche jamais le cœur — on écrit un adapter
externe vers le format canonique.

## P4 — Déterminisme event-time (live = replay)

Le temps vient des **données** (event-time), pas de l'horloge. `process(event)` est le
seul chemin : **live** pousse en direct, **replay** pousse un dataset — *même code, même
résultat*.
→ *Conséquence* : tout est reproductible et testable ; le dataset DataBento est le golden
dataset.

## P5 — Extensibilité réactive

La valeur sort par un **point d'extension propre** (push/pull · `on_bar_close` /
`on_bar_update`), les deux côtés **alignés temporellement**, pour que n'importe qui
branche SES calculs. On n'impose aucun consommateur.
→ *Conséquence* : la crate est une **brique composable**, pas un monolithe ; le layer
d'indicateurs vit ailleurs et se branche ici.

---

## Transverse — Performance / low-latency

Pas un pilier isolé mais une **exigence qui traverse tous les piliers** : hot path
propre, esprit zero-alloc, structures pensées pour le temps réel. Se rattache surtout à
P1 (le cœur d'agrégation) et P5 (le point d'extension ne doit rien coûter). Sera détaillé
en `docs/architecture/transverse/` le moment venu.
