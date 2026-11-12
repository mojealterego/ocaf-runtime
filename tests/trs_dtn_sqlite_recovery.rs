use ocaf_runtime::trs_dtn_sqlite::{acknowledge,claim,initialize,insert,purge_expired};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;

#[test]
fn uncommitted_lease_is_rolled_back_after_process_failure_simulation() {
 let file=tempfile::NamedTempFile::new().unwrap();
 let mut db=Connection::open(file.path()).unwrap();
 initialize(&db).unwrap();
 let b=ReliableBundle{id:[201;32],ciphertext:vec![3],expires_at:100,hops_remaining:3};
 insert(&mut db,"t",&b,0,2,16).unwrap();
 {
   let tx=db.transaction().unwrap();
   tx.execute("INSERT INTO trs_dtn_delivery(tenant,id,peer,deadline,ack) VALUES(?1,?2,?3,?4,0)",
       rusqlite::params!["t",b.id.as_slice(),"p","00000000000000000020"]).unwrap();
   // A crash before COMMIT must not leave an orphaned lease.
 }
 drop(db);
 let mut reopened=Connection::open(file.path()).unwrap();
 assert_eq!(claim(&mut reopened,"t","p",1,10,1).unwrap().len(),1);
 acknowledge(&mut reopened,"t","p",&b.id,2).unwrap();
 assert!(claim(&mut reopened,"t","p",3,10,1).unwrap().is_empty());
}

#[test]
fn expired_bundle_and_lease_are_cascaded_across_restart() {
 let file=tempfile::NamedTempFile::new().unwrap();
 {
   let mut db=Connection::open(file.path()).unwrap();
   initialize(&db).unwrap();
   let b=ReliableBundle{id:[202;32],ciphertext:vec![3],expires_at:10,hops_remaining:3};
   insert(&mut db,"t",&b,0,2,16).unwrap();
   claim(&mut db,"t","p",1,3,1).unwrap();
 }
 let mut db=Connection::open(file.path()).unwrap();
 assert_eq!(purge_expired(&mut db,"t",10).unwrap(),1);
 let count:i64=db.query_row("SELECT COUNT(*) FROM trs_dtn_delivery WHERE tenant='t'",[],|r|r.get(0)).unwrap();
 assert_eq!(count,0);
}
