use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim,purge_expired,acknowledge};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
fn b(id:u8)->ReliableBundle {ReliableBundle{id:[id;32],ciphertext:vec![1],expires_at:100,hops_remaining:2}}
#[test]
fn rollback_is_rejected_across_all_time_bearing_operations() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 insert(&mut db,"t",&b(1),50,2,16).unwrap();
 assert!(insert(&mut db,"t",&b(2),49,2,16).is_err());
 assert!(purge_expired(&mut db,"t",49).is_err());
 assert_eq!(claim(&mut db,"t","p",51,10,1).unwrap().len(),1);
 assert!(acknowledge(&mut db,"t","p",&[1;32],50).is_err());
 acknowledge(&mut db,"t","p",&[1;32],52).unwrap();
 assert!(purge_expired(&mut db,"t",51).is_err());
}
#[test]
fn watermark_is_persisted_by_insert_even_without_claim() {
 let file=tempfile::NamedTempFile::new().unwrap();
 {let mut db=Connection::open(file.path()).unwrap();initialize(&db).unwrap();insert(&mut db,"t",&b(1),70,2,16).unwrap();}
 let mut db=Connection::open(file.path()).unwrap();
 assert!(insert(&mut db,"t",&b(2),69,2,16).is_err());
}
