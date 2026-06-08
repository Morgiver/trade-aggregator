# T5 — Use-cases (itération consommateur)

Tranche **T5** (origine *genetic-trading*). Cible **v0.2.0**, additif & rétro-compatible.
Source de vérité co-localisée (méthodo Phase 7) : *ID · comportement · critères ·
tests qui le couvrent · issue*.

> Rappel de scope : la crate **agrège et expose**, elle **n'interprète pas**. Tous les
> use-cases ci-dessous retournent du **brut** ; normalisation/signal restent au consommateur.

---

## t5.1 — Replay fusionné event-time (issue #17)

Nœud d'archi : `aggressor/` + `passive/` via `symbol-aggregator` ; mapping `databento`.

| ID | Comportement | Critères d'acceptation | Tests |
|----|--------------|------------------------|-------|
| **UC-T5-1** | `replay_merged(trades, book, agg, limit)` fusionne deux fichiers DBN (trades + MBP-10/MBO) par `ts` croissant dans **un seul** agrégateur. | k-way merge (k=2) ; un seul `finish()` ; départage déterministe **carnet avant trade** à `ts` égal ; réutilise les mappings existants. | `databento_replay.rs::replay_merged_trades_and_mbp10_if_available` (réel, gated) |
| **UC-T5-2** | `ingest_book_snapshot(ts, book)` synchronise `agg.book()` avec le tape (flux par snapshot MBP-10). | après ingestion, `book()` = dernier snapshot ≤ ts courant ; sans côté passif → no-op. | `merged_t5.rs::ingest_book_snapshot_syncs_book`, `::ingest_snapshot_without_passive_is_noop` |
| **UC-T5-3** | L'ingestion de snapshot participe à la détection de désordre temporel (`TR-5`). | un snapshot daté avant le dernier event incrémente `out_of_order_count`. | `merged_t5.rs::ingest_snapshot_counts_in_out_of_order` |
| **UC-T5-4** | Le replay fusionné est **déterministe** et préserve l'ordre event-time. | même sortie à chaque run ; sur données triées, `out_of_order_count == 0`. | `databento_replay.rs::replay_merged_trades_and_mbp10_if_available` |

---

## t5.2 — Snapshot du carnet à la clôture de barre (issue #18)

Nœud d'archi : `extension.md` (`EXT-7`) + `symbol-aggregator`.

| ID | Comportement | Critères d'acceptation | Tests |
|----|--------------|------------------------|-------|
| **UC-T5-5** | `Subscriber::on_bar_close_with_book(period, bar, book)` reçoit le carnet échantillonné **au ts de clôture** de la barre. | book = état passif au moment du trade qui ferme la barre (dernier snapshot/MAJ ≤ ce ts) ; `None` si passif inactif. | `merged_t5.rs::book_snapshot_at_bar_close` |
| **UC-T5-6** | Rétro-compatibilité : un abonné n'implémentant que `on_bar_close` reçoit toujours ses clôtures (délégation par défaut), même passif actif. | l'abonné « legacy » compile et reçoit toutes les clôtures. | `merged_t5.rs::legacy_subscriber_still_receives_closes_with_passive` |

---

## t5.3 — Agrégations pures : TradeCount + VWAP (issue #19)

Nœud d'archi : `aggressor/orderflow`.

| ID | Comportement | Critères d'acceptation | Tests |
|----|--------------|------------------------|-------|
| **UC-T5-7** | Lentille `TradeCount` : `(buy_count, sell_count)` par barre (`Unknown` ignoré, comme `Delta`). | sommes correctes ; `Unknown` non compté ; activable via `LensKind::TradeCount` → `OrderFlow.trade_count`. | `orderflow.rs::trade_count_buy_sell_unknown`, `order_flow_wiring.rs::trade_count_and_vwap_lenses_attached` |
| **UC-T5-8** | Lentille `Vwap` : `Σ(price·size) / Σ size`, **tous trades** (côté-agnostique). | valeur testée à la main ; ∈ `[low, high]` ; `None` si volume nul ; via `LensKind::Vwap` → `OrderFlow.vwap`. | `orderflow.rs::vwap_value_all_trades`, `::vwap_empty_is_none`, `order_flow_wiring.rs::trade_count_and_vwap_lenses_attached` |

---

## t5.4 — Vue footprint à largeur fixe (issue #20)

Nœud d'archi : `aggressor/orderflow`.

| ID | Comportement | Critères d'acceptation | Tests |
|----|--------------|------------------------|-------|
| **UC-T5-9** | `Footprint::window(anchor, tick_size, half_width)` matérialise une fenêtre fixe indexée par offset de tick. | longueur = `2*half_width + 1` ; ancre à l'indice `half_width` ; cellules absentes = `(0,0)` ; débordements gérés ; aucune perte si `half_width` couvre une période bornée. | `orderflow.rs::footprint_window_fixed_width_and_offset`, `::footprint_window_half_width_zero_is_single_cell` |

---

## t5.5 — Helper DX `replay_to_bars` (issue #22)

Nœud d'archi : `databento` (mapping) + `extension`.

| ID | Comportement | Critères d'acceptation | Tests |
|----|--------------|------------------------|-------|
| **UC-T5-10** | `replay_to_bars(path, agg, limit)` rejoue un fichier de trades dans un agrégateur déjà configuré et renvoie `Vec<Bar>` (ordre event-time, `finish()` inclus). | équivaut au câblage `ChannelSink` manuel ; derrière la feature `databento`. | `databento_replay.rs::replay_to_bars_matches_manual_wiring_if_available` (réel, gated) |

---

## t5.6 — Renko « corps + mèche bornée » sur grille (issue #21)

Nœud d'archi : `aggressor/` (`AGG-P8` raffiné). `RenkoPeriod` (simplifiée) **conservé**
(rétro-compat) ; `RenkoBrickPeriod` ajouté.

| ID | Comportement | Critères d'acceptation | Tests |
|----|--------------|------------------------|-------|
| **UC-T5-11** | `RenkoBrickPeriod` : briques alignées sur une **grille** (référence = multiple de `brick`), sauts **multi-briques** re-snappés sur grille, **borne d'excursion** explicite `2·brick−1`. | référence snappée déterministe ; saut multi-briques correct (gap) ; `excursion_bound()` testé → largeur footprint fixe garantie ; rétro-compat (`RenkoPeriod` conservé). | `period.rs::renko_brick_grid_aligned_and_multibrick` |
