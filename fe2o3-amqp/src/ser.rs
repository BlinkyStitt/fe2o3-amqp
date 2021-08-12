use std::io::Write;

use serde::{
    ser::{self, SerializeSeq, SerializeStruct},
    Serialize,
};

use crate::{
    constructor::EncodingCodes, descriptor::DESCRIPTOR, error::Error, types::SYMBOL_MAGIC,
    value::U32_MAX_AS_USIZE,
    described::{DESCRIBED_BASIC, DESCRIBED_LIST, DESCRIBED_MAP}
};

pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, Error>
where
    T: Serialize,
{
    let mut writer = Vec::new(); // TODO: pre-allocate capacity
    let mut serializer = Serializer::new(&mut writer);
    value.serialize(&mut serializer)?;
    Ok(writer)
}

#[repr(u8)]
enum SerializerState {
    None,
    Symbol,
    DescribedList,
    DescribedMap,
    DescribedBasic,
}

impl Default for SerializerState {
    fn default() -> Self {
        Self::None
    }
}

pub struct Serializer<W> {
    writer: W,
    state: SerializerState,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            state: Default::default(),
        }
    }

    pub fn into_inner(self) -> W {
        self.writer
    }

    pub fn none(writer: W) -> Self {
        Self { 
            writer, 
            state: SerializerState::None
        }
    }

    pub fn symbol(writer: W) -> Self {
        Self {
            writer,
            state: SerializerState::Symbol
        }
    }

    pub fn described_list(writer: W) -> Self {
        Self {
            writer,
            state: SerializerState::DescribedList
        }
    }

    pub fn described_map(writer: W) -> Self {
        Self {
            writer,
            state: SerializerState::DescribedMap
        }
    }

    pub fn described_basic(writer: W) -> Self {
        Self {
            writer,
            state: SerializerState::DescribedBasic
        }
    }
}

impl<'a, W: Write + 'a> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Compound<'a, W>;
    type SerializeTuple = Compound<'a, W>;
    type SerializeMap = Compound<'a, W>;
    type SerializeTupleStruct = StructSerialzier<'a, W>;
    type SerializeStruct = StructSerialzier<'a, W>;
    type SerializeStructVariant = VariantSerializer<'a, W>;
    type SerializeTupleVariant = VariantSerializer<'a, W>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        match v {
            true => {
                let buf = [EncodingCodes::BooleanTrue as u8];
                self.writer.write_all(&buf)
            }
            false => {
                let buf = [EncodingCodes::BooleanFalse as u8];
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        let buf = [EncodingCodes::Byte as u8, v as u8];
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Short as u8];
        self.writer.write_all(&code)?;
        let buf = v.to_be_bytes();
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        match v {
            val @ -128..=127 => {
                let buf = [EncodingCodes::SmallInt as u8, val as u8];
                self.writer.write_all(&buf)
            }
            val @ _ => {
                let code = [EncodingCodes::Int as u8];
                self.writer.write_all(&code)?;
                let buf: [u8; 4] = val.to_be_bytes();
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        match v {
            val @ -128..=127 => {
                let buf = [EncodingCodes::SmallLong as u8, val as u8];
                self.writer.write_all(&buf)
            }
            val @ _ => {
                let code = [EncodingCodes::Long as u8];
                self.writer.write_all(&code)?;
                let buf: [u8; 8] = val.to_be_bytes();
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        let buf = [EncodingCodes::Ubyte as u8, v];
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Ushort as u8];
        let buf: [u8; 2] = v.to_be_bytes();
        self.writer.write_all(&code)?;
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        match v {
            // uint0
            0 => {
                let buf = [EncodingCodes::Uint0 as u8];
                self.writer.write_all(&buf)
            }
            // smalluint
            val @ 1..=255 => {
                let buf = [EncodingCodes::SmallUint as u8, val as u8];
                self.writer.write_all(&buf)
            }
            // uint
            val @ _ => {
                let code = [EncodingCodes::Uint as u8];
                let buf: [u8; 4] = val.to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        match v {
            // ulong0
            0 => {
                let buf = [EncodingCodes::Ulong0 as u8];
                self.writer.write_all(&buf)
            }
            // small ulong
            val @ 1..=255 => {
                let buf = [EncodingCodes::SmallUlong as u8, val as u8];
                self.writer.write_all(&buf)
            }
            // ulong
            val @ _ => {
                let code = [EncodingCodes::Ulong as u8];
                let buf: [u8; 8] = val.to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&buf)
            }
        }
        .map_err(Into::into)
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Float as u8];
        let buf = v.to_be_bytes();
        self.writer.write_all(&code)?;
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Double as u8];
        let buf = v.to_be_bytes();
        self.writer.write_all(&code)?;
        self.writer.write_all(&buf).map_err(Into::into)
    }

    // `char` in rust is a subset of the unicode code points and
    // can be directly treated as u32
    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        let code = [EncodingCodes::Char as u8];
        let buf = (v as u32).to_be_bytes();
        self.writer.write_all(&code)?;
        self.writer.write_all(&buf).map_err(Into::into)
    }

    // String slices are always valid utf-8
    // `String` isd utf-8 encoded
    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        let l = v.len();
        match self.state {
            SerializerState::Symbol => {
                match l {
                    // sym8
                    0..=255 => {
                        let code = [EncodingCodes::Sym8 as u8, l as u8];
                        self.writer.write_all(&code)?;
                    }
                    256..=U32_MAX_AS_USIZE => {
                        let code = [EncodingCodes::Sym32 as u8];
                        let width = (l as u32).to_be_bytes();
                        self.writer.write_all(&code)?;
                        self.writer.write_all(&width)?;
                    }
                    _ => return Err(Error::Message("Too long".into())),
                }
                self.state = SerializerState::None;
            },
            _ => {
                match l {
                    // str8-utf8
                    0..=255 => {
                        let code = [EncodingCodes::Str8 as u8, l as u8];
                        // let width: [u8; 1] = (l as u8).to_be_bytes();
                        self.writer.write_all(&code)?;
                        // self.writer.write_all(&width)?;
                    }
                    // str32-utf8
                    256..=U32_MAX_AS_USIZE => {
                        let code = [EncodingCodes::Str32 as u8];
                        let width: [u8; 4] = (l as u32).to_be_bytes();
                        self.writer.write_all(&code)?;
                        self.writer.write_all(&width)?;
                    }
                    _ => return Err(Error::Message("Too long".into())),
                }
            }
        }

        self.writer.write_all(v.as_bytes()).map_err(Into::into)
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let l = v.len();
        match l {
            // vbin8
            0..=255 => {
                let code = [EncodingCodes::VBin8 as u8];
                let width: [u8; 1] = (l as u8).to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&width)?;
            }
            // vbin32
            256..=U32_MAX_AS_USIZE => {
                let code = [EncodingCodes::VBin32 as u8];
                let width: [u8; 4] = (l as u32).to_be_bytes();
                self.writer.write_all(&code)?;
                self.writer.write_all(&width)?;
            }
            _ => return Err(Error::Message("Too long".into())),
        }
        self.writer.write_all(v).map_err(Into::into)
    }

    // None is serialized as Bson::Null in BSON
    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        let buf = [EncodingCodes::Null as u8];
        self.writer.write_all(&buf).map_err(Into::into)
    }

    #[inline]
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        // Some(T) is serialized simply as if it is T in BSON
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // unit is serialized as Bson::Null in BSON
        let buf = [EncodingCodes::Null as u8];
        self.writer.write_all(&buf).map_err(Into::into)
    }

    // JSON, BSOM, AVRO all serialized to unit
    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    // Serialize into tag
    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(variant_index)
    }

    // Treat newtype structs as insignificant wrappers around the data they contain
    //
    // Serialization of `Symbol`
    // - The serializer will determine whether name == SYMBOL_MAGIC
    // - If a Symbol is to be serialized, it will write a Symbol constructor and modify it's internal state to Symbol
    #[inline]
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        if name == SYMBOL_MAGIC {
            self.state = SerializerState::Symbol;
        }
        value.serialize(self)
    }

    // Treat newtype variant as insignificant wrappers around the data they contain
    #[inline]
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    // A variably sized heterogeneous sequence of values
    //
    // This will be encoded as primitive type `List`
    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        match len {
            Some(num) => Ok(Compound::new(self, num)),
            None => Err(Error::Message("Length must be known".into())),
        }
    }

    // A statically sized heterogeneous sequence of values
    // for which the length will be known at deserialization
    // time without looking at the serialized data
    //
    // This will be encoded as primitive type `List`
    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    // A named tuple, for example `struct Rgb(u8, u8, u8)`
    //
    // The tuple struct looks rather like a rust-exclusive data type.
    // Thus this will be treated the same as a tuple (as in JSON and AVRO)
    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        // self.serialize_seq(Some(len))
        unimplemented!()
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        match len {
            Some(num) => Ok(Compound::new(self, num * 2)),
            None => Err(Error::Message("Length must be known".into())),
        }
    }

    // The serde data model treats struct as "A statically sized heterogeneous key-value pairing"
    //
    // Naive impl: serialize into a list (because Composite types in AMQP is serialized as
    // a described list)
    //
    // Can be configured to serialize into a map or list when wrapped with `Described`
    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        // if name == DESCRIBED_TYPE {
        //     let code = [EncodingCodes::DescribedType as u8];
        //     self.writer.write_all(&code)?;
        //     // The field following is a descriptor, which should be directly written to the writer
        //     Ok(StructSerialzier::described(self, len))
        // } else if name == DESCRIPTOR {
        //     // The field in `Descriptor` should be treated as value
        //     Ok(StructSerialzier::descriptor(self, len))
        // } else if name == DESCRIBED_VALUE_LIST {
        //     // The value will be serialized as a List, and must be serialized onto a separate buffer first
        //     Ok(StructSerialzier::list_value(self, len))
        // } else if name == DESCRIBED_VALUE_MAP {
        //     // The value will be serialized as a Map, and must be serialized onto a separate buffer first
        //     Ok(StructSerialzier::map_value(self, len))
        // } else {
        //     // Other primitive types
        //     Ok(StructSerialzier::basic_value(self, len))
        // }
        match self.state {
            // A None state indicates a freshly instantiated serializer
            SerializerState::None => {
                if name == DESCRIBED_LIST {
                    self.state = SerializerState::DescribedList;
                    let code = [EncodingCodes::DescribedType as u8];
                    self.writer.write_all(&code)?;
                    Ok(StructSerialzier::list_value(self, len))
                } else if name == DESCRIBED_MAP {
                    self.state = SerializerState::DescribedMap;
                    let code = [EncodingCodes::DescribedType as u8];
                    self.writer.write_all(&code)?;
                    Ok(StructSerialzier::map_value(self, len))
                } else if name == DESCRIBED_BASIC {
                    self.state = SerializerState::DescribedBasic;
                    let code = [EncodingCodes::DescribedType as u8];
                    self.writer.write_all(&code)?;
                    Ok(StructSerialzier::basic_value(self, len))
                } else if name == DESCRIPTOR {
                    Ok(StructSerialzier::descriptor(self, len))
                } else {
                    // Only non-described struct will go to this branch
                    Ok(StructSerialzier::list_value(self, len))
                }
            },
            SerializerState::DescribedBasic => {
                Ok(StructSerialzier::basic_value(self, len))
            },
            SerializerState::DescribedList => {
                Ok(StructSerialzier::list_value(self, len))
            },
            SerializerState::DescribedMap => {
                Ok(StructSerialzier::map_value(self, len))
            },
            SerializerState::Symbol => unreachable!()
        }
    }

    // Treat this as if it is a tuple because this kind of enum is unique in rust
    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(VariantSerializer::new(
            self,
            name,
            variant_index,
            variant,
            len,
        ))
    }

    // Treat it as if it is a struct because this is not found in other languages
    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(VariantSerializer::new(
            self,
            name,
            variant_index,
            variant,
            len,
        ))
    }
}

/// Serializer for Compound types. Compound types include:
/// 1. List
/// 2. Map
pub struct Compound<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    num: usize,   // number of element
    buf: Vec<u8>, // byte buffer
}

impl<'a, W: 'a> Compound<'a, W> {
    fn new(se: &'a mut Serializer<W>, num: usize) -> Self {
        Self {
            se,
            num,
            buf: Vec::new(),
        }
    }
}

// This requires some hacking way of getting the constructor (EncodingCode)
// for the type. Use TypeId?
//
// Serialize into a List
// List requires knowing the total number of bytes after serialized
impl<'a, W: Write + 'a> ser::SerializeSeq for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut serializer = Serializer::new(&mut self.buf);
        value.serialize(&mut serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        let Self { se, num, buf } = self;
        write_seq(&mut se.writer, num, buf)
    }
}

fn write_seq<'a, W: Write + 'a>(writer: &'a mut W, num: usize, buf: Vec<u8>) -> Result<(), Error> {
    let len = buf.len();

    // if `len` < 255, `num` must be smaller than 255
    match len {
        0 => {
            let code = [EncodingCodes::List0 as u8];
            writer.write_all(&code)?;
        }
        1..=255 => {
            let code = [EncodingCodes::List8 as u8, len as u8, num as u8];
            writer.write_all(&code)?;
            writer.write_all(&buf)?;
        }
        256..=U32_MAX_AS_USIZE => {
            let code = [EncodingCodes::List32 as u8];
            let len: [u8; 4] = (len as u32).to_be_bytes();
            let num: [u8; 4] = (num as u32).to_be_bytes();
            writer.write_all(&code)?;
            writer.write_all(&len)?;
            writer.write_all(&num)?;
            writer.write_all(&buf)?;
        }
        _ => return Err(Error::Message("Too long".into())),
    }
    Ok(())
}

// Serialize into a List
// List requires knowing the total number of bytes after serialized
impl<'a, W: Write + 'a> ser::SerializeTuple for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        <Self as SerializeSeq>::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        <Self as SerializeSeq>::end(self)
    }
}

// Map is a compound type and thus requires knowing the size of the total number
// of bytes
impl<'a, W: Write + 'a> ser::SerializeMap for Compound<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        let mut serializer = Serializer::new(&mut self.buf);
        key.serialize(&mut serializer)?;
        value.serialize(&mut serializer)?;
        Ok(())
    }

    #[inline]
    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unreachable!()
    }

    #[inline]
    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unreachable!()
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        let Self { se, num, buf } = self;
        write_map(&mut se.writer, num, buf)
    }
}

fn write_map<'a, W: Write + 'a>(writer: &'a mut W, num: usize, buf: Vec<u8>) -> Result<(), Error> {
    let len = buf.len();

    match len {
        0..=255 => {
            let code = [EncodingCodes::Map8 as u8, len as u8, num as u8];
            writer.write_all(&code)?;
            writer.write_all(&buf)?;
        }
        256..=U32_MAX_AS_USIZE => {
            let code = [EncodingCodes::Map32 as u8];
            let len = (len as u32).to_be_bytes();
            let num = (num as u32).to_be_bytes();
            writer.write_all(&code)?;
            writer.write_all(&len)?;
            writer.write_all(&num)?;
            writer.write_all(&buf)?;
        }
        _ => return Err(Error::Message("Too long".into())),
    }
    Ok(())
}

pub enum StructSerializerType {
    Described,
    Descriptor,
    BasicValue,
    ListValue,
    MapValue,
    // BasicValue,
}

pub struct StructSerialzier<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    state: StructSerializerType,
    num: usize,
    buf: Vec<u8>,
}

impl<'a, W: 'a> StructSerialzier<'a, W> {
    pub fn described(se: &'a mut Serializer<W>, num: usize) -> Self {
        Self {
            se,
            state: StructSerializerType::Described,
            num,
            buf: Vec::new(), // TODO: this should be ommitable by allocating with capacity 0
        }
    }

    pub fn descriptor(se: &'a mut Serializer<W>, num: usize) -> Self {
        Self {
            se,
            state: StructSerializerType::Descriptor,
            num,
            buf: Vec::new(),
        }
    }

    pub fn basic_value(se: &'a mut Serializer<W>, num: usize) -> Self {
        Self {
            se,
            state: StructSerializerType::BasicValue,
            num,
            buf: Vec::new(),
        }
    }

    pub fn list_value(se: &'a mut Serializer<W>, num: usize) -> Self {
        Self {
            se,
            state: StructSerializerType::ListValue,
            num,
            buf: Vec::new(),
        }
    }

    pub fn map_value(se: &'a mut Serializer<W>, num: usize) -> Self {
        Self {
            se,
            state: StructSerializerType::MapValue,
            num,
            buf: Vec::new(),
        }
    }
}

impl<'a, W: 'a> AsMut<Serializer<W>> for StructSerialzier<'a, W> {
    fn as_mut(&mut self) -> &mut Serializer<W> {
        self.se
    }
}

// Serialize into a List with
impl<'a, W: Write + 'a> ser::SerializeTupleStruct for StructSerialzier<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        unimplemented!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        unimplemented!()
    }
}

// Serialize into a list
impl<'a, W: Write + 'a> ser::SerializeStruct for StructSerialzier<'a, W> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        match self.state {
            StructSerializerType::Descriptor => {
                value.serialize(self.as_mut())
            },
            StructSerializerType::Described => {
                value.serialize(self.as_mut())
            },
            StructSerializerType::BasicValue => {
                value.serialize(self.as_mut())
            },
            StructSerializerType::ListValue => {
                let mut serializer = Serializer::described_list(&mut self.buf);
                value.serialize(&mut serializer)
            },
            StructSerializerType::MapValue => {
                let mut serializer = Serializer::described_map(&mut self.buf);
                value.serialize(&mut serializer)
            },
        }
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        match self.state {
            StructSerializerType::Descriptor => {
                Ok(())
            },
            StructSerializerType::Described => {
                Ok(())
            },
            StructSerializerType::BasicValue => {
                Ok(())
            },
            StructSerializerType::ListValue => {
                write_seq(&mut self.se.writer, self.num, self.buf)
            },
            StructSerializerType::MapValue => {
                write_map(&mut self.se.writer, self.num, self.buf)
            },
        }
    }
}

pub struct VariantSerializer<'a, W: 'a> {
    se: &'a mut Serializer<W>,
    _name: &'static str,
    _variant_index: u32,
    variant: &'static str,
    num: usize,
    list_buf: Vec<u8>,
}

impl<'a, W: 'a> VariantSerializer<'a, W> {
    pub fn new(
        se: &'a mut Serializer<W>,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        num: usize,
    ) -> Self {
        let buf = Vec::new();

        Self {
            se,
            _name: name,
            _variant_index: variant_index,
            variant,
            num,
            list_buf: buf,
        }
    }
}

impl<'a, W: Write + 'a> ser::SerializeTupleVariant for VariantSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut se = Serializer::new(&mut self.list_buf);
        value.serialize(&mut se)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let Self {
            se,
            variant,
            num,
            list_buf,
            ..
        } = self;

        // finish serializing the tuple
        let mut val_buf = Vec::new();
        write_seq(&mut val_buf, num, list_buf)?;

        // serialize key as string
        let mut buf = Vec::new();
        let mut key_serializer = Serializer::new(&mut buf);
        variant.serialize(&mut key_serializer)?;

        // Join key_buf and val_buf
        buf.append(&mut val_buf);

        // write as a map
        write_map(&mut se.writer, num, buf)
    }
}

impl<'a, W: Write + 'a> ser::SerializeStructVariant for VariantSerializer<'a, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let mut se = Serializer::new(&mut self.list_buf);
        value.serialize(&mut se)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let Self {
            se,
            variant,
            num,
            list_buf,
            ..
        } = self;

        // finish serializing the struct as list
        let mut val_buf = Vec::new();
        write_seq(&mut val_buf, num, list_buf)?;

        // serialize key as string
        let mut buf = Vec::new();
        let mut key_serializer = Serializer::new(&mut buf);
        ser::Serialize::serialize(&variant, &mut key_serializer)?;

        // Join key_buf and val_buf
        buf.append(&mut val_buf);

        // write as a map
        write_map(&mut se.writer, num, buf)
    }
}

#[cfg(test)]
mod test {
    use crate::{constructor::EncodingCodes, descriptor::Descriptor};

    use super::*;

    fn assert_eq_on_serialized_vs_expected<T: Serialize>(val: T, expected: Vec<u8>) {
        let serialized = to_vec(&val).unwrap();
        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_bool() {
        let val = true;
        let expected = vec![EncodingCodes::BooleanTrue as u8];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = false;
        let expected = vec![EncodingCodes::BooleanFalse as u8];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i8() {
        let val = 0i8;
        let expected = vec![EncodingCodes::Byte as u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = i8::MIN;
        let expected = vec![EncodingCodes::Byte as u8, 128u8];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = i8::MAX;
        let expected = vec![EncodingCodes::Byte as u8, 127u8];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i16() {
        let val = 0i16;
        let expected = vec![EncodingCodes::Short as u8, 0, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = -1i16;
        let expected = vec![EncodingCodes::Short as u8, 255, 255];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i32() {
        // small int
        let val = 0i32;
        let expected = vec![EncodingCodes::SmallInt as u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        // int
        let val = i32::MAX;
        let expected = vec![EncodingCodes::Int as u8, 127, 255, 255, 255];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_i64() {
        // small long
        let val = 0i64;
        let expected = vec![EncodingCodes::SmallLong as u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        // long
        let val = i64::MAX;
        let expected = vec![
            EncodingCodes::Long as u8,
            127,
            255,
            255,
            255,
            255,
            255,
            255,
            255,
        ];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u8() {
        let val = u8::MIN;
        let expected = vec![EncodingCodes::Ubyte as u8, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = u8::MAX;
        let expected = vec![EncodingCodes::Ubyte as u8, 255];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u16() {
        let val = 0u16;
        let expected = vec![EncodingCodes::Ushort as u8, 0, 0];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = 131u16;
        let expected = vec![EncodingCodes::Ushort as u8, 0, 131];
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = 65535u16;
        let expected = vec![EncodingCodes::Ushort as u8, 255, 255];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u32() {
        // uint0
        let val = 0u32;
        let expected = vec![EncodingCodes::Uint0 as u8];
        assert_eq_on_serialized_vs_expected(val, expected);

        // small uint
        let val = 255u32;
        let expected = vec![EncodingCodes::SmallUint as u8, 255];
        assert_eq_on_serialized_vs_expected(val, expected);

        // uint
        let val = u32::MAX;
        let mut expected = vec![EncodingCodes::Uint as u8];
        expected.append(&mut vec![255; 4]);
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_u64() {
        // ulong0
        let val = 0u64;
        let expected = vec![EncodingCodes::Ulong0 as u8];
        assert_eq_on_serialized_vs_expected(val, expected);

        // small ulong
        let val = 255u64;
        let expected = vec![EncodingCodes::SmallUlong as u8, 255];
        assert_eq_on_serialized_vs_expected(val, expected);

        // ulong
        let val = u64::MAX;
        let mut expected = vec![EncodingCodes::Ulong as u8];
        expected.append(&mut vec![255u8; 8]);
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_f32() {
        let val = f32::MIN;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = -123.456f32;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = 0.0f32;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = 123.456f32;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        let val = f32::MAX;
        let mut expected = vec![EncodingCodes::Float as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_f64() {
        let val = 123.456f64;
        let mut expected = vec![EncodingCodes::Double as u8];
        expected.append(&mut val.to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_char() {
        let val = 'c';
        let mut expected = vec![EncodingCodes::Char as u8];
        expected.append(&mut (val as u32).to_be_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_str() {
        const SMALL_STRING_VALUIE: &str = "Small String";
        const LARGE_STRING_VALUIE: &str = r#"Large String: 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog. 
            "The quick brown fox jumps over the lazy dog."#;

        // str8
        let val = SMALL_STRING_VALUIE;
        let len = val.len() as u8;
        let mut expected = vec![EncodingCodes::Str8 as u8, len];
        expected.append(&mut val.as_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        // str32
        let val = LARGE_STRING_VALUIE;
        let len = val.len() as u32;
        let mut expected = vec![EncodingCodes::Str32 as u8];
        expected.append(&mut len.to_be_bytes().to_vec());
        expected.append(&mut val.as_bytes().to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_bytes() {
        // serialize_bytes only works with serde_bytes wrapper types `Bytes` and `ByteBuf`
        use serde_bytes::ByteBuf;
        const SMALL_BYTES_VALUE: &[u8] = &[133u8; 200];
        const LARGE_BYTES_VALUE: &[u8] = &[199u8; 1000];

        // vbin8
        let val = ByteBuf::from(SMALL_BYTES_VALUE);
        let len = val.len() as u8;
        let mut expected = vec![EncodingCodes::VBin8 as u8, len];
        expected.append(&mut val.to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);

        // vbin32
        let val = ByteBuf::from(LARGE_BYTES_VALUE);
        let len = val.len() as u32;
        let mut expected = vec![EncodingCodes::VBin32 as u8];
        expected.append(&mut len.to_be_bytes().to_vec());
        expected.append(&mut val.to_vec());
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_none() {
        let val: Option<()> = None;
        let expected = vec![EncodingCodes::Null as u8];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_some() {
        let val = Some(1i32);
        let expected = super::to_vec(&1i32).unwrap();
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_unit() {
        let val = ();
        let expected = vec![EncodingCodes::Null as u8];
        assert_eq_on_serialized_vs_expected(val, expected);
    }

    #[test]
    fn test_serialize_symbol() {
        use crate::types::Symbol;
        let symbol = Symbol::new("amqp".into());
        let expected = vec![0xa3 as u8, 0x04, 0x61, 0x6d, 0x71, 0x70];
        assert_eq_on_serialized_vs_expected(symbol, expected);
    }

    #[test]
    fn test_serialize_descriptor_name() {
        // The descriptor name should just be serialized as a symbol
        let descriptor = Descriptor::new(String::from("amqp"), None);
        let expected = vec![0xa3 as u8, 0x04, 0x61, 0x6d, 0x71, 0x70];
        assert_eq_on_serialized_vs_expected(descriptor, expected);
    }

    #[test]
    fn test_serialize_descriptor_code() {
        let descriptor = Descriptor::new(String::from("amqp"), Some(0xf2));
        let expected = vec![0x53, 0xf2];
        assert_eq_on_serialized_vs_expected(descriptor, expected);
    }

    #[test]
    fn test_serialize_non_described_struct() {
        use serde::Serialize;

        #[derive(Serialize)]
        struct Foo {
            a: i32,
            b: bool
        }

        let val = Foo {a: 13, b: true };
        let output = to_vec(&val).unwrap();
        println!("{:?}", &output);
    }
}