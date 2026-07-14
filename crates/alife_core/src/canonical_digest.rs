//! Contract-only typed canonical digest byte construction.
//!
//! Callers choose a versioned domain and write every field in contract order.
//! The builder exposes no reflective struct, field-name, map, or formatting
//! surface, so language-level names cannot become causal identity inputs.

use crate::ScaffoldContractError;

const TAG_BOOL: u8 = 0x01;
const TAG_U8: u8 = 0x02;
const TAG_U16: u8 = 0x03;
const TAG_U32: u8 = 0x04;
const TAG_U64: u8 = 0x05;
const TAG_I8: u8 = 0x06;
const TAG_I16: u8 = 0x07;
const TAG_I32: u8 = 0x08;
const TAG_I64: u8 = 0x09;
const TAG_F32: u8 = 0x0a;
const TAG_F64: u8 = 0x0b;
const TAG_BYTES: u8 = 0x0c;
const TAG_SEQUENCE: u8 = 0x0d;
const TAG_NONE: u8 = 0x0e;
const TAG_SOME: u8 = 0x0f;
const TAG_UTF8: u8 = 0x10;
const TAG_DOMAIN: u8 = 0xd0;

const SPLITMIX64_SEEDS: [u64; 4] = [
    0xa11f_ea7e_d00d_0001,
    0xc0de_cafe_51a7_0002,
    0x9e37_79b9_7f4a_7c15,
    0xd1b5_4a32_d192_ed03,
];
const LENGTH_FINALIZER: u64 = 0xf1a1_d165_e57a_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalDigestBuilder {
    bytes: Vec<u8>,
}

impl CanonicalDigestBuilder {
    pub fn new(domain: &[u8]) -> Self {
        let mut builder = Self { bytes: Vec::new() };
        builder.bytes.push(TAG_DOMAIN);
        builder.write_raw_len(domain.len());
        builder.bytes.extend_from_slice(domain);
        builder
    }

    pub fn write_bool(&mut self, value: bool) {
        self.bytes.push(TAG_BOOL);
        self.bytes.push(u8::from(value));
    }

    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(TAG_U8);
        self.bytes.push(value);
    }

    pub fn write_u16(&mut self, value: u16) {
        self.bytes.push(TAG_U16);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u32(&mut self, value: u32) {
        self.bytes.push(TAG_U32);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u64(&mut self, value: u64) {
        self.bytes.push(TAG_U64);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_i8(&mut self, value: i8) {
        self.bytes.push(TAG_I8);
        self.bytes.push(value as u8);
    }

    pub fn write_i16(&mut self, value: i16) {
        self.bytes.push(TAG_I16);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.bytes.push(TAG_I32);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.bytes.push(TAG_I64);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_f32(&mut self, value: f32) -> Result<(), ScaffoldContractError> {
        if !value.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        self.bytes.push(TAG_F32);
        let bits = if value == 0.0 { 0 } else { value.to_bits() };
        self.bytes.extend_from_slice(&bits.to_le_bytes());
        Ok(())
    }

    pub fn write_f64(&mut self, value: f64) -> Result<(), ScaffoldContractError> {
        if !value.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        self.bytes.push(TAG_F64);
        let bits = if value == 0.0 { 0 } else { value.to_bits() };
        self.bytes.extend_from_slice(&bits.to_le_bytes());
        Ok(())
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.bytes.push(TAG_BYTES);
        self.write_raw_len(value.len());
        self.bytes.extend_from_slice(value);
    }

    pub fn write_utf8(&mut self, value: &str) {
        self.bytes.push(TAG_UTF8);
        self.write_raw_len(value.len());
        self.bytes.extend_from_slice(value.as_bytes());
    }

    pub fn write_sequence_len(&mut self, len: usize) {
        self.bytes.push(TAG_SEQUENCE);
        self.write_raw_len(len);
    }

    pub fn write_none(&mut self) {
        self.bytes.push(TAG_NONE);
    }

    pub fn write_some(&mut self) {
        self.bytes.push(TAG_SOME);
    }

    pub fn finish128(self) -> [u64; 2] {
        let digest = self.finish256();
        [digest[0], digest[1]]
    }

    pub fn finish256(self) -> [u64; 4] {
        let byte_len = u64::try_from(self.bytes.len()).expect("canonical input length fits in u64");
        let mut streams = SPLITMIX64_SEEDS;
        for chunk in self.bytes.chunks(8) {
            let mut padded = [0u8; 8];
            padded[..chunk.len()].copy_from_slice(chunk);
            let word = u64::from_le_bytes(padded);
            for stream in &mut streams {
                *stream = splitmix64(*stream ^ word);
            }
        }
        let length_word = byte_len ^ LENGTH_FINALIZER;
        for stream in &mut streams {
            *stream = splitmix64(*stream ^ length_word);
        }
        streams
    }

    fn write_raw_len(&mut self, len: usize) {
        let len = u64::try_from(len).expect("canonical input length fits in u64");
        self.bytes.extend_from_slice(&len.to_le_bytes());
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
