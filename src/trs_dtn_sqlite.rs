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
        CREATE TABLE IF NOT EXISTS trs_dtn_delivery(
          tenant TEXT NOT NULL, id BLOB NOT NULL, peer TEXT NOT NULL,
          deadline TEXT, ack INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY(tenant,id,peer),
          FOREIGN KEY(tenant,id) REFERENCES trs_dtn_bundles(tenant,id) ON DELETE CASCADE
        ) WITHOUT ROWID; PRAGMA foreign_keys=ON;")?;
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
    let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute("DELETE FROM trs_dtn_bundles WHERE tenant=?1 AND CAST(expires AS INTEGER)<=?2",
               params![tenant,now.to_string()])?;
    let existing: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM trs_dtn_bundles WHERE tenant=?1 AND id=?2)",
               params![tenant,bundle.id.as_slice()],|r|r.get(0))?;
    if existing { return Err(DtnStoreError::Duplicate); }
    let count: i64=tx.query_row("SELECT COUNT(*) FROM trs_dtn_bundles WHERE tenant=?1",params![tenant],|r|r.get(0))?;
    if count as usize>=max_bundles { return Err(DtnStoreError::Capacity); }
    tx.execute("INSERT INTO trs_dtn_bundles(tenant,id,ciphertext,expires,hops) VALUES(?1,?2,?3,?4,?5)",
               params![tenant,bundle.id.as_slice(),&bundle.ciphertext,bundle.expires_at.to_string(),i64::from(bundle.hops_remaining)])?;
    tx.commit()?;Ok(())
}
pub fn claim(conn: &mut Connection, tenant: &str, peer: &str, now: u64,
             retry_secs: u64, limit: usize) -> Result<Vec<ReliableBundle>, DtnStoreError> {
    if !identity(tenant)||!identity(peer)||retry_secs==0||limit==0||limit>4096 { return Err(DtnStoreError::Invalid); }
    let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute("DELETE FROM trs_dtn_bundles WHERE tenant=?1 AND CAST(expires AS INTEGER)<=?2",
               params![tenant,now.to_string()])?;
    let mut candidates=Vec::new();
    {
        let mut stmt=tx.prepare("SELECT b.id,b.ciphertext,b.expires,b.hops FROM trs_dtn_bundles b
           LEFT JOIN trs_dtn_delivery d ON d.tenant=b.tenant AND d.id=b.id AND d.peer=?2
           WHERE b.tenant=?1 AND b.hops>1 AND (d.ack IS NULL OR (d.ack=0 AND CAST(d.deadline AS INTEGER)<=?3))
           ORDER BY b.id LIMIT ?4")?;
        let rows=stmt.query_map(params![tenant,peer,now.to_string(),limit as i64],|r|{
            let id:Vec<u8>=r.get(0)?;let ciphertext:Vec<u8>=r.get(1)?;
            let expires:String=r.get(2)?;let hops:i64=r.get(3)?;
            Ok((id,ciphertext,expires,hops))
        })?;
        for row in rows { candidates.push(row?); }
    }
    let mut out=Vec::new();
    for (id, ciphertext, expires, hops) in candidates {
        if id.len()!=32 || !(2..=255).contains(&hops) { return Err(DtnStoreError::Invalid); }
        let mut key=[0u8;32];key.copy_from_slice(&id);
        let expires_at=expires.parse::<u64>().map_err(|_|DtnStoreError::Invalid)?;
        let peers:i64=tx.query_row("SELECT COUNT(*) FROM trs_dtn_delivery WHERE tenant=?1 AND id=?2",
                                  params![tenant,&id],|r|r.get(0))?;
        let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM trs_dtn_delivery WHERE tenant=?1 AND id=?2 AND peer=?3)",
                                    params![tenant,&id,peer],|r|r.get(0))?;
        if !exists && peers>=64 { continue; }
        tx.execute("INSERT INTO trs_dtn_delivery(tenant,id,peer,deadline,ack) VALUES(?1,?2,?3,?4,0)
                    ON CONFLICT(tenant,id,peer) DO UPDATE SET deadline=excluded.deadline WHERE ack=0",
                   params![tenant,&id,peer,now.saturating_add(retry_secs).to_string()])?;
        out.push(ReliableBundle{id:key,ciphertext,expires_at,hops_remaining:(hops-1) as u8});
    }
    tx.commit()?;Ok(out)
}
pub fn acknowledge(conn:&mut Connection,tenant:&str,peer:&str,id:&[u8;32],now:u64)->Result<(),DtnStoreError>{
    if !identity(tenant)||!identity(peer) {return Err(DtnStoreError::Invalid);}
    let tx=conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let expires:Option<String>=tx.query_row("SELECT expires FROM trs_dtn_bundles WHERE tenant=?1 AND id=?2",
       params![tenant,id.as_slice()],|r|r.get(0)).optional()?;
    let expires=expires.ok_or(DtnStoreError::Missing)?.parse::<u64>().map_err(|_|DtnStoreError::Invalid)?;
    if expires<=now {return Err(DtnStoreError::Missing);}
    let n=tx.execute("UPDATE trs_dtn_delivery SET ack=1,deadline=NULL WHERE tenant=?1 AND id=?2 AND peer=?3 AND ack=0",
                     params![tenant,id.as_slice(),peer])?;
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
