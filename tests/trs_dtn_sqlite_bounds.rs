use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim,acknowledge};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
#[test]
fn u64_max_deadline_and_expiry_do_not_overflow_sqlite() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 let bundle=ReliableBundle{id:[0xA5;32],ciphertext:vec![1],expires_at:u64::MAX,hops_remaining:2};
 insert(&mut db,"tenant",&bundle,u64::MAX-100,1,1).unwrap();
 assert_eq!(claim(&mut db,"tenant","peer",u64::MAX-10,20,1).unwrap().len(),1);
 assert!(claim(&mut db,"tenant","peer",u64::MAX-1,20,1).unwrap().is_empty());
 acknowledge(&mut db,"tenant","peer",&bundle.id,u64::MAX-1).unwrap();
 assert!(claim(&mut db,"tenant","peer",u64::MAX-1,20,1).unwrap().is_empty());
}
#[test]
fn invalid_identity_and_exhausted_capacity_fail_closed() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 let a=ReliableBundle{id:[1;32],ciphertext:vec![1],expires_at:100,hops_remaining:2};
 let b=ReliableBundle{id:[2;32],ciphertext:vec![2],expires_at:100,hops_remaining:2};
 assert!(insert(&mut db,"",&a,0,1,16).is_err());
 insert(&mut db,"tenant",&a,0,1,16).unwrap();
 assert!(insert(&mut db,"tenant",&b,0,1,16).is_err());
 assert!(claim(&mut db,"tenant","\n",1,10,1).is_err());
}
