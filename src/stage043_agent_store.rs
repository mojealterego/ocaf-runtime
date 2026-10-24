//! Durable staged-agent registry for OMAs 0.43. Operator-provisioned SQLite,
//! fail-closed; no ACTIVE admission and no source-access authorization.
use crate::omas_server::{
    AdmissionDecision, AgentRegistrationRequest, OmasServer, OmasServerConfig,
};
use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};
use std::path::{Path, PathBuf};

pub struct Stage043AgentStore {
    path: PathBuf,
    config: OmasServerConfig,
}

impl Stage043AgentStore {
    pub fn open(path: &Path, config: OmasServerConfig) -> Result<Self, String> {
        config.validate()?;
        if !path.is_file()
            || path
                .symlink_metadata()
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
        {
            return Err("operator-provisioned regular SQLite file required".into());
        }
        let store = Self {
            path: path.to_path_buf(),
            config,
        };
        let conn = store.connection()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS stage043_agents (
                tenant TEXT NOT NULL, agent_id TEXT NOT NULL,
                digest TEXT NOT NULL, lifecycle TEXT NOT NULL,
                PRIMARY KEY(tenant, agent_id)
            );
            CREATE TABLE IF NOT EXISTS stage043_agent_requests (
                request_id TEXT PRIMARY KEY
            );",
        )
        .map_err(|e| e.to_string())?;
        Ok(store)
    }

    fn connection(&self) -> Result<Connection, String> {
        let conn = Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        Ok(conn)
    }

    pub fn config(&self) -> &OmasServerConfig {
        &self.config
    }

    pub fn registered_count(&self) -> Result<usize, String> {
        let count: i64 = self
            .connection()?
            .query_row("SELECT COUNT(*) FROM stage043_agents", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        usize::try_from(count).map_err(|e| e.to_string())
    }

    pub fn admit(&self, req: AgentRegistrationRequest) -> Result<AdmissionDecision, String> {
        // Reuse the canonical OMAs validation contract without committing
        // the temporary in-memory registration as durable authority.
        let validator = OmasServer::new(self.config.clone())?;
        let preliminary = validator.admit_registration(req.clone());
        if !preliminary.admitted {
            return Ok(preliminary);
        }
        let deny = |reason: &str| AdmissionDecision {
            request_id: req.request_id.clone(),
            agent_id: req.agent_id.clone(),
            admitted: false,
            lifecycle: "DRAFT".into(),
            reason: reason.into(),
        };
        let mut conn = self.connection()?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let replay: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM stage043_agent_requests WHERE request_id = ?1",
                params![req.request_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if replay != 0 {
            return Ok(deny("replay_request_id"));
        }
        let count: i64 = tx
            .query_row("SELECT COUNT(*) FROM stage043_agents", [], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        let existing: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM stage043_agents WHERE tenant = ?1 AND agent_id = ?2",
                params![req.tenant, req.agent_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if existing == 0 && count >= i64::from(self.config.max_agents) {
            return Ok(deny("agent_capacity_exhausted"));
        }
        let request_count: i64 = tx
            .query_row("SELECT COUNT(*) FROM stage043_agent_requests", [], |row| {
                row.get(0)
            })
            .map_err(|e| e.to_string())?;
        if request_count >= i64::from(self.config.max_agents) * 16 {
            return Ok(deny("replay_capacity_exhausted"));
        }
        tx.execute(
            "INSERT INTO stage043_agent_requests(request_id) VALUES (?1)",
            params![req.request_id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO stage043_agents(tenant,agent_id,digest,lifecycle)
             VALUES (?1,?2,?3,'STAGED')
             ON CONFLICT(tenant,agent_id) DO UPDATE SET digest=excluded.digest",
            params![req.tenant, req.agent_id, req.manifest_digest],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(preliminary)
    }
}
