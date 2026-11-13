use ocaf_runtime::trs_dtn_sqlite::{initialize,insert};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::{params,Connection};

#[test]
fn actual_tenant_payload_budget_cannot_be_bypassed_by_changing_limits() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 // A database may contain legacy records written with earlier configuration.
 // The next insert must enforce the actual byte budget, not just the new
 // max_bundles * max_payload estimate.
 for id in 0..1024u32 {
   let mut key=[0u8;32];key[..4].copy_from_slice(&id.to_be_bytes());
   db.execute("INSERT INTO trs_dtn_bundles(tenant,id,ciphertext,expires,hops) VALUES(?1,?2,?3,?4,?5)",
      params!["t",key.as_slice(),vec![7u8;65536],"00000000000000001000",2]).unwrap();
 }
 let next=ReliableBundle{id:[255;32],ciphertext:vec![1],expires_at:1000,hops_remaining:2};
 assert!(insert(&mut db,"t",&next,1,4096,16384).is_err());
 let count:i64=db.query_row("SELECT COUNT(*) FROM trs_dtn_bundles WHERE tenant='t'",[],|r|r.get(0)).unwrap();
 assert_eq!(count,1024);
}
