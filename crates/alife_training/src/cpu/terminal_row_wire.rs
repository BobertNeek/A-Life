//! Diagnostic research rows cannot masquerade as ordinary trainer inputs.
use serde::{Serialize, Serializer};

pub(super) fn serialize<T: Serialize + ?Sized, S: Serializer>(
    flat: &T,
    research: bool,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if research {
        #[derive(Serialize)]
        struct Research<'a, T: ?Sized> {
            terminal_context2135_research_record_v1: &'a T,
        }
        Research {
            terminal_context2135_research_record_v1: flat,
        }
        .serialize(serializer)
    } else {
        flat.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Flat {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        terminal_context2135: Option<f32>,
        organism_id: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sampling_counter: Option<u32>,
        features: Vec<f32>,
    }
    #[derive(Deserialize)]
    struct OldFlat {
        #[allow(dead_code)]
        organism_id: u64,
    }
    struct Wire<'a>(&'a Flat);
    impl Serialize for Wire<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            #[derive(Serialize)]
            struct Borrowed<'a> {
                #[serde(skip_serializing_if = "Option::is_none")]
                terminal_context2135: &'a Option<f32>,
                organism_id: &'a u64,
                #[serde(skip_serializing_if = "Option::is_none")]
                sampling_counter: &'a Option<u32>,
                features: &'a Vec<f32>,
            }
            let flat = Borrowed {
                terminal_context2135: &self.0.terminal_context2135,
                organism_id: &self.0.organism_id,
                sampling_counter: &self.0.sampling_counter,
                features: &self.0.features,
            };
            super::serialize(&flat, self.0.terminal_context2135.is_some(), s)
        }
    }
    #[test]
    fn off_is_exact_flat_bytes_with_original_order_and_omissions() {
        for sampling_counter in [None, Some(7)] {
            let row = Flat {
                terminal_context2135: None,
                organism_id: 5,
                sampling_counter,
                features: vec![1., -2.],
            };
            let before = serde_json::to_vec(&row).unwrap();
            assert_eq!(serde_json::to_vec(&Wire(&row)).unwrap(), before);
            let decoded: Flat = serde_json::from_slice(&before).unwrap();
            assert_eq!(decoded, row);
            assert!(!String::from_utf8(before)
                .unwrap()
                .contains("terminal_context2135"));
        }
    }
    #[test]
    fn enabled_complete_payload_is_not_an_ordinary_row_even_at_zero() {
        for delta in [0., f32::MIN_POSITIVE, 0.25] {
            let row = Flat {
                terminal_context2135: Some(delta),
                organism_id: 5,
                sampling_counter: Some(7),
                features: vec![1., -2.],
            };
            let before = serde_json::to_vec(&row).unwrap();
            let bytes = serde_json::to_vec(&Wire(&row)).unwrap();
            let mut expected = b"{\"terminal_context2135_research_record_v1\":".to_vec();
            expected.extend_from_slice(&before);
            expected.push(b'}');
            assert_eq!(bytes, expected);
            let envelope: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(envelope.as_object().unwrap().len(), 1);
            assert_eq!(
                envelope["terminal_context2135_research_record_v1"],
                serde_json::from_slice::<serde_json::Value>(&before).unwrap()
            );
            assert!(serde_json::from_slice::<Flat>(&bytes).is_err());
            assert!(serde_json::from_slice::<OldFlat>(&bytes).is_err());
            assert_eq!(serde_json::to_vec(&row).unwrap(), before);
        }
    }
}
