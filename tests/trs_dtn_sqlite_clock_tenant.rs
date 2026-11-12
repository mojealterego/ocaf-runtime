use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
#[test]
fn clock_watermark_is_tenant_scoped() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 for (tenant,id) in [("a",1u8),("b",2u8)] {
   insert(&mut db,tenant,&ReliableBundle{id:[id;32],ciphertext:vec![1],expires_at:100,hops_remaining:2},0,1,16).unwrap();
 }
 assert_eq!(claim(&mut db,"a","peer",50,10,1).unwrap().len(),1);
 assert_eq!(claim(&mut db,"b","peer",10,10,1).unwrap().len(),1);
 assert!(claim(&mut db,"a","peer",49,10,1).is_err());
 assert!(claim(&mut db,"b","peer",9,10,1).is_err());
}
