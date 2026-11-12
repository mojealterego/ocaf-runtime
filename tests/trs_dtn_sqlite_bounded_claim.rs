use ocaf_runtime::trs_dtn_sqlite::{claim,initialize,insert};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;

#[test]
fn bounded_claim_skips_saturated_bundles_without_materializing_them() {
    let mut db=Connection::open_in_memory().unwrap();
    initialize(&db).unwrap();
    for id in 1..=32u8 {
        insert(&mut db,"t",&ReliableBundle{id:[id;32],ciphertext:vec![id;4096],expires_at:100,hops_remaining:3},0,32,4096).unwrap();
    }
    for i in 0..64 {
        let peer=format!("peer-{i}");
        let selected=claim(&mut db,"t",&peer,1,10,1).unwrap();
        assert_eq!(selected[0].id,[1;32]);
    }
    let selected=claim(&mut db,"t","new-peer",2,10,2).unwrap();
    assert_eq!(selected.len(),2);
    assert_eq!(selected[0].id,[2;32]);
    assert_eq!(selected[1].id,[3;32]);
}
