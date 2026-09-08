//! One bounded V2 choice probe using the unchanged V1 trained genetic weights.
use super::nociception_food_tests::{assert_choices, run_food_life};
use super::*;
use alife_core::{ActionCandidateCreditProfileV1, Nano512ActionCreditCandidateV2};

#[test]
#[ignore = "bounded manual V2 probe; requires retained ALIFE_FOUNDER_CANDIDATE"]
fn inherited_action_credit_food_choice_probe() {
    let bytes = std::fs::read(std::env::var("ALIFE_FOUNDER_CANDIDATE").unwrap()).unwrap();
    let source = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(
        source.digest().bytes(),
        &[
            14, 148, 152, 50, 97, 32, 184, 207, 71, 123, 153, 74, 88, 1, 122, 247, 18, 136, 157,
            226, 80, 112, 127, 203, 223, 222, 92, 5, 5, 204, 232, 246
        ]
    );
    let configured = Nano512ActionCreditCandidateV2::new(
        &source,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
    assert_eq!(
        configured.asset().unwrap().encode_canonical().unwrap(),
        bytes
    );
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&configured).unwrap();
    for (synapse, weight) in phenotype.synapses().iter().zip(source.weights()) {
        assert_eq!(synapse.genetic_weight().to_bits(), weight.to_bits());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/action-credit-food-{}",
        std::process::id()
    ));
    assert!(!root.exists(), "preserve prior evidence");
    std::fs::create_dir_all(&root).unwrap();
    for (name, value) in [
        (
            "configured-candidate.json",
            serde_json::to_value(&configured).unwrap(),
        ),
        (
            "neural-phenotype.json",
            serde_json::to_value(&phenotype).unwrap(),
        ),
        (
            "compiler-inputs.json",
            serde_json::to_value(&inputs).unwrap(),
        ),
    ] {
        std::fs::write(root.join(name), serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
    let harmful = run_food_life(
        &source,
        &phenotype,
        true,
        &root.join("cyan-nutritious"),
        Some(&configured),
    );
    let nutritious = run_food_life(
        &source,
        &phenotype,
        false,
        &root.join("amber-nutritious"),
        Some(&configured),
    );
    std::fs::write(
        root.join("outcomes.json"),
        serde_json::to_vec_pretty(&[&harmful, &nutritious]).unwrap(),
    )
    .unwrap();
    println!(
        "action_credit_choices: {harmful:?}; {nutritious:?}; evidence={}",
        root.display()
    );
    assert_choices(&harmful, &nutritious);
}
