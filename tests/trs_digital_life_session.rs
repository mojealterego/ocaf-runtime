use ocaf_runtime::trs_digital_life::{AuthenticatedPrincipal, BridgeError, BridgeRequest, DIGITAL_LIFE_BRIDGE_VERSION};
use ocaf_runtime::trs_digital_life_session::{AuthorizedSession, SessionError};

fn req() -> BridgeRequest {
    BridgeRequest {version:DIGITAL_LIFE_BRIDGE_VERSION,tenant:"tenant-a".into(),sender:"worker".into(),operation:"trs.health".into(),payload:vec![]}
}
fn principal() -> AuthenticatedPrincipal {
    AuthenticatedPrincipal {tenant:"tenant-a".into(),sender:"worker".into(),allowed_operations:vec!["trs.health".into()]}
}
#[test]
fn replay_is_rejected_and_sequence_must_increase() {
    let mut session=AuthorizedSession::new(principal(), 10).unwrap();
    assert!(session.dispatch(1,&req()).is_ok());
    assert_eq!(session.dispatch(1,&req()),Err(SessionError::Replay));
    assert_eq!(session.dispatch(0,&req()),Err(SessionError::Replay));
    assert!(session.dispatch(2,&req()).is_ok());
}
#[test]
fn invalid_request_does_not_advance_sequence() {
    let mut session=AuthorizedSession::new(principal(), 10).unwrap();
    let mut invalid=req();invalid.sender="intruder".into();
    assert_eq!(session.dispatch(1,&invalid),Err(SessionError::Bridge(BridgeError::Unauthorized)));
    assert!(session.dispatch(1,&req()).is_ok());
}
#[test]
fn request_budget_exhaustion_fails_closed() {
    let mut session=AuthorizedSession::new(principal(), 1).unwrap();
    assert!(session.dispatch(1,&req()).is_ok());
    assert_eq!(session.dispatch(2,&req()),Err(SessionError::Exhausted));
    assert!(AuthorizedSession::new(principal(),0).is_err());
}
