use alife_core::CanonicalDigestBuilder;
use serde::Serialize;

fn named_values_digest(first: u16, second: u32) -> [u64; 4] {
    let mut builder = CanonicalDigestBuilder::new(b"alife.test.field-names.v1");
    builder.write_u16(first);
    builder.write_u32(second);
    builder.finish256()
}

#[test]
fn canonical_builder_has_a_stable_typed_golden_vector() {
    let mut builder = CanonicalDigestBuilder::new(b"alife.test.canonical.v1");
    builder.write_u16(0x1234);
    builder.write_some();
    builder.write_sequence_len(2);
    builder.write_f32(-0.0).unwrap();
    builder.write_f32(1.5).unwrap();
    builder.write_bytes(b"ab");

    assert_eq!(
        builder.finish256(),
        [
            0x8bf8_8789_29d6_7d01,
            0x0ad9_0c8e_d9c3_1691,
            0x64d8_58d4_9290_37f3,
            0xdce4_ce20_dd10_5f23,
        ]
    );
}

#[test]
fn canonical_builder_binds_domain_length_and_order() {
    let mut ordered = CanonicalDigestBuilder::new(b"alife.test.order.v1");
    ordered.write_sequence_len(2);
    ordered.write_u16(1);
    ordered.write_u16(2);

    let mut reversed = CanonicalDigestBuilder::new(b"alife.test.order.v1");
    reversed.write_sequence_len(2);
    reversed.write_u16(2);
    reversed.write_u16(1);

    let mut other_domain = CanonicalDigestBuilder::new(b"alife.test.order.v2");
    other_domain.write_sequence_len(2);
    other_domain.write_u16(1);
    other_domain.write_u16(2);

    let mut split_bytes = CanonicalDigestBuilder::new(b"alife.test.order.v1");
    split_bytes.write_bytes(b"a");
    split_bytes.write_bytes(b"b");
    let mut joined_bytes = CanonicalDigestBuilder::new(b"alife.test.order.v1");
    joined_bytes.write_bytes(b"ab");

    let ordered_digest = ordered.finish256();
    assert_ne!(ordered_digest, reversed.finish256());
    assert_ne!(ordered_digest, other_domain.finish256());
    assert_ne!(split_bytes.finish256(), joined_bytes.finish256());
}

#[test]
fn rust_and_serde_field_names_are_not_digest_inputs() {
    #[derive(Serialize)]
    struct OriginalNames {
        first_value: u16,
        second_value: u32,
    }

    #[derive(Serialize)]
    struct RenamedFields {
        renamed_alpha: u16,
        renamed_beta: u32,
    }

    let original = OriginalNames {
        first_value: 7,
        second_value: 11,
    };
    let renamed = RenamedFields {
        renamed_alpha: 7,
        renamed_beta: 11,
    };

    assert_ne!(
        serde_json::to_string(&original).unwrap(),
        serde_json::to_string(&renamed).unwrap()
    );
    assert_eq!(
        named_values_digest(original.first_value, original.second_value),
        named_values_digest(renamed.renamed_alpha, renamed.renamed_beta)
    );

    let source = include_str!("../src/canonical_digest.rs");
    assert!(!source.contains("use serde"));
    assert!(!source.contains("impl Serializer"));
    assert!(!source.contains("SerializeMap"));
    assert!(!source.contains("write_map("));
    assert!(!source.contains("write_str("));
    assert!(!source.contains("fmt::Display"));
}

#[test]
fn canonical_builder_rejects_non_finite_floats_and_normalizes_signed_zero() {
    let mut positive = CanonicalDigestBuilder::new(b"alife.test.float.v1");
    positive.write_f32(0.0).unwrap();
    let mut negative = CanonicalDigestBuilder::new(b"alife.test.float.v1");
    negative.write_f32(-0.0).unwrap();

    assert_eq!(positive.finish256(), negative.finish256());
    let mut invalid = CanonicalDigestBuilder::new(b"alife.test.float.v1");
    assert!(invalid.write_f32(f32::NAN).is_err());
    assert!(invalid.write_f64(f64::INFINITY).is_err());
}

#[test]
fn canonical_builder_length_prefixes_utf8_and_bytes() {
    let mut builder = CanonicalDigestBuilder::new(b"alife.test.utf8.v1");
    builder.write_utf8("A-Life brain");
    builder.write_bytes(&[0, 1, 255]);

    assert_eq!(
        builder.finish256(),
        [
            0x411d_4b44_5392_3551,
            0x57d5_0b16_16c6_db2c,
            0xecbd_8e9b_dd03_176c,
            0xb1a0_2fa4_3588_49f3,
        ]
    );
}
