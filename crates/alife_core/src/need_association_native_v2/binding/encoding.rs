//! Private V2-only canonical structural encoding. No persistence format changes.
//! Struct/variant names, field names, sequence lengths and scalar types are
//! tagged; floats are finite and signed zero is canonicalized. The expressed
//! phenotype has ordered sequences/structs. Maps are rejected, not order-guessed.

use super::*;
use serde::ser::{self, Serialize};

type Error = NativeNeedAssociationErrorV2;
impl ser::Error for Error {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        Self::Encoding
    }
}
struct Encoder {
    hash: blake3::Hasher,
    values: u64,
    bytes: u64,
}
impl Encoder {
    fn raw(&mut self, bytes: &[u8]) {
        self.hash.update(bytes);
        self.bytes += bytes.len() as u64;
    }
    fn tag(&mut self, tag: u8) {
        self.values += 1;
        self.raw(&[tag]);
    }
    fn len(&mut self, len: usize) {
        self.raw(&(len as u64).to_le_bytes());
    }
    fn text(&mut self, text: &str) {
        self.len(text.len());
        self.raw(text.as_bytes());
    }
    fn end(&mut self) -> Result<(), Error> {
        self.raw(&[255]);
        Ok(())
    }
}
pub(super) fn phenotype_identity(
    phenotype: &CreaturePhenotype,
) -> Result<([u64; 4], NativeNeedBindingWorkV2), Error> {
    let mut encoder = Encoder {
        hash: blake3::Hasher::new(),
        values: 0,
        bytes: 0,
    };
    encoder.text("alife.cpu.need-association.canonical-expressed-phenotype.v2");
    phenotype.serialize(&mut encoder)?;
    let hash = encoder.hash.finalize();
    let identity = std::array::from_fn(|i| {
        u64::from_le_bytes(hash.as_bytes()[i * 8..i * 8 + 8].try_into().unwrap())
    });
    Ok((
        identity,
        NativeNeedBindingWorkV2 {
            phenotype_hash_passes: 1,
            encoded_values: encoder.values,
            encoded_bytes: encoder.bytes,
            ..Default::default()
        },
    ))
}
macro_rules! scalar {
    ($name:ident, $ty:ty, $tag:expr) => {
        fn $name(self, value: $ty) -> Result<(), Error> {
            self.tag($tag);
            self.raw(&value.to_le_bytes());
            Ok(())
        }
    };
}
impl ser::Serializer for &mut Encoder {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;
    fn serialize_bool(self, value: bool) -> Result<(), Error> {
        self.tag(1);
        self.raw(&[u8::from(value)]);
        Ok(())
    }
    scalar!(serialize_i8, i8, 2);
    scalar!(serialize_i16, i16, 3);
    scalar!(serialize_i32, i32, 4);
    scalar!(serialize_i64, i64, 5);
    scalar!(serialize_i128, i128, 6);
    scalar!(serialize_u8, u8, 7);
    scalar!(serialize_u16, u16, 8);
    scalar!(serialize_u32, u32, 9);
    scalar!(serialize_u64, u64, 10);
    scalar!(serialize_u128, u128, 11);
    fn serialize_f32(self, value: f32) -> Result<(), Error> {
        if !value.is_finite() {
            return Err(Error::Encoding);
        }
        self.tag(12);
        self.raw(&(if value == 0. { 0.0 } else { value }).to_le_bytes());
        Ok(())
    }
    fn serialize_f64(self, value: f64) -> Result<(), Error> {
        if !value.is_finite() {
            return Err(Error::Encoding);
        }
        self.tag(13);
        self.raw(&(if value == 0. { 0.0 } else { value }).to_le_bytes());
        Ok(())
    }
    fn serialize_char(self, value: char) -> Result<(), Error> {
        self.tag(14);
        self.raw(&(value as u32).to_le_bytes());
        Ok(())
    }
    fn serialize_str(self, value: &str) -> Result<(), Error> {
        self.tag(15);
        self.text(value);
        Ok(())
    }
    fn serialize_bytes(self, value: &[u8]) -> Result<(), Error> {
        self.tag(16);
        self.len(value.len());
        self.raw(value);
        Ok(())
    }
    fn serialize_none(self) -> Result<(), Error> {
        self.tag(17);
        Ok(())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), Error> {
        self.tag(18);
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Error> {
        self.tag(19);
        Ok(())
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<(), Error> {
        self.tag(20);
        self.text(name);
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
    ) -> Result<(), Error> {
        self.tag(21);
        self.text(name);
        self.raw(&index.to_le_bytes());
        self.text(variant);
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.tag(22);
        self.text(name);
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.tag(23);
        self.text(name);
        self.raw(&index.to_le_bytes());
        self.text(variant);
        value.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self, Error> {
        self.tag(24);
        self.len(len.ok_or(Error::Encoding)?);
        Ok(self)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self, Error> {
        self.tag(25);
        self.len(len);
        Ok(self)
    }
    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self, Error> {
        self.tag(26);
        self.text(name);
        self.len(len);
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self, Error> {
        self.tag(27);
        self.text(name);
        self.raw(&index.to_le_bytes());
        self.text(variant);
        self.len(len);
        Ok(self)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(Error::Encoding)
    }
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self, Error> {
        self.tag(29);
        self.text(name);
        self.len(len);
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self, Error> {
        self.tag(30);
        self.text(name);
        self.raw(&index.to_le_bytes());
        self.text(variant);
        self.len(len);
        Ok(self)
    }
    fn is_human_readable(&self) -> bool {
        false
    }
}
macro_rules! positional {
    ($trait:ident, $method:ident) => {
        impl ser::$trait for &mut Encoder {
            type Ok = ();
            type Error = Error;
            fn $method<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Error> {
                value.serialize(&mut **self)
            }
            fn end(self) -> Result<(), Error> {
                Encoder::end(self)
            }
        }
    };
}
positional!(SerializeSeq, serialize_element);
positional!(SerializeTuple, serialize_element);
positional!(SerializeTupleStruct, serialize_field);
positional!(SerializeTupleVariant, serialize_field);
macro_rules! named {
    ($trait:ident) => {
        impl ser::$trait for &mut Encoder {
            type Ok = ();
            type Error = Error;
            fn serialize_field<T: ?Sized + Serialize>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<(), Error> {
                self.text(key);
                value.serialize(&mut **self)
            }
            fn end(self) -> Result<(), Error> {
                Encoder::end(self)
            }
        }
    };
}
named!(SerializeStruct);
named!(SerializeStructVariant);
