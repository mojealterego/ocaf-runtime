use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
#[test]
fn rollback_clock_cannot_reopen_expired_lease() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 let b=ReliableBundle{id:[77;32],ciphertext:vec![1],expires_at:100,hops_remaining:2};
 insert(&mut db,"tenant",&b,0,1,16).unwrap();
 assert_eq!(claim(&mut db,"tenant","peer",50,10,1).unwrap().len(),1);
 assert!(claim(&mut db,"tenant","peer",40,10,1).is_err(),
   "trusted monotonic time must not move backwards");
}
#[test]
fn rollback_watermark_survives_reopen() {
 let file=tempfile::NamedTempFile::new().unwrap();
 {
 let mut db=Connection::open(file.path()).unwrap();initialize(&db).unwrap();
 let b=ReliableBundle{id:[78;32],ciphertext:vec![1],expires_at:100,hops_remaining:2};
 insert(&mut db,"tenant",&b,0,1,16).unwrap();
 claim(&mut db,"tenant","peer",50,10,1).unwrap();
 }
 let mut db=Connection::open(file.path()).unwrap();
 assert!(claim(&mut db,"tenant","peer",49,10,1).is_err());
}
