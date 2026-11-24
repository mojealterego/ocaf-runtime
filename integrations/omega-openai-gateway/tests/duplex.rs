use omega_openai_gateway::duplex::SessionBudget;

#[test]
fn requires_audio_and_serializes_responses(){
    let mut s=SessionBudget::default();
    assert!(s.commit().is_err());
    s.append(4800).unwrap();
    s.commit().unwrap();
    assert!(s.commit().is_err());
    s.append(4800).unwrap();
    s.output(4).unwrap();
    s.done().unwrap();
    s.commit().unwrap();
}
#[test]
fn three_turns_and_bytes_are_bounded(){
    let mut s=SessionBudget::default();
    for _ in 0..3{
        s.append(4800).unwrap();s.commit().unwrap();s.output(2).unwrap();s.done().unwrap();
    }
    assert!(s.append(4800).is_err());
    let mut s=SessionBudget::default();
    assert!(s.append(1).is_err());
    assert!(s.append(32770).is_err());
    assert!(s.output(2).is_err());
    assert!(s.done().is_err());
}
#[test]
fn output_must_be_even_and_terminal_response_must_have_audio(){
    let mut s=SessionBudget::default();
    s.append(4800).unwrap();s.commit().unwrap();
    assert!(s.output(3).is_err());
    assert!(s.done().is_err());
    s.output(4).unwrap();
    s.done().unwrap();
    assert!(s.output(2).is_err());
}
