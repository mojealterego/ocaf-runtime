//! Durable, tenant-scoped FinOps reservations for stage 0.65.
//!
//! This is a ledger primitive, not a payment processor or an authorization
//! endpoint. Budget administration must be guarded by the existing OCAF
//! capability/identity policy before callers reach configure_budget.

use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FinOpsError {
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(&'static str),
    #[error("amount must be positive and fit SQLite signed integer range")]
    InvalidAmount,
    #[error("no configured budget for tenant and agent")]
    BudgetNotConfigured,
    #[error("budget exhausted")]
    BudgetExceeded,
    #[error("cannot reduce budget below already reserved and committed spend")]
    BudgetBelowUsage,
    #[error("idempotency key already used with different amount or released")]
    IdempotencyConflict,
    #[error("reservation not found")]
    ReservationNotFound,
    #[error("invalid reservation state transition")]
    InvalidTransition,
    #[error("arithmetic overflow")]
    Overflow,
    #[error("FinOps ledger mutex poisoned")]
    LockPoisoned,
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReservationState {
    Reserved,
    Committed,
    Released,
}

impl ReservationState {
    fn from_db(value: &str) -> Result<Self, FinOpsError> {
        match value {
            "RESERVED" => Ok(Self::Reserved),
            "COMMITTED" => Ok(Self::Committed),
            "RELEASED" => Ok(Self::Released),
            _ => Err(FinOpsError::InvalidTransition),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetSnapshot {
    pub limit_minor: u64,
    pub reserved_minor: u64,
    pub committed_minor: u64,
    pub available_minor: u64,
}

/// SQLite IMMEDIATE transactions serialize reservations across processes.
/// The same request ID is never reused after release; this avoids replay
/// resurrecting a reservation after a payment has been cancelled.
pub struct FinOpsLedger {
    conn: Mutex<Connection>,
}

impl FinOpsLedger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, FinOpsError> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS finops_budget (
                 tenant_id TEXT NOT NULL,
                 agent_id TEXT NOT NULL,
                 limit_minor INTEGER NOT NULL CHECK(limit_minor >= 0),
                 PRIMARY KEY(tenant_id, agent_id)
             );
             CREATE TABLE IF NOT EXISTS finops_reservation (
                 tenant_id TEXT NOT NULL,
                 agent_id TEXT NOT NULL,
                 request_id TEXT NOT NULL,
                 amount_minor INTEGER NOT NULL CHECK(amount_minor > 0),
                 state TEXT NOT NULL CHECK(state IN ('RESERVED','COMMITTED','RELEASED')),
                 PRIMARY KEY(tenant_id, agent_id, request_id),
                 FOREIGN KEY(tenant_id, agent_id)
                   REFERENCES finops_budget(tenant_id, agent_id)
             );
             CREATE INDEX IF NOT EXISTS finops_active_usage
               ON finops_reservation(tenant_id, agent_id, state);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Administrative operation. The caller MUST verify operator identity and
    /// capability through OCAF before invoking this method.
    pub fn configure_budget(
        &self,
        tenant_id: &str,
        agent_id: &str,
        limit_minor: u64,
    ) -> Result<BudgetSnapshot, FinOpsError> {
        validate_id(tenant_id, "tenant_id")?;
        validate_id(agent_id, "agent_id")?;
        let limit = signed_amount(limit_minor)?;
        let mut conn = self.conn.lock().map_err(|_| FinOpsError::LockPoisoned)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (reserved, committed) = usage(&tx, tenant_id, agent_id)?;
        let used = reserved
            .checked_add(committed)
            .ok_or(FinOpsError::Overflow)?;
        if used > limit {
            return Err(FinOpsError::BudgetBelowUsage);
        }
        tx.execute(
            "INSERT INTO finops_budget(tenant_id, agent_id, limit_minor)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(tenant_id, agent_id)
             DO UPDATE SET limit_minor = excluded.limit_minor",
            params![tenant_id, agent_id, limit],
        )?;
        tx.commit()?;
        snapshot(limit, reserved, committed)
    }

    pub fn reserve(
        &self,
        tenant_id: &str,
        agent_id: &str,
        request_id: &str,
        amount_minor: u64,
    ) -> Result<ReservationState, FinOpsError> {
        validate_id(tenant_id, "tenant_id")?;
        validate_id(agent_id, "agent_id")?;
        validate_id(request_id, "request_id")?;
        if amount_minor == 0 {
            return Err(FinOpsError::InvalidAmount);
        }
        let amount = signed_amount(amount_minor)?;
        let mut conn = self.conn.lock().map_err(|_| FinOpsError::LockPoisoned)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<(i64, String)> = tx
            .query_row(
                "SELECT amount_minor, state FROM finops_reservation
                 WHERE tenant_id=?1 AND agent_id=?2 AND request_id=?3",
                params![tenant_id, agent_id, request_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((existing_amount, state)) = prior {
            let state = ReservationState::from_db(&state)?;
            if existing_amount != amount || state == ReservationState::Released {
                return Err(FinOpsError::IdempotencyConflict);
            }
            return Ok(state);
        }
        let limit: i64 = tx
            .query_row(
                "SELECT limit_minor FROM finops_budget
                 WHERE tenant_id=?1 AND agent_id=?2",
                params![tenant_id, agent_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(FinOpsError::BudgetNotConfigured)?;
        let (reserved, committed) = usage(&tx, tenant_id, agent_id)?;
        let new_total = reserved
            .checked_add(committed)
            .and_then(|used| used.checked_add(amount))
            .ok_or(FinOpsError::Overflow)?;
        if new_total > limit {
            return Err(FinOpsError::BudgetExceeded);
        }
        tx.execute(
            "INSERT INTO finops_reservation
               (tenant_id, agent_id, request_id, amount_minor, state)
             VALUES (?1, ?2, ?3, ?4, 'RESERVED')",
            params![tenant_id, agent_id, request_id, amount],
        )?;
        tx.commit()?;
        Ok(ReservationState::Reserved)
    }

    pub fn commit(
        &self,
        tenant_id: &str,
        agent_id: &str,
        request_id: &str,
    ) -> Result<ReservationState, FinOpsError> {
        self.transition(tenant_id, agent_id, request_id, ReservationState::Committed)
    }

    pub fn release(
        &self,
        tenant_id: &str,
        agent_id: &str,
        request_id: &str,
    ) -> Result<ReservationState, FinOpsError> {
        self.transition(tenant_id, agent_id, request_id, ReservationState::Released)
    }

    pub fn snapshot(&self, tenant_id: &str, agent_id: &str) -> Result<BudgetSnapshot, FinOpsError> {
        validate_id(tenant_id, "tenant_id")?;
        validate_id(agent_id, "agent_id")?;
        let mut conn = self.conn.lock().map_err(|_| FinOpsError::LockPoisoned)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Deferred)?;
        let limit: i64 = tx
            .query_row(
                "SELECT limit_minor FROM finops_budget
                 WHERE tenant_id=?1 AND agent_id=?2",
                params![tenant_id, agent_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(FinOpsError::BudgetNotConfigured)?;
        let (reserved, committed) = usage(&tx, tenant_id, agent_id)?;
        tx.commit()?;
        snapshot(limit, reserved, committed)
    }

    fn transition(
        &self,
        tenant_id: &str,
        agent_id: &str,
        request_id: &str,
        target: ReservationState,
    ) -> Result<ReservationState, FinOpsError> {
        validate_id(tenant_id, "tenant_id")?;
        validate_id(agent_id, "agent_id")?;
        validate_id(request_id, "request_id")?;
        let mut conn = self.conn.lock().map_err(|_| FinOpsError::LockPoisoned)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: String = tx
            .query_row(
                "SELECT state FROM finops_reservation
                 WHERE tenant_id=?1 AND agent_id=?2 AND request_id=?3",
                params![tenant_id, agent_id, request_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(FinOpsError::ReservationNotFound)?;
        let current = ReservationState::from_db(&current)?;
        if current == target {
            return Ok(target);
        }
        if current != ReservationState::Reserved {
            return Err(FinOpsError::InvalidTransition);
        }
        let target_db = match target {
            ReservationState::Committed => "COMMITTED",
            ReservationState::Released => "RELEASED",
            ReservationState::Reserved => return Err(FinOpsError::InvalidTransition),
        };
        tx.execute(
            "UPDATE finops_reservation SET state=?4
             WHERE tenant_id=?1 AND agent_id=?2 AND request_id=?3 AND state='RESERVED'",
            params![tenant_id, agent_id, request_id, target_db],
        )?;
        tx.commit()?;
        Ok(target)
    }
}

fn validate_id(value: &str, field: &'static str) -> Result<(), FinOpsError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
    {
        return Err(FinOpsError::InvalidIdentifier(field));
    }
    Ok(())
}

fn signed_amount(value: u64) -> Result<i64, FinOpsError> {
    i64::try_from(value).map_err(|_| FinOpsError::InvalidAmount)
}

fn usage(tx: &Transaction<'_>, tenant_id: &str, agent_id: &str) -> Result<(i64, i64), FinOpsError> {
    let (reserved, committed): (i64, i64) = tx.query_row(
        "SELECT
           COALESCE(SUM(CASE WHEN state='RESERVED' THEN amount_minor ELSE 0 END), 0),
           COALESCE(SUM(CASE WHEN state='COMMITTED' THEN amount_minor ELSE 0 END), 0)
         FROM finops_reservation WHERE tenant_id=?1 AND agent_id=?2",
        params![tenant_id, agent_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok((reserved, committed))
}

fn snapshot(limit: i64, reserved: i64, committed: i64) -> Result<BudgetSnapshot, FinOpsError> {
    let used = reserved
        .checked_add(committed)
        .ok_or(FinOpsError::Overflow)?;
    let available = limit.checked_sub(used).ok_or(FinOpsError::BudgetExceeded)?;
    Ok(BudgetSnapshot {
        limit_minor: u64::try_from(limit).map_err(|_| FinOpsError::Overflow)?,
        reserved_minor: u64::try_from(reserved).map_err(|_| FinOpsError::Overflow)?,
        committed_minor: u64::try_from(committed).map_err(|_| FinOpsError::Overflow)?,
        available_minor: u64::try_from(available).map_err(|_| FinOpsError::Overflow)?,
    })
}
