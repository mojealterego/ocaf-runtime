use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
#[test]
fn reopened_connection_purges_delivery_rows_with_expired_bundle() {
 let file=tempfile::NamedTempFile::new().unwrap();
 {
  let mut db=Connection::open(file.path()).unwrap();initialize(&db).unwrap();
  let b=ReliableBundle{id:[6;32],ciphertext:vec![1],expires_at:10,hops_remaining:2};
  insert(&mut db,"tenant",&b,0,2,16).unwrap();
  claim(&mut db,"tenant","peer",1,5,1).unwrap();
 }
 let mut db=Connection::open(file.path()).unwrap();
 assert!(claim(&mut db,"tenant","peer",10,5,1).unwrap().is_empty());
 let count:i64=db.query_row("SELECT COUNT(*) FROM trs_dtn_delivery",[],|r|r.get(0)).unwrap();
 assert_eq!(count,0);
}
