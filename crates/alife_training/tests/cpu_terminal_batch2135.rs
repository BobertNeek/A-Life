#![cfg(feature = "cpu-backend")]
use alife_training::cpu::*;
#[path = "support/terminal_parent2135.rs"]
mod parent;

// Compile and retain this executable before changing recipe dispatch. Compare
// the exact bytes again afterward; this exercises actual recipe2/3 host code.
#[test]
fn legacy_recipe_continuation_probe() {
    let output = parent::local_output();
    for version in [2, 3] {
        let (mut host, trainer) = parent::fixture(None);
        let config = if version == 2 {
            TerminalContextConfig2135::legacy_v2(16.0).unwrap()
        } else {
            TerminalContextConfig2135::new(16.0).unwrap()
        };
        host.enable_terminal_context2135(config, &trainer).unwrap();
        let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        for index in 0..3 {
            let step = host.step(&trainer, 1.0).unwrap();
            assert_eq!(step.decisions.len(), 1);
            let bytes = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
            let mut cold = CpuCanonicalHost::from_checkpoint(
                serde_json::from_slice(&bytes).unwrap(),
                &trainer,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap(),
                bytes
            );
            assert_eq!(
                serde_json::to_vec(&host.snapshot().step(&trainer, 1.0).unwrap().decisions)
                    .unwrap(),
                serde_json::to_vec(&cold.step(&trainer, 1.0).unwrap().decisions).unwrap()
            );
            if let Some(root) = &output {
                std::fs::write(root.join(format!("v{version}-{index}.cp.json")), bytes).unwrap();
                std::fs::write(
                    root.join(format!("v{version}-{index}.row.json")),
                    serde_json::to_vec(&step.decisions).unwrap(),
                )
                .unwrap();
            }
        }
        assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), founder);
    }
}
