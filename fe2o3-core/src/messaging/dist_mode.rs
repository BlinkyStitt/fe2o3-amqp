use fe2o3_amqp::types::Symbol;
use serde::{de, ser};

/// 3.5.7 Standard Distribution Mode
/// Link distribution policy.
/// <type name="std-dist-mode" class="restricted" source="symbol" provides="distribution-mode">
///     <choice name="move" value="move"/>
///     <choice name="copy" value="copy"/>
/// </type>
///
#[derive(Debug)]
pub enum DistributionMode {
    Move,
    Copy,
}

impl From<&DistributionMode> for Symbol {
    fn from(v: &DistributionMode) -> Self {
        let s = match v {
            DistributionMode::Move => "move",
            DistributionMode::Copy => "copy"
        };

        Symbol::from(s)
    }
}

impl ser::Serialize for DistributionMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let val = Symbol::from(self);
        val.serialize(serializer)
    }
}

enum Field {
    Move,
    Copy
}

struct FieldVisitor {}

impl<'de> de::Visitor<'de> for FieldVisitor {
    type Value = Field;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("variant identifier")
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
            E: de::Error, {
        self.visit_str(&v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
            E: de::Error, {
        let val = match v {
            "move" => Field::Move,
            "copy" => Field::Copy,
            _ => return Err(de::Error::custom("Invalid symbol value for DistributionMode")),
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
            D: serde::Deserializer<'de> {
        deserializer.deserialize_identifier(FieldVisitor {})
    }
}

struct Visitor {}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = DistributionMode;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("enum DistributionMode")
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
            A: de::EnumAccess<'de>, {
        let (val, _) = data.variant()?;
        let val = match val {
            Field::Move => DistributionMode::Move,
            Field::Copy => DistributionMode::Copy
        };
        Ok(val)
    }
}

impl<'de> de::Deserialize<'de> for DistributionMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const VARIANTS: &'static [&'static str] = &[
            "move",
            "copy"
        ];
        deserializer.deserialize_enum("std-dist-mode", VARIANTS, Visitor { })
    }
}