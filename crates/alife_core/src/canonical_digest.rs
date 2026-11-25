//! Typed canonical digest construction.
//!
//! Callers supply a versioned domain and write fields in encoding order.
//! Rust field names and formatting do not enter the identity.

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
    streams: [u64; 4],
    tail: [u8; 8],
    tail_len: u8,
    byte_len: u64,
}

impl CanonicalDigestBuilder {
    pub fn new(domain: &[u8]) -> Self {
        let mut builder = Self {
            streams: SPLITMIX64_SEEDS,
            tail: [0; 8],
            tail_len: 0,
            byte_len: 0,
        };
        builder.write_tagged_len(TAG_DOMAIN, domain.len());
        builder.write_raw(domain);
        builder
    }

    pub fn write_bool(&mut self, value: bool) {
        self.write_raw(&[TAG_BOOL, u8::from(value)]);
    }

    pub fn write_u8(&mut self, value: u8) {
        self.write_raw(&[TAG_U8, value]);
    }

    pub fn write_u16(&mut self, value: u16) {
        let bytes = value.to_le_bytes();
        self.write_raw(&[TAG_U16, bytes[0], bytes[1]]);
    }

    pub fn write_u32(&mut self, value: u32) {
        let bytes = value.to_le_bytes();
        self.write_raw(&[TAG_U32, bytes[0], bytes[1], bytes[2], bytes[3]]);
    }

    pub fn write_u64(&mut self, value: u64) {
        let mut buf = [0u8; 9];
        buf[0] = TAG_U64;
        buf[1..].copy_from_slice(&value.to_le_bytes());
        self.write_raw(&buf);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.write_raw(&[TAG_I8, value as u8]);
    }

    pub fn write_i16(&mut self, value: i16) {
        let bytes = value.to_le_bytes();
        self.write_raw(&[TAG_I16, bytes[0], bytes[1]]);
    }

    pub fn write_i32(&mut self, value: i32) {
        let bytes = value.to_le_bytes();
        self.write_raw(&[TAG_I32, bytes[0], bytes[1], bytes[2], bytes[3]]);
    }

    pub fn write_i64(&mut self, value: i64) {
        let mut buf = [0u8; 9];
        buf[0] = TAG_I64;
        buf[1..].copy_from_slice(&value.to_le_bytes());
        self.write_raw(&buf);
    }

    pub fn write_f32(&mut self, value: f32) -> Result<(), ScaffoldContractError> {
        if !value.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        let bits = if value == 0.0 { 0 } else { value.to_bits() };
        let bytes = bits.to_le_bytes();
        self.write_raw(&[TAG_F32, bytes[0], bytes[1], bytes[2], bytes[3]]);
        Ok(())
    }

    pub fn write_f64(&mut self, value: f64) -> Result<(), ScaffoldContractError> {
        if !value.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        let bits = if value == 0.0 { 0 } else { value.to_bits() };
        let mut buf = [0u8; 9];
        buf[0] = TAG_F64;
        buf[1..].copy_from_slice(&bits.to_le_bytes());
        self.write_raw(&buf);
        Ok(())
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.write_tagged_len(TAG_BYTES, value.len());
        self.write_raw(value);
    }

    pub fn write_utf8(&mut self, value: &str) {
        self.write_tagged_len(TAG_UTF8, value.len());
        self.write_raw(value.as_bytes());
    }

    pub fn write_sequence_len(&mut self, len: usize) {
        self.write_tagged_len(TAG_SEQUENCE, len);
    }

    pub fn write_none(&mut self) {
        self.write_raw(&[TAG_NONE]);
    }

    pub fn write_some(&mut self) {
        self.write_raw(&[TAG_SOME]);
    }

    pub fn finish128(self) -> [u64; 2] {
        let digest = self.finish256();
        [digest[0], digest[1]]
    }

    pub fn finish256(mut self) -> [u64; 4] {
        if self.tail_len > 0 {
            self.absorb_word(u64::from_le_bytes(self.tail));
        }
        self.absorb_word(self.byte_len ^ LENGTH_FINALIZER);
        self.streams
    }

    #[inline]
    fn write_tagged_len(&mut self, tag: u8, len: usize) {
        let len = u64::try_from(len).expect("canonical input length fits in u64");
        let mut buf = [0u8; 9];
        buf[0] = tag;
        buf[1..].copy_from_slice(&len.to_le_bytes());
        self.write_raw(&buf);
    }

    #[inline]
    fn absorb_word(&mut self, word: u64) {
        for stream in &mut self.streams {
            *stream = splitmix64(*stream ^ word);
        }
    }

    fn write_raw(&mut self, mut input: &[u8]) {
        let add = u64::try_from(input.len()).expect("canonical input length fits in u64");
        self.byte_len = self
            .byte_len
            .checked_add(add)
            .expect("canonical input length fits in u64");
        if self.tail_len > 0 {
            let tail_len = usize::from(self.tail_len);
            let need = 8 - tail_len;
            if input.len() < need {
                self.tail[tail_len..tail_len + input.len()].copy_from_slice(input);
                self.tail_len += input.len() as u8;
                return;
            }
            self.tail[tail_len..8].copy_from_slice(&input[..need]);
            self.absorb_word(u64::from_le_bytes(self.tail));
            self.tail = [0; 8];
            self.tail_len = 0;
            input = &input[need..];
        }
        let mut chunks = input.chunks_exact(8);
        for chunk in chunks.by_ref() {
            let mut word = [0u8; 8];
            word.copy_from_slice(chunk);
            self.absorb_word(u64::from_le_bytes(word));
        }
        let rem = chunks.remainder();
        if !rem.is_empty() {
            self.tail[..rem.len()].copy_from_slice(rem);
            self.tail_len = rem.len() as u8;
        }
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
