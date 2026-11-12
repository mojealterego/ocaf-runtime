use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
#[test]
fn saturated_first_bundle_does_not_starve_next_bundle() {
 let mut db=Connection::open_in_memory().unwrap();initialize(&db).unwrap();
 for id in [1u8,2u8] {
  insert(&mut db,"tenant",&ReliableBundle{id:[id;32],ciphertext:vec![id],expires_at:100,hops_remaining:2},0,2,16).unwrap();
 }
 for i in 0..64 {
  let peer=format!("existing-{i}");
  let first=claim(&mut db,"tenant",&peer,1,10,1).unwrap();
  assert!(!first.is_empty());
 }
 let selected=claim(&mut db,"tenant","new-peer",2,10,1).unwrap();
 assert_eq!(selected.len(),1);
 assert_eq!(selected[0].id,[2;32]);
}
