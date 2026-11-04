//! Compact lossless transport inside disposable experiments. Original native
//! bytes and their hashes remain unchanged; no optimizer values are converted.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{de::Error, Deserialize, Deserializer, Serializer};
pub(super) mod blob {
    use super::*;
    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(bytes))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        STANDARD
            .decode(String::deserialize(d)?)
            .map_err(D::Error::custom)
    }
}
pub(super) mod files {
    use super::*;
    use std::collections::BTreeMap;
    pub fn serialize<S: Serializer>(
        files: &BTreeMap<String, Vec<u8>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(Some(files.len()))?;
        for (name, bytes) in files {
            map.serialize_entry(name, &STANDARD.encode(bytes))?;
        }
        map.end()
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<String, Vec<u8>>, D::Error> {
        BTreeMap::<String, String>::deserialize(d)?
            .into_iter()
            .map(|(name, value)| {
                STANDARD
                    .decode(value)
                    .map(|bytes| (name, bytes))
                    .map_err(D::Error::custom)
            })
            .collect()
    }
}
