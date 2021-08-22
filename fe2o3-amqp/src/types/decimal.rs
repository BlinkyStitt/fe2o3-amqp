//! Custom structs that hold bytes for decimal types

use std::convert::TryFrom;

use serde::ser;
use serde::de;
use serde_bytes::Bytes;

use crate::error::Error;


mod dec32 {
    use serde_bytes::ByteBuf;

    use super::*;

    pub const DECIMAL32: &str = "DECIMAL32";
    pub const DECIMAL32_LEN: usize = 4;
    
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Dec32([u8; DECIMAL32_LEN]);
    impl From<[u8; DECIMAL32_LEN]> for Dec32 {
        fn from(val: [u8; DECIMAL32_LEN]) -> Self {
            Self(val)
        }
    }
    
    impl TryFrom<&[u8]> for Dec32 {
        type Error = Error;
    
        fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
            if value.len() != DECIMAL32_LEN {
                return Err(Error::InvalidLength)
            }
    
            let mut buf = [0u8; DECIMAL32_LEN];
            buf.copy_from_slice(&value[..DECIMAL32_LEN]);
            Ok(Self(buf))
        }
    }
    
    impl ser::Serialize for Dec32 {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer 
        {
            serializer.serialize_newtype_struct(DECIMAL32, Bytes::new(&self.0))
        }
    }

    struct Visitor { }

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Dec32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct Dec32")
        }

        fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>, 
        {
            let val: ByteBuf = de::Deserialize::deserialize(deserializer)?;
            Dec32::try_from(val.as_slice())
                .map_err(|err| de::Error::custom(err.to_string()))
        }
    }

    impl<'de> de::Deserialize<'de> for Dec32 {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> 
        {
            deserializer.deserialize_newtype_struct(DECIMAL32, Visitor { })
        }
    }
}

mod dec64 {
    use serde_bytes::ByteBuf;

    use super::*;

    pub const DECIMAL64: &str = "DECIMAL64";
    pub const DECIMAL64_LEN: usize = 8;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Dec64([u8; DECIMAL64_LEN]);
    
    impl From<[u8; DECIMAL64_LEN]> for Dec64 {
        fn from(val: [u8; DECIMAL64_LEN]) -> Self {
            Self(val)
        }
    }
    
    impl TryFrom<&[u8]> for Dec64 {
        type Error = Error;
    
        fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
            if value.len() != DECIMAL64_LEN {
                return Err(Error::InvalidLength)
            }
    
            let mut buf = [0u8; DECIMAL64_LEN];
            buf.copy_from_slice(&value[..DECIMAL64_LEN]);
            Ok(Self(buf))
        }
    }
    
    impl ser::Serialize for Dec64 {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer 
        {
            serializer.serialize_newtype_struct(DECIMAL64, Bytes::new(&self.0))
        }
    }

    struct Visitor { }

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Dec64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct Dec64")
        }

        fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>, 
        {
            let val: ByteBuf = de::Deserialize::deserialize(deserializer)?;
            Dec64::try_from(val.as_slice())
                .map_err(|err| de::Error::custom(err.to_string()))
        }
    }

    impl<'de> de::Deserialize<'de> for Dec64 {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> 
        {
            deserializer.deserialize_newtype_struct(DECIMAL64, Visitor { })
        }
    }
}

mod dec128 {
    use serde_bytes::ByteBuf;

    use super::*;

    pub const DECIMAL128: &str = "DECIMAL128";
    pub const DECIMAL128_LEN: usize = 16;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Dec128([u8; DECIMAL128_LEN]);
    
    impl From<[u8; DECIMAL128_LEN]> for Dec128 {
        fn from(val: [u8; DECIMAL128_LEN]) -> Self {
            Self(val)
        }
    }
    
    impl TryFrom<&[u8]> for Dec128 {
        type Error = Error;
    
        fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
            if value.len() != DECIMAL128_LEN {
                return Err(Error::InvalidLength)
            }
    
            let mut buf = [0u8; DECIMAL128_LEN];
            buf.copy_from_slice(&value[..DECIMAL128_LEN]);
            Ok(Self(buf))
        }
    }
    
    impl ser::Serialize for Dec128 {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer 
        {
            serializer.serialize_newtype_struct(DECIMAL128, Bytes::new(&self.0))
        }
    }

    struct Visitor { }

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Dec128;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("struct Dec128")
        }

        fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>, 
        {
            let val: ByteBuf = de::Deserialize::deserialize(deserializer)?;
            Dec128::try_from(val.as_slice())
                .map_err(|err| de::Error::custom(err.to_string()))
        }
    }

    impl<'de> de::Deserialize<'de> for Dec128 {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> 
        {
            deserializer.deserialize_newtype_struct(DECIMAL128, Visitor { })    
        }
    }
}

pub use dec32::*;
pub use dec64::*;
pub use dec128::*;