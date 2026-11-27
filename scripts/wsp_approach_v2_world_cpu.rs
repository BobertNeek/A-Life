// Device-free tests against the actual game World, never against a saved actor.
use alife_core::{LanguageTokenId,OrganismId,UtteranceSourceKind,Vec3f};
use alife_world::{HeadlessScenarioBuilder,WorldObjectKind};

#[test]
fn real_world_produces_silence_one_word_and_ordered_pair_without_dispatch() {
    let owner=OrganismId(1);
    let mut world=HeadlessScenarioBuilder::new(8123)
        .agent("test-listener",owner,Vec3f::ZERO)
        .food("test-food",Vec3f::new(2.1,0.0,0.35),0.6)
        .toy("test-ball",Vec3f::new(2.0,0.0,-0.4),true)
        .build().unwrap();
    let now=world.tick();
    let targets=world.object_snapshots().into_iter()
        .filter(|p|matches!(p.kind,WorldObjectKind::Food | WorldObjectKind::Ball)).collect::<Vec<_>>();
    assert_eq!(targets.len(),2);
    for p in &targets {
        assert_eq!(world.retained_language_pilot_grab_probe(owner,p.id),(false,false));
    }
    let heard=|w:&alife_world::HeadlessWorld|w.sensory_report(owner,now).unwrap().core_snapshot.language_context.heard_tokens;
    assert!(world.audible_utterances().is_empty() && heard(&world).iter().all(Option::is_none));
    let mut one=world.clone();
    let stamp=one.emit_player_tokens(Some(owner),Vec3f::new(0.25,0.0,0.0),vec![LanguageTokenId::new(9).unwrap()]).unwrap();
    let h=heard(&one);
    assert_eq!(h.iter().flatten().count(),1);
    assert_eq!(h[0].unwrap().utterance_id,stamp.utterance_id);
    assert!(h[1].is_none());
    let stamp=world.emit_player_tokens(Some(owner),Vec3f::new(0.25,0.0,0.0),
       vec![LanguageTokenId::new(9).unwrap(),LanguageTokenId::new(14).unwrap()]).unwrap();
    let h=heard(&world);
    assert_eq!(h.iter().flatten().count(),2);
    for (j,noun) in [9,14].into_iter().enumerate() {
        let cue=h[j].unwrap();
        assert_eq!(cue.source_kind,UtteranceSourceKind::Player);
        assert_eq!((cue.token_id,usize::from(cue.sequence_position)),(noun,j));
        assert_eq!(cue.utterance_id,stamp.utterance_id);
    }
    assert_eq!(world.tick(),now);
}
