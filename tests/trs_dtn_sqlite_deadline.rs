use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;

#[test]
fn retry_deadline_never_exceeds_bundle_expiration() {
    let mut db=Connection::open_in_memory().unwrap();
    initialize(&db).unwrap();
    let bundle=ReliableBundle{id:[89;32],ciphertext:vec![1],expires_at:100,hops_remaining:2};
    insert(&mut db,"tenant",&bundle,0,1,16).unwrap();
    assert_eq!(claim(&mut db,"tenant","peer",90,1000,1).unwrap().len(),1);
    let deadline:String=db.query_row(
        "SELECT deadline FROM trs_dtn_delivery WHERE tenant='tenant' AND peer='peer'",
        [],|row|row.get(0)
    ).unwrap();
    assert_eq!(deadline,"00000000000000000100");
}
#[test]
fn overflow_retry_deadline_is_clamped_to_expiration() {
    let mut db=Connection::open_in_memory().unwrap();
    initialize(&db).unwrap();
    let bundle=ReliableBundle{id:[90;32],ciphertext:vec![1],expires_at:u64::MAX,hops_remaining:2};
    insert(&mut db,"tenant",&bundle,u64::MAX-10,1,16).unwrap();
    assert_eq!(claim(&mut db,"tenant","peer",u64::MAX-9,u64::MAX,1).unwrap().len(),1);
    let deadline:String=db.query_row(
        "SELECT deadline FROM trs_dtn_delivery WHERE tenant='tenant' AND peer='peer'",
        [],|row|row.get(0)
    ).unwrap();
    assert_eq!(deadline,format!("{:020}",u64::MAX));
}
