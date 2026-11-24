use omega_openai_gateway::webrtc::{validate_sdp,call_id,CallStore};
use std::path::Path;

const OFFER:&str="v=0\r\no=- 1 1 IN IP4 127.0.0.1\r\ns=-\r\nt=0 0\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\na=ice-ufrag:abc\r\na=ice-pwd:abcdefghijklmnopqrstuv\r\na=fingerprint:sha-256 AA:BB\r\na=rtcp-mux\r\n";

#[test]
fn only_one_audio_media_section(){
    assert!(validate_sdp(OFFER).is_ok());
    assert!(validate_sdp(&format!("{OFFER}m=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\n")).is_err());
    assert!(validate_sdp(&format!("{OFFER}m=video 9 UDP/TLS/RTP/SAVPF 96\r\n")).is_err());
    assert!(validate_sdp(&OFFER.replace("a=ice-pwd:abcdefghijklmnopqrstuv\r\n","")).is_err());
    assert!(validate_sdp("v=0\n").is_err());
}

#[test]
fn call_location_cannot_redirect_hangup(){
    assert_eq!(call_id("/v1/realtime/calls/rtc_123").unwrap(),"rtc_123");
    assert_eq!(call_id("https://api.openai.com/v1/realtime/calls/rtc_123").unwrap(),"rtc_123");
    for bad in ["https://other.example/v1/realtime/calls/x","/v1/realtime/calls/../x","/v1/realtime/calls/x?token=y","/v1/realtime/calls/x/extra"]{
        assert!(call_id(bad).is_err());
    }
}

#[test]
fn unresolved_leases_survive_restart_and_block_new_calls(){
    let dir=tempfile::tempdir().unwrap();
    let path=dir.path().join("rtc.sqlite");
    let store=CallStore::open(Path::new(&path)).unwrap();
    let ticket=store.begin().unwrap();
    assert!(store.begin().is_err());
    store.bind(&ticket,"rtc_123").unwrap();
    drop(store);
    let store=CallStore::open(&path).unwrap();
    let record=store.active().unwrap().unwrap();
    assert_eq!(record.ticket,ticket);
    assert_eq!(record.call_id.as_deref(),Some("rtc_123"));
    assert!(store.begin().is_err());
    assert!(!store.clear("wrong").unwrap());
    assert!(store.clear(&ticket).unwrap());
    assert!(store.begin().is_ok());
}

#[test]
fn unknown_creation_is_not_silently_released(){
    let dir=tempfile::tempdir().unwrap();
    let store=CallStore::open(&dir.path().join("rtc.sqlite")).unwrap();
    let ticket=store.begin().unwrap();
    let record=store.active().unwrap().unwrap();
    assert_eq!(record.ticket,ticket);
    assert!(record.call_id.is_none());
    assert!(store.bind("wrong","rtc_123").is_err());
    assert!(store.begin().is_err());
}
