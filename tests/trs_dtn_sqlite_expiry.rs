use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim,acknowledge};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
#[test]
fn expired_u64_max_bundle_must_not_be_claimed() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 let b=ReliableBundle{id:[33;32],ciphertext:vec![1],expires_at:u64::MAX-1,hops_remaining:2};
 insert(&mut db,"tenant",&b,u64::MAX-3,1,16).unwrap();
 assert!(claim(&mut db,"tenant","peer",u64::MAX-1,10,1).unwrap().is_empty());
}
#[test]
fn cannot_acknowledge_a_released_delivery() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 let b=ReliableBundle{id:[44;32],ciphertext:vec![1],expires_at:100,hops_remaining:2};
 insert(&mut db,"tenant",&b,0,1,16).unwrap();
 assert_eq!(claim(&mut db,"tenant","peer",1,10,1).unwrap().len(),1);
 ocaf_runtime::trs_dtn_sqlite::release(&mut db,"tenant","peer",&b.id).unwrap();
 assert!(acknowledge(&mut db,"tenant","peer",&b.id,2).is_err());
}
