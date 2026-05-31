# Produit — trade-aggregator

> Ce qu'on construit, pour qui, et la valeur qu'on apporte.
> Sources : [`../ideation/idea.md`](../ideation/idea.md).

## Ce que c'est

Une **crate Rust** (librairie, pas application) qui transforme un flux de données de
marché brutes (tape + book) en **données agrégées riches** — order flow agressif et
profils de liquidité passifs — en **temps réel** et en **replay**, sous un même modèle
déterministe. **Source-agnostic** : un seul format d'entrée canonique.

## Proposition de valeur

1. **L'agrégation order-flow qu'on ne veut pas réécrire.** Footprint, delta/CVD, TPO et
   profils de liquidité (avec reconstruction du carnet) sous un modèle unique
   `Aggressor` / `Passive` — au lieu de la bricoler à la main pour chaque projet.
2. **Des types de barres absents de l'écosystème Rust** : dollar/notional bars,
   **imbalance & run bars** (López de Prado) — aucune implémentation Rust connue.
3. **Déterministe et testable** : event-time, *live = replay* sur la même API → on
   rejoue un dataset et on obtient exactement le même résultat.
4. **Ne vous enferme pas** : aucun indicateur imposé. Un **point d'extension réactif**
   (zero-cost) pour brancher VOS calculs temps réel sur les données agrégées.
5. **Pensé low-latency** : hot path propre, esprit zero-alloc.

## Public visé

- **Devs quant / traders algo en Rust** qui construisent leurs propres outils et veulent
  une couche d'agrégation order-flow fiable et rapide, sans la réimplémenter.
- **Chercheurs en ML financier** voulant des imbalance/run bars performantes en Rust.
- **Builders d'outils de visualisation** (footprint, TPO/Market Profile, heatmaps de
  liquidité) qui ont besoin de la donnée agrégée, pas du rendu.
- **Nous-mêmes** : brique fondation pour de futurs projets (dont un éventuel layer
  d'indicateurs, hors de cette crate).

## Ce que ce n'est pas (rappel, détail dans `scope.md`)

Pas une plateforme, pas une GUI, pas un moteur d'indicateurs/signaux, pas un connecteur
d'exchange, pas un système de backtest. **Une brique, qui fait une chose : agréger.**
