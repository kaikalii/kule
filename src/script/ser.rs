use std::{convert::TryInto, fmt, num::TryFromIntError};

use mlua::{Lua, Value};
use serde::ser::*;

/// A serializer that turns serializable values into Lua values
pub struct LuaSerializer<'lua> {
    lua: &'lua Lua,
    output: Value<'lua>,
    last_key: Option<Value<'lua>>,
}

impl<'lua> LuaSerializer<'lua> {
    /// Create a new `LuaSerializer` from a Lua context
    pub fn new(lua: &'lua Lua) -> Self {
        LuaSerializer {
            lua,
            output: Value::Nil,
            last_key: None,
        }
    }
    fn another(&self) -> Self {
        LuaSerializer::new(self.lua)
    }
    /// Serialize a value to a Lua value
    pub fn serialize<T>(&mut self, value: &T) -> Result<Value<'lua>, LuaSerializeError>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut *self)?;
        let res = std::mem::replace(&mut self.output, Value::Nil);
        Ok(res)
    }
}

impl<'lua> From<&'lua Lua> for LuaSerializer<'lua> {
    fn from(lua: &'lua Lua) -> Self {
        LuaSerializer::new(lua)
    }
}

/// An error generated when attempting to serialize into a lua value
#[derive(Debug, Clone, thiserror::Error)]
pub enum LuaSerializeError {
    /// A custom error type output by serde
    #[error("{0}")]
    Custom(String),
    /// Error converting integer
    #[error("{0}")]
    IntConversion(#[from] TryFromIntError),
    /// Lua error
    #[error("{0}")]
    Lua(#[from] mlua::Error),
}

impl serde::ser::Error for LuaSerializeError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        LuaSerializeError::Custom(msg.to_string())
    }
}

impl<'a, 'lua> SerializeSeq for &'a mut LuaSerializer<'lua> {
    type Ok = ();
    type Error = LuaSerializeError;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        if let Value::Table(table) = &self.output {
            table.set(table.raw_len() + 1, self.another().serialize(&value)?)?;
            Ok(())
        } else {
            panic!()
        }
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'lua> SerializeTuple for &'a mut LuaSerializer<'lua> {
    type Ok = ();
    type Error = LuaSerializeError;
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        if let Value::Table(table) = &self.output {
            table.set(table.raw_len() + 1, self.another().serialize(&value)?)?;
            Ok(())
        } else {
            panic!()
        }
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'lua> SerializeTupleStruct for &'a mut LuaSerializer<'lua> {
    type Ok = ();
    type Error = LuaSerializeError;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize,
    {
        if let Value::Table(table) = &self.output {
            table.set(table.raw_len() + 1, self.another().serialize(&value)?)?;
            Ok(())
        } else {
            panic!()
        }
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'lua> SerializeTupleVariant for &'a mut LuaSerializer<'lua> {
    type Ok = ();
    type Error = LuaSerializeError;
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if let Value::Table(table) = &self.output {
            table.set(table.raw_len() + 1, self.another().serialize(&value)?)?;
            Ok(())
        } else {
            panic!()
        }
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'lua> SerializeMap for &'a mut LuaSerializer<'lua> {
    type Ok = ();
    type Error = LuaSerializeError;
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if let Value::Table(_) = &self.output {
            self.last_key = Some(self.another().serialize(&key)?);
            Ok(())
        } else {
            panic!()
        }
    }
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if let Value::Table(table) = &self.output {
            table.set(
                self.last_key.take().unwrap(),
                self.another().serialize(&value)?,
            )?;
            Ok(())
        } else {
            panic!()
        }
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'lua> SerializeStruct for &'a mut LuaSerializer<'lua> {
    type Ok = ();
    type Error = LuaSerializeError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if let Value::Table(table) = &self.output {
            table.set(
                self.another().serialize(&key)?,
                self.another().serialize(&value)?,
            )?;
            Ok(())
        } else {
            panic!()
        }
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'lua> SerializeStructVariant for &'a mut LuaSerializer<'lua> {
    type Ok = ();
    type Error = LuaSerializeError;
    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        if let Value::Table(table) = &self.output {
            table.set(
                self.another().serialize(&key)?,
                self.another().serialize(&value)?,
            )?;
            Ok(())
        } else {
            panic!()
        }
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<'a, 'lua> Serializer for &'a mut LuaSerializer<'lua> {
    type Ok = ();
    type Error = LuaSerializeError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Boolean(v);
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Integer(v.into());
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Integer(v.into());
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Integer(v.into());
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Integer(v);
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Integer(v.into());
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Integer(v.into());
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Integer(v.into());
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Integer(v.try_into()?);
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Number(v.into());
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Number(v);
        Ok(())
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.output = Value::String(self.lua.create_string(v)?);
        Ok(())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for b in v {
            SerializeSeq::serialize_element(&mut seq, b)?;
        }
        SerializeSeq::end(seq)
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Nil;
        Ok(())
    }
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.output = Value::Nil;
        Ok(())
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(name)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.output = Value::String(self.lua.create_string(variant)?);
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::Serialize,
    {
        let table = self.lua.create_table()?;
        table.set("variant", variant)?;
        table.set("value", self.another().serialize(&value)?)?;
        self.output = Value::Table(table);
        Ok(())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.output = Value::Table(self.lua.create_table()?);
        Ok(self)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        let table = self.lua.create_table()?;
        table.set("variant", variant)?;
        self.output = Value::Table(table);
        Ok(self)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.output = Value::Table(self.lua.create_table()?);
        Ok(self)
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        let table = self.lua.create_table()?;
        table.set("variant", variant)?;
        self.output = Value::Table(table);
        Ok(self)
    }
}

#[cfg(test)]
#[test]
fn lua_ser() {
    let lua = mlua::Lua::new();
    let module = crate::Module {
        name: "core".into(),
        enabled: false,
    };
    #[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
    enum MyEnum {
        Foo,
        Bar(u32),
        Baz(f64, bool),
        Qux { name: &'static str, enabled: bool },
    }
    let mut serializer = LuaSerializer::new(&lua);
    let val = serializer.serialize(&module).unwrap();
    if let Value::Table(table) = val {
        for pair in table.pairs::<String, Value>() {
            let (key, value) = pair.unwrap();
            println!("{:?} => {:?}", key, value);
        }
    } else {
        panic!()
    }
    println!();
    for my_enum in vec![
        MyEnum::Foo,
        MyEnum::Bar(5),
        MyEnum::Baz(3.7, true),
        MyEnum::Qux {
            name: "Dave",
            enabled: true,
        },
    ] {
        println!("{:?}", my_enum);
        let val = serializer.serialize(&my_enum).unwrap();
        if let Value::Table(table) = val {
            for pair in table.pairs::<String, Value>() {
                let (key, value) = pair.unwrap();
                println!("{:?} => {:?}", key, value);
            }
        } else {
            panic!()
        }
        println!();
    }
}
