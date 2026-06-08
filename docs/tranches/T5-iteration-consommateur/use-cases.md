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
