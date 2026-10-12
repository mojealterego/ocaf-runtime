-- READ ONLY. Run against a VERIFIED, RESTORABLE backup target first.
-- Verify legacy job ownership independently before any v0.23+ migration.
-- This report deliberately makes NO cluster assignment automatically.
BEGIN READ ONLY;
SELECT cluster_id, status, count(*)::BIGINT AS rows
  FROM ocaf_pg_jobs_v1
 GROUP BY cluster_id, status
 ORDER BY cluster_id, status;
SELECT count(*)::BIGINT AS legacy_unscoped_jobs
  FROM ocaf_pg_jobs_v1 WHERE cluster_id='';
SELECT count(*)::BIGINT AS unconfirmed_running_leases
  FROM ocaf_pg_jobs_v1
 WHERE status='running' AND (leader_epoch <= 0 OR cluster_id='');
SELECT count(*)::BIGINT AS uncertain_jobs_needing_reconciliation
  FROM ocaf_pg_jobs_v1 WHERE status='uncertain';
COMMIT;
