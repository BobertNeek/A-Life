//! Canonical BLAKE3-256 identities for durable neural ABIs.

use serde::{Deserialize, Serialize};

/// A canonical 256-bit digest whose algorithm is part of the public contract.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Blake3Digest([u8; 32]);

impl Blake3Digest {
    pub(crate) fn from_hasher(hasher: blake3::Hasher) -> Self {
        Self(*hasher.finalize().as_bytes())
    }

    pub const fn bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub const fn algorithm(&self) -> &'static str {
        "BLAKE3-256"
    }
}

pub(crate) fn domain_hasher(domain: &[u8]) -> blake3::Hasher {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&(domain.len() as u64).to_le_bytes());
    hasher.update(domain);
    hasher
}

pub(crate) trait Blake3Write {
    fn write_u8(&mut self, value: u8);
    fn write_u16(&mut self, value: u16);
    fn write_u32(&mut self, value: u32);
    fn write_u64(&mut self, value: u64);
    fn write_len(&mut self, value: usize);
}

impl Blake3Write for blake3::Hasher {
    fn write_u8(&mut self, value: u8) {
        self.update(&[value]);
    }

    fn write_u16(&mut self, value: u16) {
        self.update(&value.to_le_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.update(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.update(&value.to_le_bytes());
    }

    fn write_len(&mut self, value: usize) {
        self.write_u64(value as u64);
    }
}
