use std::{
    convert::TryInto,
    env, fmt, fs,
    num::TryFromIntError,
    path::{Path, PathBuf},
};

use rlua::{Function, Lua, Table, Value};
use serde::ser::*;

use crate::KuleResult;

pub use rlua;
pub use rlua::Context as LuaContext;
pub use rlua::StdLib;

/// Defines where script modules should be saved to and loaded from
#[derive(Debug, Clone)]
pub struct ScriptEnv {
    /// The directory that should contain script modules
    pub dir: PathBuf,
    /// The name of the config file
    ///
    /// This will be joined onto `dir` and given a `toml` extension
    pub config: String,
    /// The Lua standard library to use
    ///
    /// An error will occur if you include `StdLib::DEBUG`. `ScriptEnv::new`
    /// automatically removes `DEBUG` from whatever flags you pass it.
    pub std_lib: StdLib,
}

impl Default for ScriptEnv {
    fn default() -> Self {
        ScriptEnv::new("modules", "modules", StdLib::ALL & !StdLib::IO)
    }
}

impl ScriptEnv {
    /// Create a new `ScriptEnv
    ///
    /// `StdLib::DEBUG` is automatically removed for safety reasons.
    pub fn new<D, C>(dir: D, config: C, std_lib: StdLib) -> Self
    where
        D: AsRef<Path>,
        C: Into<String>,
    {
        ScriptEnv {
            dir: dir.as_ref().into(),
            config: config.into(),
            std_lib: std_lib & !StdLib::DEBUG,
        }
    }
    /// Get the file name of the config file
    pub fn config_file(&self) -> PathBuf {
        PathBuf::from(&self.config).with_extension("toml")
    }
    /// Get the path to the config file
    pub fn config_path(&self) -> PathBuf {
        self.dir.join(self.config_file())
    }
}

/// A handle to a scripting environment
pub struct Scripts {
    /// The list of modules
    pub modules: Vec<Module>,
    /// The script environment
    pub env: ScriptEnv,
    lua: Lua,
}

impl Scripts {
    /**
    Access the Lua context

    For the duration of the passed closue, the program's current directory
    will be the script modules directory
    */
    #[allow(clippy::redundant_closure)]
    pub fn lua<F, R>(&self, f: F) -> KuleResult<R>
    where
        F: FnOnce(LuaContext) -> KuleResult<R>,
    {
        let current_dir = env::current_dir()?;
        fs::create_dir_all(&self.env.dir)?;
        env::set_current_dir(&self.env.dir)?;
        let res = self.lua.context(f)?;
        env::set_current_dir(current_dir)?;
        Ok(res)
    }
    /// Serialize a value into a global Lua value
    pub fn serialize_global<T>(&self, name: &str, val: &T) -> KuleResult<()>
    where
        T: Serialize,
    {
        self.lua(move |ctx| -> KuleResult<()> {
            let mut ser = LuaSerializer::new(ctx);
            let value = ser.serialize(val)?;
            ctx.globals().set(name, value)?;
            Ok(())
        })
    }
    /// Load scripts with the given lua std library
    pub fn load(env: ScriptEnv) -> KuleResult<Self> {
        let mut scripts = Scripts {
            lua: Lua::new(),
            modules: Vec::new(),
            env,
        };
        scripts.reload()?;
        Ok(scripts)
    }
    /// Reload the scripts
    #[allow(clippy::redundant_closure)]
    pub fn reload(&mut self) -> KuleResult<()> {
        let (lua, modules) = self.lua(|_| {
            let config_text = fs::read_to_string(self.env.config_file())?;
            let modules: Modules = toml::from_str(&config_text)?;
            let lua = Lua::new_with(self.env.std_lib);
            lua.context(|ctx| -> rlua::Result<()> {
                // Load modules
                ctx.load(
                    &modules
                        .list
                        .iter()
                        .filter(|m| m.enabled)
                        .map(|m| format!("{0} = require(\"{0}\")\n", m.name))
                        .collect::<String>(),
                )
                .exec()?;
                Ok(())
            })?;
            Ok((lua, modules))
        })?;
        self.lua = lua;
        self.modules = modules.list;
        Ok(())
    }
    /// Save the script modules
    pub fn save_modules(&self) -> KuleResult<()> {
        self.lua(|_| {
            Modules {
                list: self.modules.clone(),
            }
            .save(&self.env.config_path())
        })?;
        Ok(())
    }
    /// Iterate over the names of the enabled modules
    pub fn enabled_modules(&self) -> impl Iterator<Item = &str> {
        self.modules
            .iter()
            .filter(|m| m.enabled)
            .map(|m| m.name.as_str())
    }
    /**
    Call the same function in each module that has it

    Module order is respected.

    This makes it easy to have multiple modules define the same type of behavior
    and execute it all at once.
    */
    pub fn batch_call<'a, F>(&'a self, function_name: &str, call: F) -> KuleResult<()>
    where
        F: Fn(Function) -> KuleResult<()>,
    {
        self.lua(move |ctx| {
            let globals = ctx.globals();
            for name in self.enabled_modules() {
                if let Ok(module) = globals.get::<_, Table>(name) {
                    if let Ok(function) = module.get::<_, Function>(function_name) {
                        call(function)?;
                    }
                }
            }
            Ok(())
        })
    }
}

fn default_enabled() -> bool {
    true
}

/// An identifier for a script module
#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Module {
    name: String,
    #[serde(default = "default_enabled", skip_serializing_if = "Clone::clone")]
    enabled: bool,
}

impl Module {
    /// Get the module name
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Get whether the module is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    /// Set whether the module is enabled
    ///
    /// After enabling or disabling a module [`Scripts::reload`] must
    /// be called to actually see the changes
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[derive(Debug, Default, serde_derive::Serialize, serde_derive::Deserialize)]
struct Modules {
    #[serde(rename = "mod")]
    list: Vec<Module>,
}

impl Modules {
    fn save(&self, path: &Path) -> KuleResult<()> {
        let bytes = toml::to_vec(self)?;
        fs::write(path, &bytes)?;
        Ok(())
    }
}

/// A serializer that turns serializable values into Lua values
pub struct LuaSerializer<'lua> {
    ctx: LuaContext<'lua>,
    output: Value<'lua>,
    last_key: Option<Value<'lua>>,
}

impl<'lua> LuaSerializer<'lua> {
    /// Create a new `LuaSerializer` from a Lua context
    pub fn new(ctx: LuaContext<'lua>) -> Self {
        LuaSerializer {
            ctx,
            output: Value::Nil,
            last_key: None,
        }
    }
    fn another(&self) -> Self {
        LuaSerializer::new(self.ctx)
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

impl<'lua> From<LuaContext<'lua>> for LuaSerializer<'lua> {
    fn from(ctx: LuaContext<'lua>) -> Self {
        LuaSerializer::new(ctx)
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
    Lua(#[from] rlua::Error),
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
        self.output = Value::String(self.ctx.create_string(v)?);
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
        let table = self.ctx.create_table()?;
        table.set("variant", variant)?;
        self.output = Value::Table(table);
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
        let table = self.ctx.create_table()?;
        table.set("variant", variant)?;
        table.set("value", self.another().serialize(&value)?)?;
        self.output = Value::Table(table);
        Ok(())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.output = Value::Table(self.ctx.create_table()?);
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
        let table = self.ctx.create_table()?;
        table.set("variant", variant)?;
        self.output = Value::Table(table);
        Ok(self)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.output = Value::Table(self.ctx.create_table()?);
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
        let table = self.ctx.create_table()?;
        table.set("variant", variant)?;
        self.output = Value::Table(table);
        Ok(self)
    }
}

#[cfg(test)]
#[test]
fn lua_ser() {
    let lua = Lua::new();
    let module = Module {
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
    lua.context(|ctx| {
        let mut serializer = LuaSerializer::new(ctx);
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
    });
}
