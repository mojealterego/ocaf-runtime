use omega_openai_gateway::voice::{VoiceRequest, TranscribeRequest, AudioCollector};
use serde_json::json;

#[test]
fn realtime_requires_opt_in_and_bounded_text_or_audio() {
    let mut req: VoiceRequest = serde_json::from_value(json!({"text":"Przeczytaj po polsku."})).unwrap();
    assert!(req.item(true).is_err());
    req.allow_cloud = true;
    assert!(req.item(false).is_err());
    assert_eq!(req.item(true).unwrap()["item"]["content"][0]["type"], "input_text");
    req.pcm16_base64 = Some("AAAA".into());
    assert!(req.item(true).is_err());
}
#[test]
fn realtime_accepts_only_even_length_pcm() {
    let mut req: VoiceRequest = serde_json::from_value(json!({"allow_cloud":true,"pcm16_base64":"AAABAA=="})).unwrap();
    assert_eq!(req.item(true).unwrap()["item"]["content"][0]["type"], "input_audio");
    req.pcm16_base64 = Some("AA==".into());
    assert!(req.item(true).is_err());
}
#[test]
fn transcription_checks_container_and_consent() {
    let mut req: TranscribeRequest = serde_json::from_value(json!({"wav_base64":"UklGRgQAAABXQVZF"})).unwrap();
    assert!(req.wav(true).is_err());
    req.allow_cloud = true;
    assert!(req.wav(false).is_err());
    assert_eq!(&req.wav(true).unwrap()[8..12], b"WAVE");
    req.wav_base64 = "AAAA".into();
    assert!(req.wav(true).is_err());
}
#[test]
fn audio_collector_requires_completed_terminal_event() {
    let mut c = AudioCollector::default();
    assert!(c.push(&json!({"type":"response.output_audio.delta","delta":"AAAAAA=="})).is_ok());
    assert!(c.finish().is_err());
    assert!(c.push(&json!({"type":"response.done","response":{"status":"completed"}})).unwrap());
    assert_eq!(c.finish().unwrap(), &[0,0,0,0]);
}
#[test]
fn invalid_or_failed_events_are_rejected() {
    let mut c = AudioCollector::default();
    assert!(c.push(&json!({"type":"response.output_audio.delta","delta":"!"})).is_err());
    assert!(c.push(&json!({"type":"response.done","response":{"status":"failed"}})).is_err());
    assert!(c.push(&json!({"type":"error"})).is_err());
}
