use ocaf_runtime::trs_dtn_sqlite::{initialize,insert,claim};
use ocaf_runtime::trs_dtn_reliable::ReliableBundle;
use rusqlite::Connection;
use std::sync::{Arc,Barrier};
use std::thread;
use std::time::Duration;

#[test]
fn concurrent_claims_of_same_peer_have_single_winner() {
    let file=tempfile::NamedTempFile::new().unwrap();
    let mut setup=Connection::open(file.path()).unwrap();
    initialize(&setup).unwrap();
    insert(&mut setup,"tenant",&ReliableBundle{
        id:[91;32],ciphertext:vec![1,2,3],expires_at:100,hops_remaining:3
    },0,1,32).unwrap();
    drop(setup);
    let barrier=Arc::new(Barrier::new(3));
    let handles:Vec<_>=(0..2).map(|_|{
        let path=file.path().to_path_buf();
        let barrier=Arc::clone(&barrier);
        thread::spawn(move || {
            let mut conn=Connection::open(path).unwrap();
            conn.busy_timeout(Duration::from_secs(5)).unwrap();
            barrier.wait();
            claim(&mut conn,"tenant","same-peer",1,20,1).unwrap().len()
        })
    }).collect();
    barrier.wait();
    let count:usize=handles.into_iter().map(|h|h.join().unwrap()).sum();
    assert_eq!(count,1,"exactly one lease must be granted");
}
