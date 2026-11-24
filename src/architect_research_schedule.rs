//! Durable, tenant-scoped research schedule for OCAF Meta Agent Architekt.
//!
//! AB-OM-10 (partial): three UTC periods, SQLite deduplication, bounded
//! leases, monotonic fencing, receipt replay and explicit timeout reconciliation.
//! This module does NOT perform research, approve releases or run autonomously.
//! Tenant/worker/verifier identity and UTC time must be trusted by the host.

use crate::omnidev_workflow::Role;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex, time::Duration};
use thiserror::Error;

const MAX_ATTEMPTS: i64 = 3;
const MAX_LEASE_SECONDS: i64 = 3600;
const MAX_UNIX: i64 = 253_402_300_799;
const DAY: i64 = 86_400;
const MAX_CATCH_UP_DAYS: i64 = 90;
const MAX_DUE_QUERY: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResearchKind {
    Competitive,
    Market,
    SystemEvolution,
}

impl ResearchKind {
    fn decode(raw: &str) -> Result<Self, ResearchError> {
        match raw {
            "COMPETITIVE_DEEP_RESEARCH" => Ok(Self::Competitive),
            "MARKET_OPPORTUNITY_DEEP_RESEARCH" => Ok(Self::Market),
            "ULTRA_DEEP_RESEARCH_SYSTEM_EVOLUTION" => Ok(Self::SystemEvolution),
            _ => Err(ResearchError::Integrity),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Competitive => "COMPETITIVE_DEEP_RESEARCH",
            Self::Market => "MARKET_OPPORTUNITY_DEEP_RESEARCH",
            Self::SystemEvolution => "ULTRA_DEEP_RESEARCH_SYSTEM_EVOLUTION",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunState {
    Ready,
    Leased,
    Succeeded,
    Halted,
}

impl RunState {
    fn decode(raw: &str) -> Result<Self, ResearchError> {
        match raw {
            "READY" => Ok(Self::Ready),
            "LEASED" => Ok(Self::Leased),
            "SUCCEEDED" => Ok(Self::Succeeded),
            "HALTED" => Ok(Self::Halted),
            _ => Err(ResearchError::Integrity),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::Leased => "LEASED",
            Self::Succeeded => "SUCCEEDED",
            Self::Halted => "HALTED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchPeriod {
    pub kind: ResearchKind,
    pub start_unix: i64,
    pub end_unix: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatchUpReport {
    pub from_unix: i64,
    pub through_unix: i64,
    pub new_runs: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchLease {
    pub kind: ResearchKind,
    pub period_start_unix: i64,
    pub epoch: i64,
    pub attempt: i64,
    pub lease_until_unix: i64,
    pub worker_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchSnapshot {
    pub state: RunState,
    pub epoch: i64,
    pub attempts: i64,
    pub lease_until_unix: Option<i64>,
    pub last_evidence_sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recorded {
    New,
    Replay,
}

#[derive(Debug, Error)]
pub enum ResearchError {
    #[error("invalid research scheduler input")]
    Invalid,
    #[error("research run not found for tenant and fleet")]
    NotFound,
    #[error("only a trusted verifier may reconcile expired leases")]
    Denied,
    #[error("research run not due")]
    NotDue,
    #[error("research run leased or terminal")]
    Busy,
    #[error("lease expired: trusted verifier reconciliation required")]
    Uncertain,
    #[error("epoch or worker has been fenced")]
    Fenced,
    #[error("attempt budget exhausted")]
    Exhausted,
    #[error("stored state is invalid")]
    Integrity,
    #[error("scheduler capacity exhausted")]
    Capacity,
    #[error("scheduler lock poisoned")]
    Poisoned,
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
}

pub struct ArchitectResearchScheduler {
    db: Mutex<Connection>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn valid_time(t: i64) -> bool {
    (0..=MAX_UNIX).contains(&t)
}

/// Query calendar-month boundaries using SQLite's UTC Gregorian calendar.
/// The repository does not need system-local timezone configuration for UTC.
fn month_bounds(tx: &rusqlite::Transaction<'_>, unix: i64) -> Result<(i64, i64), ResearchError> {
    let period: (Option<i64>, Option<i64>) = tx.query_row(
        "SELECT CAST(strftime('%s',?1,'unixepoch','start of month') AS INTEGER),
                CAST(strftime('%s',?1,'unixepoch','start of month','+1 month') AS INTEGER)",
        params![unix],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    match period {
        (Some(start), Some(end)) if start >= 0 && end > start => Ok((start, end)),
        _ => Err(ResearchError::Invalid),
    }
}

impl ArchitectResearchScheduler {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ResearchError> {
        if path
            .as_ref()
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(ResearchError::Invalid);
        }
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS architect_research_runs (
                 tenant TEXT NOT NULL, fleet TEXT NOT NULL,
                 kind TEXT NOT NULL CHECK(kind IN (
                   'COMPETITIVE_DEEP_RESEARCH',
                   'MARKET_OPPORTUNITY_DEEP_RESEARCH',
                   'ULTRA_DEEP_RESEARCH_SYSTEM_EVOLUTION')),
                 period_start INTEGER NOT NULL, period_end INTEGER NOT NULL,
                 state TEXT NOT NULL CHECK(state IN
                   ('READY','LEASED','SUCCEEDED','HALTED')),
                 epoch INTEGER NOT NULL DEFAULT 0,
                 attempts INTEGER NOT NULL DEFAULT 0,
                 worker TEXT, lease_until INTEGER,
                 last_receipt_epoch INTEGER,
                 last_receipt_worker TEXT,
                 last_receipt_pass INTEGER,
                 last_evidence_sha256 TEXT,
                 reconciliation_evidence_sha256 TEXT,
                 PRIMARY KEY(tenant,fleet,kind,period_start)
             );
             CREATE INDEX IF NOT EXISTS architect_research_due
                 ON architect_research_runs(tenant,fleet,state,period_start);
             CREATE TABLE IF NOT EXISTS architect_research_scan_state (
                 tenant TEXT NOT NULL, fleet TEXT NOT NULL,
                 last_scan_unix INTEGER NOT NULL,
                 PRIMARY KEY(tenant,fleet)
             );",
        )?;
        Ok(Self { db: Mutex::new(db) })
    }

    /// Idempotent, transactional scheduler tick with an SQLite-persisted
    /// high-water mark. On first use only the current periods are registered.
    /// Subsequent invocations backfill each missed UTC week and month.
    /// Outages beyond 90 days fail atomically to require operator intervention.
    /// Calls never perform research, dispatch a worker or approve an evolution.
    pub fn tick(
        &self,
        tenant: &str,
        fleet: &str,
        now_unix: i64,
    ) -> Result<CatchUpReport, ResearchError> {
        if !valid_id(tenant) || !valid_id(fleet) || !valid_time(now_unix) {
            return Err(ResearchError::Invalid);
        }
        let mut db = self.db.lock().map_err(|_| ResearchError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous: Option<i64> = tx
            .query_row(
                "SELECT last_scan_unix FROM architect_research_scan_state
             WHERE tenant=?1 AND fleet=?2",
                params![tenant, fleet],
                |r| r.get(0),
            )
            .optional()?;
        let from = previous.unwrap_or(now_unix);
        if !valid_time(from) {
            return Err(ResearchError::Integrity);
        }
        if from > now_unix {
            return Err(ResearchError::Invalid);
        }
        if now_unix - from > MAX_CATCH_UP_DAYS * DAY {
            return Err(ResearchError::Capacity);
        }
        // 1970-01-01 is Thursday: Monday-based ISO weeks (UTC).
        let mut week = (from / DAY - (from / DAY + 3) % 7) * DAY;
        let last_week = (now_unix / DAY - (now_unix / DAY + 3) % 7) * DAY;
        if week < 0 {
            return Err(ResearchError::Invalid);
        }
        let mut new_runs = 0_usize;
        while week <= last_week {
            let end = week.checked_add(7 * DAY).ok_or(ResearchError::Invalid)?;
            for kind in [ResearchKind::Competitive, ResearchKind::Market] {
                new_runs += tx.execute(
                    "INSERT OR IGNORE INTO architect_research_runs
                     (tenant,fleet,kind,period_start,period_end,state)
                     VALUES (?1,?2,?3,?4,?5,'READY')",
                    params![tenant, fleet, kind.key(), week, end],
                )?;
            }
            week = end;
        }
        let (mut month, _) = month_bounds(&tx, from)?;
        loop {
            let (_, month_end) = month_bounds(&tx, month)?;
            if month > now_unix {
                break;
            }
            new_runs += tx.execute(
                "INSERT OR IGNORE INTO architect_research_runs
                 (tenant,fleet,kind,period_start,period_end,state)
                 VALUES (?1,?2,?3,?4,?5,'READY')",
                params![
                    tenant,
                    fleet,
                    ResearchKind::SystemEvolution.key(),
                    month,
                    month_end
                ],
            )?;
            if month_end > now_unix {
                break;
            }
            month = month_end;
        }
        tx.execute(
            "INSERT INTO architect_research_scan_state (tenant,fleet,last_scan_unix)
             VALUES (?1,?2,?3)
             ON CONFLICT(tenant,fleet) DO UPDATE
             SET last_scan_unix=excluded.last_scan_unix",
            params![tenant, fleet, now_unix],
        )?;
        tx.commit()?;
        Ok(CatchUpReport {
            from_unix: from,
            through_unix: now_unix,
            new_runs,
        })
    }

    /// Read-only bounded backlog: exclusively READY periods whose start has
    /// passed. This provides deterministic ordering for a trusted dispatcher.
    pub fn list_due(
        &self,
        tenant: &str,
        fleet: &str,
        now_unix: i64,
        limit: usize,
    ) -> Result<Vec<ResearchPeriod>, ResearchError> {
        if !valid_id(tenant)
            || !valid_id(fleet)
            || !valid_time(now_unix)
            || !(1..=MAX_DUE_QUERY).contains(&limit)
        {
            return Err(ResearchError::Invalid);
        }
        let db = self.db.lock().map_err(|_| ResearchError::Poisoned)?;
        let mut statement = db.prepare(
            "SELECT kind,period_start,period_end FROM architect_research_runs
             WHERE tenant=?1 AND fleet=?2 AND state='READY' AND period_start<=?3
             ORDER BY period_start ASC, kind ASC LIMIT ?4",
        )?;
        let rows = statement.query_map(params![tenant, fleet, now_unix, limit as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (raw, start, end) = row?;
            let kind = ResearchKind::decode(&raw)?;
            if !valid_time(start) || end <= start {
                return Err(ResearchError::Integrity);
            }
            result.push(ResearchPeriod {
                kind,
                start_unix: start,
                end_unix: end,
            });
        }
        Ok(result)
    }

    /// Registers two weekly periods and one calendar-month period in UTC.
    /// Calling twice in the same period never resets a lease or receipt.
    /// The host must call this on a real timer; offline catch-up is not yet provided.
    pub fn enqueue_current(
        &self,
        tenant: &str,
        fleet: &str,
        now_unix: i64,
    ) -> Result<[ResearchPeriod; 3], ResearchError> {
        if !valid_id(tenant) || !valid_id(fleet) || !valid_time(now_unix) {
            return Err(ResearchError::Invalid);
        }
        // 1970-01-01 was Thursday: 3 days after Monday.
        let day = now_unix / DAY;
        let week_start = (day - (day + 3) % 7) * DAY;
        let week_end = week_start + 7 * DAY;

        let mut db = self.db.lock().map_err(|_| ResearchError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let month: (Option<i64>, Option<i64>) = tx.query_row(
            "SELECT CAST(strftime('%s',?1,'unixepoch','start of month') AS INTEGER),
                    CAST(strftime('%s',?1,'unixepoch','start of month','+1 month') AS INTEGER)",
            params![now_unix],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let (month_start, month_end) = match month {
            (Some(start), Some(end)) if start >= 0 && end > start => (start, end),
            _ => return Err(ResearchError::Invalid),
        };
        let periods = [
            ResearchPeriod {
                kind: ResearchKind::Competitive,
                start_unix: week_start,
                end_unix: week_end,
            },
            ResearchPeriod {
                kind: ResearchKind::Market,
                start_unix: week_start,
                end_unix: week_end,
            },
            ResearchPeriod {
                kind: ResearchKind::SystemEvolution,
                start_unix: month_start,
                end_unix: month_end,
            },
        ];
        for period in &periods {
            tx.execute(
                "INSERT OR IGNORE INTO architect_research_runs
                 (tenant,fleet,kind,period_start,period_end,state)
                 VALUES (?1,?2,?3,?4,?5,'READY')",
                params![
                    tenant,
                    fleet,
                    period.kind.key(),
                    period.start_unix,
                    period.end_unix
                ],
            )?;
        }
        tx.commit()?;
        Ok(periods)
    }

    /// Transactionally claims a due job. An expired lease never auto-retries:
    /// external effects may have succeeded before the worker was lost.
    #[allow(clippy::too_many_arguments)]
    pub fn claim(
        &self,
        tenant: &str,
        fleet: &str,
        kind: ResearchKind,
        period_start_unix: i64,
        worker_id: &str,
        now_unix: i64,
        lease_seconds: i64,
    ) -> Result<ResearchLease, ResearchError> {
        if !valid_id(tenant)
            || !valid_id(fleet)
            || !valid_id(worker_id)
            || !valid_time(period_start_unix)
            || !valid_time(now_unix)
            || !(1..=MAX_LEASE_SECONDS).contains(&lease_seconds)
        {
            return Err(ResearchError::Invalid);
        }
        if now_unix < period_start_unix {
            return Err(ResearchError::NotDue);
        }
        let lease_until = now_unix
            .checked_add(lease_seconds)
            .ok_or(ResearchError::Invalid)?;
        let mut db = self.db.lock().map_err(|_| ResearchError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row: Option<(String, i64, i64, Option<i64>)> = tx
            .query_row(
                "SELECT state,epoch,attempts,lease_until
             FROM architect_research_runs
             WHERE tenant=?1 AND fleet=?2 AND kind=?3 AND period_start=?4",
                params![tenant, fleet, kind.key(), period_start_unix],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let (state, epoch, attempts, previous_until) = row.ok_or(ResearchError::NotFound)?;
        match RunState::decode(&state)? {
            RunState::Ready => {}
            RunState::Leased => {
                return Err(if previous_until.is_some_and(|until| now_unix > until) {
                    ResearchError::Uncertain
                } else {
                    ResearchError::Busy
                })
            }
            _ => return Err(ResearchError::Busy),
        }
        if epoch < 0 || attempts < 0 {
            return Err(ResearchError::Integrity);
        }
        if attempts >= MAX_ATTEMPTS {
            return Err(ResearchError::Exhausted);
        }
        let next_epoch = epoch.checked_add(1).ok_or(ResearchError::Capacity)?;
        tx.execute(
            "UPDATE architect_research_runs SET state='LEASED',
             epoch=?5, attempts=attempts+1, worker=?6, lease_until=?7
             WHERE tenant=?1 AND fleet=?2 AND kind=?3 AND period_start=?4",
            params![
                tenant,
                fleet,
                kind.key(),
                period_start_unix,
                next_epoch,
                worker_id,
                lease_until
            ],
        )?;
        tx.commit()?;
        Ok(ResearchLease {
            kind,
            period_start_unix,
            epoch: next_epoch,
            attempt: attempts + 1,
            lease_until_unix: lease_until,
            worker_id: worker_id.to_owned(),
        })
    }

    /// Fenced result receipt, not a deployment/release authorization.
    pub fn complete(
        &self,
        tenant: &str,
        fleet: &str,
        lease: &ResearchLease,
        passed: bool,
        evidence_sha256: &str,
        now_unix: i64,
    ) -> Result<Recorded, ResearchError> {
        if !valid_id(tenant)
            || !valid_id(fleet)
            || !valid_id(&lease.worker_id)
            || !valid_time(lease.period_start_unix)
            || !valid_time(now_unix)
            || !valid_digest(evidence_sha256)
        {
            return Err(ResearchError::Invalid);
        }
        let mut db = self.db.lock().map_err(|_| ResearchError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        struct StoredReceipt {
            state: String,
            epoch: i64,
            attempts: i64,
            until: Option<i64>,
            worker: Option<String>,
            last_epoch: Option<i64>,
            last_worker: Option<String>,
            last_pass: Option<i64>,
            last_digest: Option<String>,
        }
        let row: Option<StoredReceipt> = tx
            .query_row(
                "SELECT state,epoch,attempts,lease_until,worker,
                    last_receipt_epoch,last_receipt_worker,last_receipt_pass,last_evidence_sha256
                 FROM architect_research_runs
                 WHERE tenant=?1 AND fleet=?2 AND kind=?3 AND period_start=?4",
                params![tenant, fleet, lease.kind.key(), lease.period_start_unix],
                |r| {
                    Ok(StoredReceipt {
                        state: r.get(0)?,
                        epoch: r.get(1)?,
                        attempts: r.get(2)?,
                        until: r.get(3)?,
                        worker: r.get(4)?,
                        last_epoch: r.get(5)?,
                        last_worker: r.get(6)?,
                        last_pass: r.get(7)?,
                        last_digest: r.get(8)?,
                    })
                },
            )
            .optional()?;
        let StoredReceipt {
            state,
            epoch,
            attempts,
            until,
            worker,
            last_epoch,
            last_worker,
            last_pass,
            last_digest,
        } = row.ok_or(ResearchError::NotFound)?;
        if last_epoch == Some(lease.epoch)
            && last_worker.as_deref() == Some(&lease.worker_id)
            && last_pass == Some(i64::from(passed))
            && last_digest.as_deref() == Some(evidence_sha256)
        {
            return Ok(Recorded::Replay);
        }
        if RunState::decode(&state)? != RunState::Leased
            || epoch != lease.epoch
            || worker.as_deref() != Some(&lease.worker_id)
            || until != Some(lease.lease_until_unix)
        {
            return Err(ResearchError::Fenced);
        }
        if now_unix > lease.lease_until_unix {
            return Err(ResearchError::Uncertain);
        }
        if !(1..=MAX_ATTEMPTS).contains(&attempts) {
            return Err(ResearchError::Integrity);
        }
        let next = if passed {
            RunState::Succeeded
        } else if attempts >= MAX_ATTEMPTS {
            RunState::Halted
        } else {
            RunState::Ready
        };
        tx.execute(
            "UPDATE architect_research_runs SET state=?5,
                worker=NULL,lease_until=NULL,last_receipt_epoch=?6,
                last_receipt_worker=?7,last_receipt_pass=?8,last_evidence_sha256=?9
             WHERE tenant=?1 AND fleet=?2 AND kind=?3 AND period_start=?4",
            params![
                tenant,
                fleet,
                lease.kind.key(),
                lease.period_start_unix,
                next.key(),
                lease.epoch,
                lease.worker_id,
                i64::from(passed),
                evidence_sha256
            ],
        )?;
        tx.commit()?;
        Ok(Recorded::New)
    }

    /// ONLY trusted Verifier code may call this after independent effect
    /// reconciliation. The caller-provided boolean/digest is not an attestation.
    #[allow(clippy::too_many_arguments)]
    pub fn reconcile_expired(
        &self,
        trusted_role: Role,
        tenant: &str,
        fleet: &str,
        kind: ResearchKind,
        period_start_unix: i64,
        epoch: i64,
        no_effect_confirmed: bool,
        verifier_evidence_sha256: &str,
        now_unix: i64,
    ) -> Result<(), ResearchError> {
        if trusted_role != Role::Verifier {
            return Err(ResearchError::Denied);
        }
        if !valid_id(tenant)
            || !valid_id(fleet)
            || !valid_time(period_start_unix)
            || !valid_time(now_unix)
            || epoch <= 0
            || !valid_digest(verifier_evidence_sha256)
        {
            return Err(ResearchError::Invalid);
        }
        let mut db = self.db.lock().map_err(|_| ResearchError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let row: Option<(String, i64, i64, Option<i64>)> = tx
            .query_row(
                "SELECT state,epoch,attempts,lease_until FROM architect_research_runs
             WHERE tenant=?1 AND fleet=?2 AND kind=?3 AND period_start=?4",
                params![tenant, fleet, kind.key(), period_start_unix],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let (state, stored_epoch, attempts, until) = row.ok_or(ResearchError::NotFound)?;
        if RunState::decode(&state)? != RunState::Leased || stored_epoch != epoch {
            return Err(ResearchError::Fenced);
        }
        if !until.is_some_and(|until| now_unix > until) {
            return Err(ResearchError::Busy);
        }
        if !(1..=MAX_ATTEMPTS).contains(&attempts) {
            return Err(ResearchError::Integrity);
        }
        let next = if no_effect_confirmed && attempts < MAX_ATTEMPTS {
            RunState::Ready
        } else {
            RunState::Halted
        };
        tx.execute(
            "UPDATE architect_research_runs SET state=?5,worker=NULL,
                lease_until=NULL,reconciliation_evidence_sha256=?6
             WHERE tenant=?1 AND fleet=?2 AND kind=?3 AND period_start=?4",
            params![
                tenant,
                fleet,
                kind.key(),
                period_start_unix,
                next.key(),
                verifier_evidence_sha256
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Seal a period only after the internal signed-evidence finalizer has
    /// verified all three OM Engine receipts against separately pinned signer
    /// keys. This method is deliberately crate-private, not an externally
    /// exposed "mark successful" API.
    pub(crate) fn seal_verified(
        &self,
        tenant: &str,
        fleet: &str,
        kind: ResearchKind,
        period_start_unix: i64,
        evidence_sha256: &str,
        now_unix: i64,
    ) -> Result<Recorded, ResearchError> {
        if !valid_id(tenant)
            || !valid_id(fleet)
            || !valid_time(period_start_unix)
            || !valid_time(now_unix)
            || !valid_digest(evidence_sha256)
        {
            return Err(ResearchError::Invalid);
        }
        if now_unix < period_start_unix {
            return Err(ResearchError::NotDue);
        }
        let mut db = self.db.lock().map_err(|_| ResearchError::Poisoned)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: Option<(String, Option<String>)> = tx
            .query_row(
                "SELECT state,last_evidence_sha256
             FROM architect_research_runs
             WHERE tenant=?1 AND fleet=?2 AND kind=?3 AND period_start=?4",
                params![tenant, fleet, kind.key(), period_start_unix],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (state, previous_evidence) = current.ok_or(ResearchError::NotFound)?;
        let result = match RunState::decode(&state)? {
            RunState::Succeeded if previous_evidence.as_deref() == Some(evidence_sha256) => {
                Recorded::Replay
            }
            RunState::Succeeded => return Err(ResearchError::Integrity),
            RunState::Ready => {
                tx.execute(
                    "UPDATE architect_research_runs
                     SET state='SUCCEEDED',last_evidence_sha256=?5
                     WHERE tenant=?1 AND fleet=?2 AND kind=?3 AND period_start=?4 AND state='READY'",
                    params![tenant,fleet,kind.key(),period_start_unix,evidence_sha256],
                )?;
                Recorded::New
            }
            RunState::Leased | RunState::Halted => return Err(ResearchError::Busy),
        };
        tx.commit()?;
        Ok(result)
    }

    /// Current persisted state, also suitable for read-only offline UI.
    pub fn snapshot(
        &self,
        tenant: &str,
        fleet: &str,
        kind: ResearchKind,
        period_start_unix: i64,
    ) -> Result<ResearchSnapshot, ResearchError> {
        if !valid_id(tenant) || !valid_id(fleet) || !valid_time(period_start_unix) {
            return Err(ResearchError::Invalid);
        }
        let db = self.db.lock().map_err(|_| ResearchError::Poisoned)?;
        struct StoredSnapshot {
            state: String,
            epoch: i64,
            attempts: i64,
            lease_until: Option<i64>,
            evidence: Option<String>,
        }
        let row: Option<StoredSnapshot> = db
            .query_row(
                "SELECT state,epoch,attempts,lease_until,last_evidence_sha256
             FROM architect_research_runs
             WHERE tenant=?1 AND fleet=?2 AND kind=?3 AND period_start=?4",
                params![tenant, fleet, kind.key(), period_start_unix],
                |r| {
                    Ok(StoredSnapshot {
                        state: r.get(0)?,
                        epoch: r.get(1)?,
                        attempts: r.get(2)?,
                        lease_until: r.get(3)?,
                        evidence: r.get(4)?,
                    })
                },
            )
            .optional()?;
        let StoredSnapshot {
            state,
            epoch,
            attempts,
            lease_until,
            evidence,
        } = row.ok_or(ResearchError::NotFound)?;
        if epoch < 0 || !(0..=MAX_ATTEMPTS).contains(&attempts) {
            return Err(ResearchError::Integrity);
        }
        Ok(ResearchSnapshot {
            state: RunState::decode(&state)?,
            epoch,
            attempts,
            lease_until_unix: lease_until,
            last_evidence_sha256: evidence,
        })
    }
}
