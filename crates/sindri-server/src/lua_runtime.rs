use mlua::{Lua, Table};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, RwLock};

use sindri::component::Component;
use sindri::scene::Scene;

pub struct LuaRuntime {
    lua: Lua,
    envs: HashMap<(u64, String), mlua::RegistryKey>,
    started: HashSet<(u64, String)>,
    pub elapsed: f64,
    // Key state shared into Lua globals each frame
    keys: Arc<RwLock<HashSet<String>>>,
    prev_keys: HashSet<String>,
}

impl LuaRuntime {
    pub fn new() -> anyhow::Result<Self> {
        let lua = Lua::new();

        let print_fn = lua.create_function(|_, args: mlua::MultiValue| {
            let parts: Vec<String> = args
                .iter()
                .map(|v| match v {
                    mlua::Value::String(s) => s.to_str().unwrap_or("?").to_string(),
                    mlua::Value::Integer(n) => n.to_string(),
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::Boolean(b) => b.to_string(),
                    mlua::Value::Nil => "nil".to_string(),
                    _ => "?".to_string(),
                })
                .collect();
            println!("[lua] {}", parts.join("\t"));
            Ok(())
        })?;
        lua.globals().set("print", print_fn)?;

        let keys: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));

        // key_down(key) — true while key is held
        let keys_down = keys.clone();
        let key_down_fn = lua.create_function(move |_, key: String| {
            Ok(keys_down.read().map(|k| k.contains(&key)).unwrap_or(false))
        })?;
        lua.globals().set("key_down", key_down_fn)?;

        // key_pressed(key) — true only on the first frame the key is down
        // Updated each frame via update_key_globals()
        lua.globals().set(
            "key_pressed",
            lua.create_function(|_, _key: String| Ok(false))?,
        )?;

        Ok(Self {
            lua,
            envs: HashMap::new(),
            started: HashSet::new(),
            elapsed: 0.0,
            keys,
            prev_keys: HashSet::new(),
        })
    }

    fn update_key_globals(&mut self, new_keys: &HashSet<String>) {
        // Update the shared key set
        if let Ok(mut k) = self.keys.write() {
            *k = new_keys.clone();
        }

        // Recompute key_pressed: keys that are new this frame
        let pressed_this_frame: HashSet<String> =
            new_keys.difference(&self.prev_keys).cloned().collect();
        let keys_pressed = Arc::new(RwLock::new(pressed_this_frame));
        let kp = keys_pressed.clone();
        let _ = self.lua.globals().set(
            "key_pressed",
            self.lua
                .create_function(move |_, key: String| {
                    Ok(kp.read().map(|k| k.contains(&key)).unwrap_or(false))
                })
                .expect("key_pressed fn"),
        );

        self.prev_keys = new_keys.clone();
    }

    /// Load a script into a sandboxed environment for the given (entity, path) pair.
    /// Returns the env table from the registry.
    fn load_env(&mut self, entity_id: u64, path: &str, code: &str) -> anyhow::Result<()> {
        let key = (entity_id, path.to_string());
        if self.envs.contains_key(&key) {
            return Ok(());
        }

        // Create a sandboxed env that falls back to Lua globals for builtins
        let env: Table = self.lua.create_table()?;
        let mt: Table = self.lua.create_table()?;
        mt.set("__index", self.lua.globals())?;
        env.set_metatable(Some(mt));

        match self
            .lua
            .load(code)
            .set_name(path)
            .set_environment(env.clone())
            .exec()
        {
            Ok(_) => {}
            Err(e) => eprintln!("[lua] load error ({path}): {e}"),
        }

        let reg = self.lua.create_registry_value(env)?;
        self.envs.insert(key, reg);
        Ok(())
    }

    /// Called when play starts — clears cached envs and started set so scripts restart cleanly.
    pub fn reset(&mut self) {
        self.envs.clear();
        self.started.clear();
        self.elapsed = 0.0;
        self.prev_keys.clear();
        if let Ok(mut k) = self.keys.write() {
            k.clear();
        }
    }

    pub fn update(
        &mut self,
        scene: &mut Scene,
        scripts_root: &Path,
        dt: f32,
        keys: &HashSet<String>,
    ) {
        self.update_key_globals(keys);
        self.elapsed += dt as f64;
        let elapsed = self.elapsed;

        // Collect entity ids to avoid borrow issues
        let entity_ids: Vec<u64> = scene.entities.keys().cloned().collect();

        for entity_id in entity_ids {
            let Some(entity) = scene.entities.get(&entity_id) else {
                continue;
            };

            // Collect script paths and current transform
            let script_paths: Vec<String> = entity
                .components
                .iter()
                .filter_map(|c| {
                    if let Component::Script(s) = c {
                        Some(s.path.clone())
                    } else {
                        None
                    }
                })
                .collect();

            if script_paths.is_empty() {
                continue;
            }

            let transform = entity.components.iter().find_map(|c| {
                if let Component::Transform(t) = c {
                    Some(t.clone())
                } else {
                    None
                }
            });

            for script_path in script_paths {
                if script_path.is_empty() {
                    continue;
                }

                let full_path = scripts_root.join(&script_path);
                if !full_path.exists() {
                    continue;
                }

                let code = match std::fs::read_to_string(&full_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                if let Err(e) = self.load_env(entity_id, &script_path, &code) {
                    eprintln!("[lua] env error: {e}");
                    continue;
                }

                let key = (entity_id, script_path.clone());
                let env: Table = match self
                    .envs
                    .get(&key)
                    .and_then(|rk| self.lua.registry_value::<Table>(rk).ok())
                {
                    Some(t) => t,
                    None => continue,
                };

                // Build self table with entity state
                let self_tbl: Table = match self.lua.create_table() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let _ = self_tbl.set("entity_id", entity_id);
                let _ = self_tbl.set("elapsed", elapsed);
                if let Some(ref t) = transform {
                    let _ = self_tbl.set("x", t.x as f64);
                    let _ = self_tbl.set("y", t.y as f64);
                    let _ = self_tbl.set("rotation", t.rotation as f64);
                    let _ = self_tbl.set("scale_x", t.scale_x as f64);
                    let _ = self_tbl.set("scale_y", t.scale_y as f64);
                } else {
                    let _ = self_tbl.set("x", 0.0f64);
                    let _ = self_tbl.set("y", 0.0f64);
                    let _ = self_tbl.set("rotation", 0.0f64);
                    let _ = self_tbl.set("scale_x", 1.0f64);
                    let _ = self_tbl.set("scale_y", 1.0f64);
                }

                // on_start (once)
                if !self.started.contains(&key) {
                    if let Ok(f) = env.get::<_, mlua::Function>("on_start") {
                        if let Err(e) = f.call::<_, ()>(self_tbl.clone()) {
                            eprintln!("[lua] on_start error ({script_path}): {e}");
                        }
                    }
                    self.started.insert(key.clone());
                }

                // on_update
                if let Ok(f) = env.get::<_, mlua::Function>("on_update") {
                    if let Err(e) = f.call::<_, ()>((self_tbl.clone(), dt as f64)) {
                        eprintln!("[lua] on_update error ({script_path}): {e}");
                    }
                }

                // Write transform back
                let read_f64 = |t: &Table, k: &str, default: f32| -> f32 {
                    t.get::<_, f64>(k).map(|v| v as f32).unwrap_or(default)
                };
                let (ox, oy, orot, osx, osy) = transform
                    .as_ref()
                    .map(|t| (t.x, t.y, t.rotation, t.scale_x, t.scale_y))
                    .unwrap_or((0.0, 0.0, 0.0, 1.0, 1.0));

                let nx = read_f64(&self_tbl, "x", ox);
                let ny = read_f64(&self_tbl, "y", oy);
                let nr = read_f64(&self_tbl, "rotation", orot);
                let nsx = read_f64(&self_tbl, "scale_x", osx);
                let nsy = read_f64(&self_tbl, "scale_y", osy);

                if let Some(entity_mut) = scene.entities.get_mut(&entity_id) {
                    if let Some(t) = entity_mut.components.iter_mut().find_map(|c| {
                        if let Component::Transform(t) = c {
                            Some(t)
                        } else {
                            None
                        }
                    }) {
                        t.x = nx;
                        t.y = ny;
                        t.rotation = nr;
                        t.scale_x = nsx;
                        t.scale_y = nsy;
                    }
                }
            }
        }
    }

    /// Drop cached state for a removed entity so scripts restart if it's re-added.
    #[allow(dead_code)]
    pub fn evict(&mut self, entity_id: u64) {
        self.envs.retain(|(id, _), _| *id != entity_id);
        self.started.retain(|(id, _)| *id != entity_id);
    }
}
