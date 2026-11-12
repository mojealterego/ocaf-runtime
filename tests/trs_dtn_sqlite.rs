use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim,acknowledge,release};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
fn bundle() -> ReliableBundle { ReliableBundle{id:[7;32],ciphertext:vec![8;16],expires_at:100,hops_remaining:3} }
#[test]
fn leases_survive_restart_and_retry_at_deadline() {
 let file=tempfile::NamedTempFile::new().unwrap();
 {let mut db=Connection::open(file.path()).unwrap();initialize(&db).unwrap();insert(&mut db,"tenant", &bundle(),0,16,1024).unwrap();
 assert_eq!(claim(&mut db,"tenant","peer",1,10,1).unwrap().len(),1);}
 let mut db=Connection::open(file.path()).unwrap();
 assert!(claim(&mut db,"tenant","peer",9,10,1).unwrap().is_empty());
 assert_eq!(claim(&mut db,"tenant","peer",11,10,1).unwrap().len(),1);
 acknowledge(&mut db,"tenant","peer",&[7;32],12).unwrap();
 assert!(claim(&mut db,"tenant","peer",13,10,1).unwrap().is_empty());
}
#[test]
fn tenant_isolation_and_explicit_failed_send_recovery() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();insert(&mut db,"a",&bundle(),0,16,1024).unwrap();
 assert!(claim(&mut db,"b","peer",1,10,1).unwrap().is_empty());
 assert_eq!(claim(&mut db,"a","peer",1,10,1).unwrap().len(),1);
 assert!(release(&mut db,"b","peer",&[7;32]).is_err());
 release(&mut db,"a","peer",&[7;32]).unwrap();
 assert_eq!(claim(&mut db,"a","peer",2,10,1).unwrap().len(),1);
}
