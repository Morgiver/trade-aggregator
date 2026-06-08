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
