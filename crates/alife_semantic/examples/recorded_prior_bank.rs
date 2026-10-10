//! Offline export/inspection of existing captured semantic-prior responses.
//! This example cannot initialize an inference provider or generate outputs.

#[cfg(feature = "local-llamacpp")]
mod offline {
    use std::{fs::File, io::Read, path::Path};

    use alife_semantic::{RecordedPriorBank, RecordedPriorOrigin, MAX_RECORDED_PRIOR_BANK_BYTES};

    const MAX_ORIGIN_BYTES: usize = 4096;

    fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|error| format!("cannot open {}: {error}", path.display()))?
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if bytes.len() > limit {
            return Err(format!("{} exceeds {limit} bytes", path.display()));
        }
        Ok(bytes)
    }

    fn export(
        cache_path: &Path,
        origin_path: &Path,
        expected_context_contract: &str,
        bank_path: &Path,
    ) -> Result<usize, String> {
        let origin: RecordedPriorOrigin =
            serde_json::from_slice(&read_bounded(origin_path, MAX_ORIGIN_BYTES)?)
                .map_err(|error| format!("invalid source capture origin JSON: {error}"))?;
        let bank = RecordedPriorBank::from_cache_json_slice(
            origin,
            &read_bounded(cache_path, MAX_RECORDED_PRIOR_BANK_BYTES)?,
            expected_context_contract,
        )?;
        bank.save(bank_path)?;
        Ok(bank.entries().len())
    }

    pub fn run(args: &[String]) -> Result<(), String> {
        match args {
            [command, cache, origin, contract, bank] if command == "export" => {
                let entries = export(Path::new(cache), Path::new(origin), contract, Path::new(bank))?;
                println!("Exported {entries} existing captured responses to {bank}; no inference was performed.");
                println!("Source provenance is caller-declared; this export does not establish recording authenticity or neural use.");
                Ok(())
            }
            [command, path, contract] if command == "inspect" => {
                let bank = RecordedPriorBank::load(path, contract)?;
                println!("Valid frozen bank: {} exact contexts", bank.entries().len());
                println!("Source provider: {}", bank.origin().provider_identity);
                println!("Declared source model: {}", bank.origin().model);
                println!("Declared model SHA256: {}", bank.origin().model_sha256);
                println!("Context contract: {}", bank.origin().context_contract);
                println!("Validation checks compatibility and bounds, not recording authenticity or neural use.");
                Ok(())
            }
            _ => Err(concat!(
                "usage: recorded_prior_bank export <captured-cache.json> <capture-origin.json> ",
                "<expected-context-contract> <new-bank.json>\n",
                "       recorded_prior_bank inspect <bank.json> <expected-context-contract>\n",
                "Origin JSON must contain provider_identity, model, model_sha256, context_contract ",
                "copied from the actual source capture/runtime receipt. Use unverified-model only ",
                "when the source model digest was not verified. Export refuses existing files."
            )
            .into()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use alife_semantic::parse_slm_prior_json;

        fn fixture_directory() -> std::path::PathBuf {
            let path = std::env::temp_dir().join(format!(
                "alife-recorded-prior-export-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&path).unwrap();
            path
        }

        fn write_synthetic_inputs(directory: &Path) {
            let origin = RecordedPriorOrigin {
                provider_identity: "synthetic-cpu-export-fixture".into(),
                model: "synthetic-fixture-model".into(),
                model_sha256: "unverified-model".into(),
                context_contract: "synthetic-context-contract".into(),
            };
            std::fs::write(
                directory.join("origin.json"),
                serde_json::to_vec(&origin).unwrap(),
            )
            .unwrap();
            let zero = parse_slm_prior_json(
                &origin.model,
                r#"{"salience_labels":["food"],"context_summary":"heard food","lexicon_associations":[{"token":"food","salience":0.0}],"perception_tags":["heard"]}"#,
            ).unwrap();
            let cache = std::collections::BTreeMap::from([("heard words food", zero)]);
            std::fs::write(
                directory.join("cache.json"),
                serde_json::to_vec(&cache).unwrap(),
            )
            .unwrap();
        }

        #[test]
        fn offline_export_preserves_inputs_zero_outputs_and_refuses_replacement() {
            let directory = fixture_directory();
            write_synthetic_inputs(&directory);
            let cache = directory.join("cache.json");
            let origin = directory.join("origin.json");
            let bank = directory.join("bank.json");
            let original_cache = std::fs::read(&cache).unwrap();
            assert_eq!(
                export(&cache, &origin, "synthetic-context-contract", &bank).unwrap(),
                1
            );
            let loaded = RecordedPriorBank::load(&bank, "synthetic-context-contract").unwrap();
            assert_eq!(
                loaded
                    .lookup("heard words food")
                    .unwrap()
                    .lexicon_associations[0]
                    .salience,
                0.0
            );
            assert!(loaded.lookup("different context").is_none());
            let original_bank = std::fs::read(&bank).unwrap();
            assert!(export(&cache, &origin, "synthetic-context-contract", &bank).is_err());
            assert_eq!(std::fs::read(&cache).unwrap(), original_cache);
            assert_eq!(std::fs::read(&bank).unwrap(), original_bank);
            std::fs::remove_dir_all(directory).unwrap();
        }

        #[test]
        fn failed_offline_export_never_writes_an_output() {
            let directory = fixture_directory();
            write_synthetic_inputs(&directory);
            let cache = directory.join("cache.json");
            let origin = directory.join("origin.json");
            let bank = directory.join("bank.json");
            assert!(export(&cache, &origin, "incompatible-context-contract", &bank).is_err());
            assert!(!bank.exists());
            std::fs::write(&cache, vec![b' '; MAX_RECORDED_PRIOR_BANK_BYTES + 1]).unwrap();
            assert!(export(&cache, &origin, "synthetic-context-contract", &bank).is_err());
            assert!(!bank.exists());
            std::fs::remove_dir_all(directory).unwrap();
        }
    }
}

#[cfg(feature = "local-llamacpp")]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = offline::run(&args) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(not(feature = "local-llamacpp"))]
fn main() {
    eprintln!("recorded_prior_bank requires --features local-llamacpp; it performs no inference");
    std::process::exit(1);
}
