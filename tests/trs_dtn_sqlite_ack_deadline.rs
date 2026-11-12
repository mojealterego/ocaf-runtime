use ocaf_runtime::trs_dtn_sqlite::{acknowledge,claim,initialize,insert};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;

#[test]
fn late_ack_after_lease_deadline_is_rejected() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 let b=ReliableBundle{id:[111;32],ciphertext:vec![1],expires_at:100,hops_remaining:2};
 insert(&mut db,"t",&b,0,1,16).unwrap();
 assert_eq!(claim(&mut db,"t","peer",1,5,1).unwrap().len(),1);
 assert!(acknowledge(&mut db,"t","peer",&b.id,7).is_err());
 assert_eq!(claim(&mut db,"t","peer",7,5,1).unwrap().len(),1);
 acknowledge(&mut db,"t","peer",&b.id,8).unwrap();
 assert!(claim(&mut db,"t","peer",9,5,1).unwrap().is_empty());
}
#[test]
fn ack_at_exact_lease_deadline_is_accepted() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 let b=ReliableBundle{id:[112;32],ciphertext:vec![1],expires_at:100,hops_remaining:2};
 insert(&mut db,"t",&b,0,1,16).unwrap();
 claim(&mut db,"t","peer",1,5,1).unwrap();
 acknowledge(&mut db,"t","peer",&b.id,6).unwrap();
}
