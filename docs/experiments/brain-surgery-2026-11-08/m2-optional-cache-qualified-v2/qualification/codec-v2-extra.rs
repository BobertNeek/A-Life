fn receipt_body_bits(b: &BodyEventDelta) -> Vec<u32> {
    vec![b.energy, b.damage, b.temperature_stress, b.nutrition, b.social_contact,
        b.player_reward, b.play_stimulation, b.perceived_novelty, b.investigation,
        b.sleep_recovery, b.mating_opportunity].into_iter().map(f32::to_bits).collect()
}
fn receipt_keys(value: &Value, names: &[&str]) {
    let actual: std::collections::BTreeSet<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(actual, names.iter().copied().collect());
}
fn receipt_complete_bytes(receipts: &[HeadlessMotorTransactionReceipt]) -> Vec<u8> {
    let originals = receipts.iter().map(world_receipt).collect::<Vec<_>>();
    for (r, j) in receipts.iter().zip(&originals) {
        receipt_keys(j, &["bundle", "outcome_tick", "succeeded", "joint", "channels", "grip_transfers", "body_event", "biology_before", "biology_after"]);
        assert_eq!(j["channels"].as_array().unwrap().len(), r.channel_receipts.len());
        for c in j["channels"].as_array().unwrap() {
            receipt_keys(c, &["command", "phase", "grip_transfers", "play_stimulation", "investigation", "observation", "physical"]);
        }
        for t in r.grip_transfers.iter().chain(r.channel_receipts.iter().flat_map(|c| &c.grip_transfers)) {
            receipt_keys(&transfer(t), &["object", "kind", "previous_owner", "new_owner", "previous_position", "new_position"]);
        }
    }
    // Retain exact historical codec bytes as an opaque byte array, not parsed/reencoded.
    // Debug is supplemental, not a replacement codec: derived Debug includes private
    // nested fields skipped by serde. BodyEventDelta raw bits explicitly bind -0 skips.
    serde_json::to_vec(&(
        serde_json::to_vec(&originals).unwrap(),
        format!("{:?}", receipts),
        receipts.iter().map(|r| receipt_body_bits(&r.body_event)).collect::<Vec<_>>(),
    )).unwrap()
}
#[test]
fn dual_policy_receipt_codec_all_fields_null_signed_zero_and_framing() {
    // Pure value/encoding controls only. No phenotype, source, biology, world,
    // host, founder, recipe, or native-forward constructor is called.
    for kind in [GripTransferKind::Acquired, GripTransferKind::Released, GripTransferKind::Placed, GripTransferKind::Consumed] {
        let t = GripTransfer { object: WorldEntityId(7), previous_owner: None,
            new_owner: Some(OrganismId(9)), previous_position: Vec3f::new(-0.0, 0.25, -0.5),
            new_position: Vec3f::new(0.0, -0.25, 0.5), kind };
        let j = transfer(&t);
        receipt_keys(&j, &["object", "kind", "previous_owner", "new_owner", "previous_position", "new_position"]);
        assert_eq!(j["object"], json!(7)); assert_eq!(j["previous_owner"], Value::Null);
        assert_eq!(j["new_owner"], json!(9)); assert_eq!(j["kind"], json!(format!("{:?}", kind)));
        let mut reverse = t;
        reverse.previous_owner = Some(OrganismId(3)); reverse.new_owner = None;
        let reversed = transfer(&reverse);
        assert_eq!(reversed["previous_owner"], json!(3)); assert!(reversed["new_owner"].is_null());
        assert_ne!(serde_json::to_vec(&j).unwrap(), serde_json::to_vec(&reversed).unwrap());
        let bytes = serde_json::to_vec(&j).unwrap();
        let decoded: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!((decoded["previous_position"]["x"].as_f64().unwrap() as f32).to_bits(), (-0.0_f32).to_bits());
        assert_eq!((decoded["new_position"]["x"].as_f64().unwrap() as f32).to_bits(), 0.0_f32.to_bits());
        assert_eq!(decoded["previous_position"], json!(t.previous_position));
        assert_eq!(decoded["new_position"], json!(t.new_position));
    }
    assert_ne!(serde_json::to_vec(&Value::Null).unwrap(), serde_json::to_vec(&json!([])).unwrap());
    assert!(!receipt_complete_bytes(&[]).is_empty());
    let positive = BodyEventDelta::zero();
    let mut negative = positive;
    negative.player_reward = -0.0; negative.play_stimulation = -0.0;
    negative.perceived_novelty = -0.0; negative.investigation = -0.0;
    assert_eq!(serde_json::to_vec(&positive).unwrap(), serde_json::to_vec(&negative).unwrap());
    assert_ne!(receipt_body_bits(&positive), receipt_body_bits(&negative));
    assert_ne!(format!("{:?}", positive), format!("{:?}", negative));
    assert_eq!(&receipt_body_bits(&negative)[5..9], &[0x80000000; 4]);
    assert_eq!(receipt_body_bits(&negative).len(), 11);
    let transaction = json!({"bundle":{},"outcome_tick":7,"succeeded":false,"joint":{},
        "channels":[{"command":{},"phase":"Attempt","grip_transfers":[],"play_stimulation":-0.0,
            "investigation":0.0,"observation":null,"physical":{}}],"grip_transfers":[],
        "body_event":{},"biology_before":{},"biology_after":{}});
    receipt_keys(&transaction, &["bundle","outcome_tick","succeeded","joint","channels","grip_transfers","body_event","biology_before","biology_after"]);
    receipt_keys(&transaction["channels"][0], &["command","phase","grip_transfers","play_stimulation","investigation","observation","physical"]);
    let json_bytes = serde_json::to_vec(&transaction).unwrap();
    let roundtrip: Value = serde_json::from_slice(&json_bytes).unwrap();
    assert!(roundtrip["channels"][0]["observation"].is_null());
    assert_eq!((roundtrip["channels"][0]["play_stimulation"].as_f64().unwrap() as f32).to_bits(), 0x80000000);
    let payloads = vec![Vec::new(), (0..=255).collect::<Vec<u8>>(), json_bytes,
        serde_json::to_vec(&receipt_body_bits(&negative)).unwrap(), b"null\0[]\n-0.0".to_vec()];
    let mut expected = Vec::new();
    for bytes in &payloads {
        expected.extend_from_slice(&(bytes.len() as u64).to_le_bytes()); expected.extend_from_slice(bytes);
        crate::qualification_sink::write_frame(bytes);
    }
    let actual = std::fs::read(std::env::var("DUAL_WITNESS_PATH").unwrap()).unwrap();
    assert_eq!(actual, expected);
    let mut cursor = 0;
    for bytes in &payloads {
        let length = u64::from_le_bytes(actual[cursor..cursor+8].try_into().unwrap()) as usize; cursor += 8;
        assert_eq!(length, bytes.len()); assert_eq!(&actual[cursor..cursor+length], bytes); cursor += length;
    }
    assert_eq!(cursor, actual.len());
    eprintln!("DUAL_CODEC_CONTROL pure=true complete_frames=5 source_calls=0 world_calls=0");
}
