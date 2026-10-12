-- v0.26 read-only preflight for changing global v1 membership to
-- per-cluster v2 membership. No data movement or destructive DDL.
-- Must be executed only against an operator-reviewed database.
BEGIN READ ONLY;
SELECT to_regclass('ocaf_pg_nodes_v1') AS legacy_global_nodes_table,
       to_regclass('ocaf_pg_tools_v1') AS legacy_global_tools_table,
       to_regclass('ocaf_pg_nodes_v2') AS cluster_scoped_nodes_table,
       to_regclass('ocaf_pg_tools_v2') AS cluster_scoped_tools_table,
       to_regclass('ocaf_pg_cursor_v2') AS cluster_scoped_cursor_table,
       to_regclass('ocaf_pg_replay_v1') AS legacy_global_replay_table,
       to_regclass('ocaf_pg_replay_v2') AS cluster_scoped_replay_table;
-- Does not query absent tables: rolling upgrade can call this safely.
-- Post-boot inspection if v2 exists:
-- SELECT cluster_id, COUNT(*) FROM ocaf_pg_nodes_v2 GROUP BY cluster_id;
-- SELECT cluster_id, COUNT(*) FROM ocaf_pg_tools_v2 GROUP BY cluster_id;
-- SELECT cluster_id, COUNT(*) FROM ocaf_pg_replay_v2 GROUP BY cluster_id;
COMMIT;
