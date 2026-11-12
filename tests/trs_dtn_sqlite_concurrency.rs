use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim,acknowledge};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
#[test]
fn competing_connections_must_not_claim_same_lease() {
 let file=tempfile::NamedTempFile::new().unwrap();
 let mut first=Connection::open(file.path()).unwrap();
 initialize(&first).unwrap();
 let b=ReliableBundle{id:[3;32],ciphertext:vec![1],expires_at:100,hops_remaining:2};
 insert(&mut first,"tenant",&b,0,2,16).unwrap();
 let mut second=Connection::open(file.path()).unwrap();
 assert_eq!(claim(&mut first,"tenant","peer",1,10,1).unwrap().len(),1);
 assert!(claim(&mut second,"tenant","peer",2,10,1).unwrap().is_empty());
 assert_eq!(claim(&mut second,"tenant","peer",11,10,1).unwrap().len(),1);
 acknowledge(&mut first,"tenant","peer",&b.id,12).unwrap();
 assert!(claim(&mut second,"tenant","peer",13,10,1).unwrap().is_empty());
}
#[test]
fn unacknowledged_delivery_is_not_success() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 let b=ReliableBundle{id:[5;32],ciphertext:vec![1],expires_at:100,hops_remaining:2};
 insert(&mut db,"tenant",&b,0,1,16).unwrap();
 assert!(acknowledge(&mut db,"tenant","peer",&b.id,1).is_err());
 assert_eq!(claim(&mut db,"tenant","peer",2,10,1).unwrap().len(),1);
}
