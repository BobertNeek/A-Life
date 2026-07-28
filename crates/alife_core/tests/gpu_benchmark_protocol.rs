use alife_core::GpuClosedLoopBenchmarkProtocolV1;

#[test]
fn canonical_gpu_benchmark_protocol_uses_the_producer_wire_digest() {
    let protocol = GpuClosedLoopBenchmarkProtocolV1::canonical();

    assert_eq!(
        protocol.protocol_digest,
        [
            8_689_651_408_761_950_731,
            14_393_468_810_155_734_722,
            16_126_579_699_626_035_972,
            2_563_444_024_877_389_873,
        ]
    );
    assert!(protocol.is_canonical());

    let wire = serde_json::to_value(protocol).unwrap();
    let decoded: GpuClosedLoopBenchmarkProtocolV1 = serde_json::from_value(wire).unwrap();
    assert!(decoded.is_canonical());

    let mut tampered = decoded;
    tampered.measured_ticks -= 1;
    assert!(!tampered.is_canonical());
}
