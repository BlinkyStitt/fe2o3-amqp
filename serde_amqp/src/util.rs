use crate::{descriptor::PeekDescriptor, value::de::ValueType};

#[derive(Debug, Default)]
pub(crate) enum NewType {
    #[default]
    None,
    Array,
    Dec32,
    Dec64,
    Dec128,
    Symbol,
    SymbolRef,
    Timestamp,
    Uuid,
    TransparentVec,
}

#[derive(Debug, Clone)]
pub enum IsArrayElement {
    False,
    FirstElement,
    OtherElement,
}

#[derive(Debug, Clone, Default)]
pub enum EnumType {
    #[default]
    None,
    Array,
    Descriptor,
    Value,
}

/// Described type has the descriptor as the first field
#[derive(Debug)]
pub(crate) enum FieldRole {
    Descriptor,
    Fields,
}

#[derive(Debug, Clone)]
#[repr(u8)]
#[derive(Default)]
pub enum StructEncoding {
    #[default]
    None,
    DescribedList,
    DescribedMap,
    DescribedBasic,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PeekTypeCode {
    Primitive(ValueType),
    Composite(PeekDescriptor),
}
