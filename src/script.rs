use std::{
    env, fs,
    path::{Path, PathBuf},
};

use rlua::{Function, Lua, StdLib, Table};

use crate::{KuleError, KuleResult};

pub use rlua;
pub use rlua::Context as LuaContext;

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
    /// Get the path to the config file
    pub fn config_path(&self) -> PathBuf {
        self.dir.join(&self.config).with_extension("toml")
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
        F: FnOnce(LuaContext) -> R,
    {
        let current_dir = env::current_dir()?;
        fs::create_dir_all(&self.env.dir)?;
        env::set_current_dir(&self.env.dir)?;
        let res = self.lua.context(f);
        env::set_current_dir(current_dir)?;
        Ok(res)
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
        let (lua, modules) = self.lua(|_| -> KuleResult<_> {
            let modules_bytes = fs::read(self.env.config_path())?;
            let modules: Modules = toml::from_slice(&modules_bytes)?;
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
        })??;
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
        })??;
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
    pub fn batch_call<'a, F, E>(&'a self, function_name: &str, call: F) -> KuleResult<()>
    where
        F: Fn(Function) -> Result<(), E>,
        KuleError: From<E>,
    {
        self.lua(move |ctx| -> KuleResult<()> {
            let globals = ctx.globals();
            for name in self.enabled_modules() {
                if let Ok(module) = globals.get::<_, Table>(name) {
                    if let Ok(function) = module.get::<_, Function>(function_name) {
                        call(function)?;
                    }
                }
            }
            Ok(())
        })?
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
