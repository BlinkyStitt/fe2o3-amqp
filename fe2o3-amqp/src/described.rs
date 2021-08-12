use serde::ser::{Serialize, SerializeStruct, Serializer};

use crate::descriptor::Descriptor;

pub const DESCRIBED_BASIC: &str = "DESCRIBED_BASIC";
pub const DESCRIBED_LIST: &str = "DESCRIBED_LIST";
pub const DESCRIBED_MAP: &str = "DESCRIBED_MAP";

pub enum EncodingType {
    Basic,
    List,
    Map,
}

/// The described type will attach a descriptor before the value.
/// There is no generic implementation of serialization. But a inner type
/// specific implementation will be generated via macro.
pub struct Described<'a, T: ?Sized> {
    pub encoding_type: EncodingType,
    pub descriptor: Descriptor,
    pub value: &'a T,
}

impl<'a, T: ?Sized + Serialize> Serialize for Described<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let name = match self.encoding_type {
            EncodingType::Basic => DESCRIBED_BASIC,
            EncodingType::List => DESCRIBED_LIST,
            EncodingType::Map => DESCRIBED_MAP,
        };
        let mut state = serializer.serialize_struct(name, 2)?;
        state.serialize_field("descriptor", &self.descriptor)?;
        state.serialize_field("value", &self.value)?;
        state.end()
    }
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn
// }