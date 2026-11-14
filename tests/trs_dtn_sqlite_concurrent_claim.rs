use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use ocaf_runtime::trs_dtn_sqlite::{claim,initialize,insert};
use rusqlite::Connection;
use std::sync::{Arc,Barrier};
use std::thread;

#[test]
fn concurrent_claims_from_independent_connections_issue_one_lease() {
    let dir=tempfile::tempdir().unwrap();
    let db_path=dir.path().join("trs.sqlite");
    {
        let mut db=Connection::open(&db_path).unwrap();
        initialize(&db).unwrap();
        insert(&mut db,"tenant",&ReliableBundle{
            id:[7;32],ciphertext:vec![1,2,3],expires_at:100,hops_remaining:3
        },1,10,1024).unwrap();
    }
    let gate=Arc::new(Barrier::new(3));
    let mut handles=Vec::new();
    for _ in 0..2 {
        let path=db_path.clone();
        let barrier=gate.clone();
        handles.push(thread::spawn(move || {
            let mut db=Connection::open(path).unwrap();
            db.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
            initialize(&db).unwrap();
            barrier.wait();
            claim(&mut db,"tenant","same-peer",2,10,1).unwrap().len()
        }));
    }
    gate.wait();
    let delivered:usize=handles.into_iter().map(|h|h.join().unwrap()).sum();
    assert_eq!(delivered,1,"one peer must not receive duplicate concurrent leases");
    let db=Connection::open(&db_path).unwrap();
    let n:i64=db.query_row(
        "SELECT COUNT(*) FROM trs_dtn_delivery WHERE tenant='tenant' AND peer='same-peer'",
        [],|row|row.get(0)).unwrap();
    assert_eq!(n,1);
}
