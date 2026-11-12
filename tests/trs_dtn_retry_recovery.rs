use ocaf_runtime::trs_dtn_reliable::{ReliableBundle,ReliableDtn,ReliableError};
fn bundle() -> ReliableBundle { ReliableBundle{id:[7;32],ciphertext:vec![9;16],expires_at:100,hops_remaining:3} }
#[test]
fn failed_send_can_be_released_for_immediate_retry() {
 let mut d=ReliableDtn::new(2,32,10).unwrap();d.insert(bundle(),0).unwrap();
 assert_eq!(d.select("peer",1,1).unwrap().len(),1);
 assert!(d.select("peer",2,1).unwrap().is_empty());
 d.cancel_pending(&[7;32],"peer").unwrap();
 assert_eq!(d.select("peer",2,1).unwrap().len(),1);
}
#[test]
fn cancel_requires_existing_inflight_and_does_not_remove_ack() {
 let mut d=ReliableDtn::new(2,32,10).unwrap();d.insert(bundle(),0).unwrap();
 assert_eq!(d.cancel_pending(&[7;32],"peer"),Err(ReliableError::NotInFlight));
 d.select("peer",1,1).unwrap();d.acknowledge(&[7;32],"peer",1).unwrap();
 assert_eq!(d.cancel_pending(&[7;32],"peer"),Err(ReliableError::NotInFlight));
 assert!(d.select("peer",20,1).unwrap().is_empty());
}
