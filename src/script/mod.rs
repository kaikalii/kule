mod ser;
pub use ser::*;
// mod de;
// pub use de::*;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use mlua::{FromLua, Function, ToLua};
use serde::ser::*;

use crate::KuleResult;

pub use mlua;
pub use mlua::{Lua, StdLib, Table};

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
        ScriptEnv::new("modules", "modules", StdLib::ALL ^ StdLib::IO)
    }
}

impl ScriptEnv {
    /// Create a new `ScriptEnv
    pub fn new<D, C>(dir: D, config: C, std_lib: StdLib) -> Self
    where
        D: AsRef<Path>,
        C: Into<String>,
    {
        ScriptEnv {
            dir: dir.as_ref().into(),
            config: config.into(),
            std_lib: std_lib & StdLib::ALL_SAFE,
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
    Access the Lua environment

    For the duration of the passed closue, the program's current directory
    will be the script modules directory
    */
    pub fn lua<F, R>(&self, f: F) -> KuleResult<R>
    where
        F: FnOnce(&Lua) -> KuleResult<R>,
    {
        let current_dir = env::current_dir()?;
        fs::create_dir_all(&self.env.dir)?;
        env::set_current_dir(&self.env.dir)?;
        let res = f(&self.lua)?;
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
            let lua = Lua::new_with(self.env.std_lib)?;
            // Load modules
            lua.load(
                &modules
                    .list
                    .iter()
                    .filter(|m| m.enabled)
                    .map(|m| format!("{0} = require(\"{0}\")\n", m.name))
                    .collect::<String>(),
            )
            .exec()?;
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
    Call a module method

    The `call` closure takes the Lua environment, the module table, and the function.
    This allows the method to be defined with either a `.` or a `:`.

    Nothing happens if the module table does not contain the method
    */
    pub fn call<'lua, F>(&self, module_name: &str, method_name: &str, call: F) -> KuleResult<()>
    where
        F: Fn(&'lua Lua, Table<'lua>, Function<'lua>) -> KuleResult<()>,
    {
        self.lua(|ctx| {
            let globals = ctx.globals();
            let table: Table = unsafe { std::mem::transmute(globals.val::<Table>(module_name)?) };
            if let Ok(function) = table.get(method_name) {
                call(unsafe { std::mem::transmute(ctx) }, table.clone(), function)?;
            }
            Ok(())
        })
    }
    /**
    Call the same method in each module that has it

    The `call` closure takes the Lua environment, the module table, and the function.
    This allows the method to be defined with either a `.` or a `:`.

    Module order is respected.

    This makes it easy to have multiple modules define the same type of behavior
    and execute it all at once.
    */
    pub fn batch_call<'lua, F>(&self, method_name: &str, call: F) -> KuleResult<()>
    where
        F: Fn(&'lua Lua, Table<'lua>, Function<'lua>) -> KuleResult<()>,
    {
        for name in self.enabled_modules() {
            self.call(name, method_name, &call)?;
        }
        Ok(())
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

/// Convenience methods for a Lua tables
pub trait TableExt<'lua, K> {
    /// Get a value based on a key
    fn val<V>(&self, key: K) -> KuleResult<V>
    where
        V: FromLua<'lua>;
}

impl<'lua, K> TableExt<'lua, K> for Table<'lua>
where
    K: ToLua<'lua>,
{
    fn val<V>(&self, key: K) -> KuleResult<V>
    where
        V: FromLua<'lua>,
    {
        Ok(Table::get::<K, V>(self, key)?)
    }
}
