use std::{convert::TryFrom, fmt, num::TryFromIntError};

use rlua::{FromLua, Table, Value};
use serde::de::*;

use crate::LuaContext;

pub struct LuaDeserializer<'lua> {
    ctx: LuaContext<'lua>,
    input: Value<'lua>,
}

impl<'lua> LuaDeserializer<'lua> {
    /// Create a new `LuaDeserializer`
    pub fn new(ctx: LuaContext<'lua>, input: Value<'lua>) -> Self {
        LuaDeserializer { ctx, input }
    }
    fn value_as<T>(&self) -> rlua::Result<T>
    where
        T: FromLua<'lua>,
    {
        T::from_lua(self.input.clone(), self.ctx)
    }
    fn another(&self, input: Value<'lua>) -> Self {
        LuaDeserializer::new(self.ctx, input)
    }
}

/// An error generated when attempting to serialize into a lua value
#[derive(Debug, Clone, thiserror::Error)]
pub enum LuaDeserializeError {
    /// A custom error type output by serde
    #[error("{0}")]
    Custom(String),
    /// Error converting integer
    #[error("{0}")]
    IntConversion(#[from] TryFromIntError),
    /// Lua error
    #[error("{0}")]
    Lua(#[from] rlua::Error),
}

impl<'lua> serde::de::Error for LuaDeserializeError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        LuaDeserializeError::Custom(msg.to_string())
    }
}

struct LuaSeqAccess<'a, 'lua> {
    de: &'a LuaDeserializer<'lua>,
    i: usize,
}

impl<'de, 'a, 'lua> LuaSeqAccess<'a, 'lua> {
    fn new(de: &'a LuaDeserializer<'lua>) -> Self {
        LuaSeqAccess { de, i: 1 }
    }
}

impl<'de, 'a, 'lua> SeqAccess<'de> for LuaSeqAccess<'a, 'lua> {
    type Error = LuaDeserializeError;
    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if let Ok(value) = self.de.value_as::<Table>()?.get::<_, Value>(self.i) {
            self.i += 1;
            seed.deserialize(&mut self.de.another(value)).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<'de, 'a, 'lua> Deserializer<'de> for &'a mut LuaDeserializer<'lua> {
    type Error = LuaDeserializeError;
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.value_as()?)
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.value_as()?)
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.value_as()?)
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.value_as()?)
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.value_as()?)
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.value_as()?)
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.value_as()?)
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.value_as()?)
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.value_as()?)
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.value_as()?)
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.value_as()?)
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(
            self.value_as::<String>()?
                .chars()
                .next()
                .unwrap_or(b'0' as char),
        )
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.value_as()?)
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.value_as()?)
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let table = self.value_as::<Table>()?;
        let mut bytes = Vec::new();
        for i in 1.. {
            if let Ok(u) = table.get::<_, u8>(i) {
                bytes.push(u);
            } else {
                break;
            }
        }
        visitor.visit_bytes(&bytes)
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let table = self.value_as::<Table>()?;
        let mut bytes = Vec::new();
        for i in 1.. {
            if let Ok(u) = table.get::<_, u8>(i) {
                bytes.push(u);
            } else {
                break;
            }
        }
        visitor.visit_byte_buf(bytes)
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if let Value::Nil = &self.input {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(LuaSeqAccess::new(self))
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
}
