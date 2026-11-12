//! Durable bounded DTN queue. Caller authenticates peers and supplies trusted time.
//! SQLite is a single-node store, not a replicated or encrypted transport.
use crate::trs_dtn_reliable::ReliableBundle;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

#[derive(Debug)]
pub enum DtnStoreError { Invalid, Duplicate, Capacity, Missing, Storage(rusqlite::Error) }
impl From<rusqlite::Error> for DtnStoreError {
    fn from(e: rusqlite::Error) -> Self { Self::Storage(e) }
}
fn identity(s: &str) -> bool { !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control) }
pub fn initialize(conn: &Connection) -> Result<(), DtnStoreError> {
    conn.execute_batch("PRAGMA synchronous=FULL;
        CREATE TABLE IF NOT EXISTS trs_dtn_bundles(
          tenant TEXT NOT NULL, id BLOB NOT NULL, ciphertext BLOB NOT NULL,
          expires TEXT NOT NULL, hops INTEGER NOT NULL,
          PRIMARY KEY(tenant,id)) WITHOUT ROWID;
        CREATE TABLE IF NOT EXISTS trs_dtn_clock(
          tenant TEXT PRIMARY KEY NOT NULL,
          high_water TEXT NOT NULL
        ) WITHOUT ROWID;
        CREATE TABLE IF NOT EXISTS trs_dtn_delivery(
          tenant TEXT NOT NULL, id BLOB NOT NULL, peer TEXT NOT NULL,
          deadline TEXT, ack INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY(tenant,id,peer),
          FOREIGN KEY(tenant,id) REFERENCES trs_dtn_bundles(tenant,id) ON DELETE CASCADE
        ) WITHOUT ROWID; PRAGMA foreign_keys=ON;")?;
    Ok(())
}
/// Clock checks and updates must run inside the same IMMEDIATE transaction
/// as the operation, so concurrent connections cannot bypass the watermark.
fn check_and_advance_clock(
    tx: &rusqlite::Transaction<'_>,
    tenant: &str,
    now: u64,
) -> Result<(), DtnStoreError> {
    let prior: Option<String> = tx.query_row(
        "SELECT high_water FROM trs_dtn_clock WHERE tenant=?1",
        params![tenant], |row| row.get(0),
    ).optional()?;
    if let Some(previous) = prior {
        let previous = previous.parse::<u64>().map_err(|_| DtnStoreError::Invalid)?;
        if now < previous { return Err(DtnStoreError::Invalid); }
    }
    tx.execute(
        "INSERT INTO trs_dtn_clock(tenant,high_water) VALUES(?1,?2)
         ON CONFLICT(tenant) DO UPDATE SET high_water=excluded.high_water",
        params![tenant,format!("{now:020}")],
    )?;
    Ok(())
}

/// Bounds are enforced for every insertion, not merely at startup.
pub fn insert(conn: &mut Connection, tenant: &str, bundle: &ReliableBundle,
              now: u64, max_bundles: usize, max_payload: usize) -> Result<(), DtnStoreError> {
    if !identity(tenant) || max_bundles == 0 || max_bundles > 4096
        || max_payload == 0 || max_payload > 65536
        || max_bundles.checked_mul(max_payload).map_or(true, |n| n > 64*1024*1024)
        || bundle.ciphertext.is_empty() || bundle.ciphertext.len() > max_payload
        || bundle.hops_remaining == 0 || bundle.expires_at <= now { return Err(DtnStoreError::Invalid); }
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    check_and_advance_clock(&tx, tenant, now)?;
    tx.execute("DELETE FROM trs_dtn_bundles WHERE tenant=?1 AND expires<=?2",
               params![tenant,format!("{now:020}")])?;
    let existing: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM trs_dtn_bundles WHERE tenant=?1 AND id=?2)",
               params![tenant,bundle.id.as_slice()],|r|r.get(0))?;
    if existing { return Err(DtnStoreError::Duplicate); }
    let count: i64=tx.query_row("SELECT COUNT(*) FROM trs_dtn_bundles WHERE tenant=?1",params![tenant],|r|r.get(0))?;
    if count as usize>=max_bundles { return Err(DtnStoreError::Capacity); }
    tx.execute("INSERT INTO trs_dtn_bundles(tenant,id,ciphertext,expires,hops) VALUES(?1,?2,?3,?4,?5)",
               params![tenant,bundle.id.as_slice(),&bundle.ciphertext,format!("{:020}",bundle.expires_at),i64::from(bundle.hops_remaining)])?;
    tx.commit()?;Ok(())
}
pub fn claim(conn: &mut Connection, tenant: &str, peer: &str, now: u64,
             retry_secs: u64, limit: usize) -> Result<Vec<ReliableBundle>, DtnStoreError> {
    if !identity(tenant)||!identity(peer)||retry_secs==0||limit==0||limit>4096 { return Err(DtnStoreError::Invalid); }
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute("DELETE FROM trs_dtn_bundles WHERE tenant=?1 AND expires<=?2",
               params![tenant,format!("{now:020}")])?;
    check_and_advance_clock(&tx, tenant, now)?;
    let mut candidates=Vec::new();
    {
        // Filter saturated fanout in SQL before LIMIT to bound ciphertext memory.
        let mut stmt=tx.prepare("SELECT b.id,b.ciphertext,b.expires,b.hops FROM trs_dtn_bundles b
           LEFT JOIN trs_dtn_delivery d ON d.tenant=b.tenant AND d.id=b.id AND d.peer=?2
           WHERE b.tenant=?1 AND b.hops>1 AND b.expires>?3
             AND (d.ack IS NULL OR (d.ack=0 AND d.deadline<=?3))
             AND (d.peer IS NOT NULL OR
                  (SELECT COUNT(*) FROM trs_dtn_delivery f
                   WHERE f.tenant=b.tenant AND f.id=b.id)<64)
           ORDER BY b.id LIMIT ?4")?;
        let rows=stmt.query_map(params![tenant,peer,format!("{now:020}"),limit as i64],|r|{
            let id:Vec<u8>=r.get(0)?;let ciphertext:Vec<u8>=r.get(1)?;
            let expires:String=r.get(2)?;let hops:i64=r.get(3)?;
            Ok((id,ciphertext,expires,hops))
        })?;
        for row in rows { candidates.push(row?); }
    }
    let mut out=Vec::new();
    for (id, ciphertext, expires, hops) in candidates {
        if out.len() >= limit { break; }
        if id.len()!=32 || !(2..=255).contains(&hops) { return Err(DtnStoreError::Invalid); }
        let mut key=[0u8;32];key.copy_from_slice(&id);
        let expires_at=expires.parse::<u64>().map_err(|_|DtnStoreError::Invalid)?;
        if expires_at <= now { continue; }
        tx.execute("INSERT INTO trs_dtn_delivery(tenant,id,peer,deadline,ack) VALUES(?1,?2,?3,?4,0)
                    ON CONFLICT(tenant,id,peer) DO UPDATE SET deadline=excluded.deadline WHERE ack=0",
                   params![tenant,&id,peer,format!("{:020}",now.saturating_add(retry_secs).min(expires_at))])?;
        out.push(ReliableBundle{id:key,ciphertext,expires_at,hops_remaining:(hops-1) as u8});
    }
    tx.commit()?;Ok(out)
}
pub fn acknowledge(conn:&mut Connection,tenant:&str,peer:&str,id:&[u8;32],now:u64)->Result<(),DtnStoreError>{
    if !identity(tenant)||!identity(peer) {return Err(DtnStoreError::Invalid);}
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    check_and_advance_clock(&tx, tenant, now)?;
    let expires:Option<String>=tx.query_row("SELECT expires FROM trs_dtn_bundles WHERE tenant=?1 AND id=?2",
       params![tenant,id.as_slice()],|r|r.get(0)).optional()?;
    let expires=expires.ok_or(DtnStoreError::Missing)?.parse::<u64>().map_err(|_|DtnStoreError::Invalid)?;
    if expires<=now {return Err(DtnStoreError::Missing);}
    let n=tx.execute("UPDATE trs_dtn_delivery SET ack=1,deadline=NULL WHERE tenant=?1 AND id=?2 AND peer=?3 AND ack=0 AND deadline>=?4",
                     params![tenant,id.as_slice(),peer,format!("{now:020}")])?;
    if n!=1 {return Err(DtnStoreError::Missing);}
    tx.commit()?;Ok(())
}
pub fn release(conn:&mut Connection,tenant:&str,peer:&str,id:&[u8;32])->Result<(),DtnStoreError>{
    if !identity(tenant)||!identity(peer) {return Err(DtnStoreError::Invalid);}
    let n=conn.execute("DELETE FROM trs_dtn_delivery WHERE tenant=?1 AND id=?2 AND peer=?3 AND ack=0",
                       params![tenant,id.as_slice(),peer])?;
    if n!=1 {return Err(DtnStoreError::Missing);}
    Ok(())
}

/// Atomically removes expired bundles and their delivery leases for one tenant.
/// Uses the same fixed-width u64 representation as insertion and claiming.
pub fn purge_expired(
    conn: &mut Connection,
    tenant: &str,
    now: u64,
) -> Result<usize, DtnStoreError> {
    if !identity(tenant) { return Err(DtnStoreError::Invalid); }
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    check_and_advance_clock(&tx, tenant, now)?;
    let deleted = tx.execute(
        "DELETE FROM trs_dtn_bundles WHERE tenant=?1 AND expires<=?2",
        params![tenant, format!("{now:020}")],
    )?;
    tx.commit()?;
    Ok(deleted)
}
