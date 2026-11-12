use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim,purge_expired};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;

#[test]
fn purge_is_tenant_scoped_and_cascades_leases() {
    let mut db=Connection::open_in_memory().unwrap();
    initialize(&db).unwrap();
    for tenant in ["a","b"] {
        insert(&mut db,tenant,&ReliableBundle{
            id:[3;32],ciphertext:vec![1],expires_at:10,hops_remaining:2
        },0,1,16).unwrap();
        assert_eq!(claim(&mut db,tenant,"peer",1,20,1).unwrap().len(),1);
    }
    assert_eq!(purge_expired(&mut db,"a",10).unwrap(),1);
    assert_eq!(purge_expired(&mut db,"a",10).unwrap(),0);
    let a:i64=db.query_row("SELECT COUNT(*) FROM trs_dtn_delivery WHERE tenant='a'",[],|r|r.get(0)).unwrap();
    let b:i64=db.query_row("SELECT COUNT(*) FROM trs_dtn_delivery WHERE tenant='b'",[],|r|r.get(0)).unwrap();
    assert_eq!(a,0);
    assert_eq!(b,1);
    assert_eq!(purge_expired(&mut db,"b",10).unwrap(),1);
}
#[test]
fn purge_rejects_untrusted_identity_without_side_effects() {
    let mut db=Connection::open_in_memory().unwrap();
    initialize(&db).unwrap();
    assert!(purge_expired(&mut db,"\n",u64::MAX).is_err());
}
