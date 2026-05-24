use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::Manager;

// ── create_project ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn create_project(parent_dir: String, name: String) -> Result<String, String> {
    let project_dir = std::path::Path::new(&parent_dir).join(&name);
    std::fs::create_dir_all(project_dir.join("scenes")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("scripts")).map_err(|e| e.to_string())?;
    let meta = serde_json::json!({ "name": name, "version": "0.1.0" });
    std::fs::write(project_dir.join("project.f2proj"), meta.to_string())
        .map_err(|e| e.to_string())?;
    let scene = serde_json::json!({
        "name": "main",
        "entities": {
            "0": {
                "id": 0,
                "name": "Main Camera",
                "parent": null,
                "children": [],
                "components": [
                    {
                        "type": "Transform",
                        "x": 0.0,
                        "y": 0.0,
                        "scale_x": 1.0,
                        "scale_y": 1.0,
                        "rotation": 0.0
                    },
                    {
                        "type": "Camera",
                        "active": true,
                        "zoom": 1.0,
                        "follow_entity": null,
                        "offset_x": 0.0,
                        "offset_y": 0.0,
                        "bounds_min_x": null,
                        "bounds_min_y": null,
                        "bounds_max_x": null,
                        "bounds_max_y": null,
                        "smoothing": 1.0,
                        "dead_zone_width": 0.0,
                        "dead_zone_height": 0.0
                    }
                ],
                "active": true
            }
        },
        "next_id": 1
    });
    std::fs::write(
        project_dir.join("scenes/main.sindri"),
        serde_json::to_string_pretty(&scene).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(project_dir.to_string_lossy().to_string())
}

// ── Ollama model management + entity suggestions ──────────────────────────────

const SUGGESTION_MODEL: &str = "qwen2.5:0.5b";

fn is_small_model(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["0.5b", "135m", "360m", ":1b", "-1b", ":1.5b", "-1.5b", ":2b", "-2b"]
        .iter()
        .any(|pat| lower.contains(pat))
}

#[tauri::command]
pub async fn list_ollama_models() -> Result<Vec<String>, String> {
    let output = Command::new("ollama")
        .arg("list")
        .output()
        .map_err(|e| format!("failed to run `ollama list`: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("`ollama list` exited with status {}", output.status)
        } else {
            stderr
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let models = stdout
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect();

    Ok(models)
}

/// Finds an installed small model, or pulls qwen2.5:0.5b if none exists.
/// Returns the model name to use for suggestions.
#[tauri::command]
pub async fn ensure_suggestion_model() -> Result<String, String> {
    let client = reqwest::Client::new();

    let tags: serde_json::Value = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(models) = tags["models"].as_array() {
        for m in models {
            if let Some(name) = m["name"].as_str() {
                if is_small_model(name) {
                    return Ok(name.to_string());
                }
            }
        }
    }

    let resp = client
        .post("http://localhost:11434/api/pull")
        .json(&serde_json::json!({ "name": SUGGESTION_MODEL, "stream": false }))
        .timeout(std::time::Duration::from_secs(900))
        .send()
        .await
        .map_err(|e| format!("pull request failed: {}", e))?;

    let body = resp.text().await.map_err(|e| e.to_string())?;
    let last = body.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("");
    let parsed: serde_json::Value = serde_json::from_str(last)
        .unwrap_or(serde_json::json!({"status": "unknown"}));

    match parsed["status"].as_str() {
        Some("success") | Some("pulling manifest") => Ok(SUGGESTION_MODEL.to_string()),
        _ => Err(format!("pull did not succeed: {}", last)),
    }
}

#[derive(Serialize)]
pub struct EntitySuggestion {
    pub label: String,
    pub prompt: String,
    pub mode: String,
}

fn describe_component(c: &serde_json::Value) -> String {
    match c["type"].as_str().unwrap_or("") {
        "Transform" => format!(
            "Transform(x={}, y={}, scale={}/{})",
            c["x"], c["y"], c["scale_x"], c["scale_y"]
        ),
        "Script" => format!(
            "Script(path={})",
            c["path"].as_str().unwrap_or("none")
        ),
        "Sprite" => format!(
            "Sprite({}x{}, texture={})",
            c["width"], c["height"],
            c["texture_path"].as_str().unwrap_or("none")
        ),
        "PhysicsBody" => format!(
            "PhysicsBody(type={}, lock_rotation={})",
            c["body_type"].as_str().unwrap_or("?"),
            c["lock_rotation"].as_bool().unwrap_or(false)
        ),
        "Collider" => format!(
            "Collider({}x{}, trigger={})",
            c["width"], c["height"],
            c["is_trigger"].as_bool().unwrap_or(false)
        ),
        "Camera" => format!(
            "Camera(zoom={}, follow={:?})",
            c["zoom"],
            c["follow_entity"]
        ),
        "AudioSource" => format!(
            "AudioSource(path={}, loop={})",
            c["path"].as_str().unwrap_or("none"),
            c["looping"].as_bool().unwrap_or(false)
        ),
        other => other.to_string(),
    }
}

/// Asks a small local model for 3 actionable prompts specific to this entity.
#[tauri::command]
pub async fn generate_entity_suggestions(
    entity_name: String,
    components: Vec<serde_json::Value>,
    model: String,
) -> Result<Vec<EntitySuggestion>, String> {
    let client = reqwest::Client::new();
    let component_lines = if components.is_empty() {
        "  (no components)".to_string()
    } else {
        components
            .iter()
            .map(|c| format!("  - {}", describe_component(c)))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let prompt = format!(
        r#"Game entity "{name}":
{components}

Write 3 short, specific action prompts for THIS entity based on its actual setup above.
Reference the real component data (script filename, physics type, etc.) — do not invent generic gameplay ideas.
JSON array only, no other text. mode "send" = complete prompt, "prefill" = needs user input (end with ": ").

[
  {{"label": "Short title", "prompt": "Specific prompt referencing actual data", "mode": "send"}},
  {{"label": "Short title", "prompt": "Prompt needing detail: ", "mode": "prefill"}},
  {{"label": "Short title", "prompt": "Specific prompt", "mode": "send"}}
]"#,
        name = entity_name,
        components = component_lines,
    );

    let req_body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false,
        "options": { "temperature": 0.3, "num_predict": 256 }
    });

    let resp: serde_json::Value = client
        .post("http://localhost:11434/api/chat")
        .json(&req_body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let raw = resp["message"]["content"].as_str().unwrap_or("[]");
    let stripped = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json_start = stripped.find('[').unwrap_or(0);
    let json_end = stripped.rfind(']').map(|i| i + 1).unwrap_or(stripped.len());
    let json_slice = &stripped[json_start..json_end];

    let items: Vec<serde_json::Value> = serde_json::from_str(json_slice).unwrap_or_default();

    let suggestions = items
        .into_iter()
        .filter_map(|item| {
            let label = item["label"].as_str()?.to_string();
            let prompt = item["prompt"].as_str()?.to_string();
            let mode = item["mode"].as_str().unwrap_or("send").to_string();
            Some(EntitySuggestion { label, prompt, mode })
        })
        .take(3)
        .collect();

    Ok(suggestions)
}

// ── File operations ───────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub kind: String,
    pub children: Vec<FileNode>,
}

fn file_kind_from_ext(ext: &str) -> &'static str {
    match ext {
        "lua" => "script",
        "sindri" => "scene",
        "animclips" => "animclips",
        "prefab" => "prefab",
        "tilepallet" => "tilepallet",
        "png" | "jpg" | "jpeg" | "webp" | "bmp" => "image",
        "ogg" | "wav" | "mp3" | "flac" => "audio",
        _ => "other",
    }
}

fn build_tree(dir: &std::path::Path, relative: &str) -> FileNode {
    let name = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
    let mut children = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| {
            let is_file = e.path().is_file() as u8;
            let n = e.file_name().to_string_lossy().to_lowercase();
            (is_file, n)
        });
        for entry in entries {
            let path = entry.path();
            let entry_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            if entry_name.starts_with('.') {
                continue;
            }
            let entry_relative = if relative.is_empty() {
                entry_name.clone()
            } else {
                format!("{}/{}", relative, entry_name)
            };
            if path.is_dir() {
                children.push(build_tree(&path, &entry_relative));
            } else {
                let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                children.push(FileNode {
                    name: entry_name,
                    path: entry_relative,
                    is_dir: false,
                    kind: file_kind_from_ext(&ext).to_string(),
                    children: vec![],
                });
            }
        }
    }

    FileNode {
        name,
        path: relative.to_string(),
        is_dir: true,
        kind: "dir".to_string(),
        children,
    }
}

#[tauri::command]
pub async fn list_project_tree(project_path: String) -> Result<FileNode, String> {
    let root = std::path::Path::new(&project_path);
    if !root.exists() {
        return Err("project path does not exist".into());
    }
    Ok(build_tree(root, ""))
}

#[tauri::command]
pub async fn create_folder(project_path: String, relative_path: String) -> Result<(), String> {
    let full = std::path::Path::new(&project_path).join(&relative_path);
    let parent = full.parent().ok_or("no parent")?;
    let canon_root = std::fs::canonicalize(&project_path).map_err(|e| e.to_string())?;
    let canon_parent = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    if !canon_parent.starts_with(&canon_root) {
        return Err("path escapes project directory".into());
    }
    std::fs::create_dir(&full).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn new_scene_file(project_path: String, relative_path: String) -> Result<String, String> {
    let path = if relative_path.ends_with(".sindri") {
        relative_path
    } else {
        format!("{}.sindri", relative_path)
    };
    let full = std::path::Path::new(&project_path).join(&path);
    if full.exists() {
        return Err(format!("{} already exists", path));
    }
    let scene_name = full.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let template = format!(
        "{{\n  \"name\": \"{}\",\n  \"entities\": {{}},\n  \"next_id\": 0,\n  \"gravity_y\": 980.0,\n  \"gravity_x\": 0.0\n}}\n",
        scene_name
    );
    std::fs::write(&full, template).map_err(|e| e.to_string())?;
    Ok(path)
}

#[tauri::command]
pub async fn move_project_entry(
    project_path: String,
    from_relative: String,
    to_folder_relative: String,
) -> Result<String, String> {
    let root = std::path::Path::new(&project_path);
    let from = root.join(&from_relative);
    let to_folder = if to_folder_relative.is_empty() {
        root.to_path_buf()
    } else {
        root.join(&to_folder_relative)
    };

    let canon_root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    let canon_from = std::fs::canonicalize(&from).map_err(|e| e.to_string())?;
    let canon_to_folder = std::fs::canonicalize(&to_folder).map_err(|e| e.to_string())?;

    if !canon_from.starts_with(&canon_root) || !canon_to_folder.starts_with(&canon_root) {
        return Err("path escapes project directory".into());
    }
    if canon_to_folder.starts_with(&canon_from) {
        return Err("cannot move a folder into itself".into());
    }

    let file_name = from.file_name().ok_or("no file name")?;
    let to = to_folder.join(file_name);
    if to.exists() {
        return Err(format!("{} already exists in destination", file_name.to_string_lossy()));
    }

    std::fs::rename(&from, &to).map_err(|e| e.to_string())?;

    let new_relative = if to_folder_relative.is_empty() {
        file_name.to_string_lossy().to_string()
    } else {
        format!("{}/{}", to_folder_relative, file_name.to_string_lossy())
    };
    Ok(new_relative)
}

#[derive(Serialize)]
pub struct ProjectFile {
    pub path: String,
    pub kind: String,
    pub name: String,
}

#[tauri::command]
pub async fn list_project_files(project_path: String) -> Result<Vec<ProjectFile>, String> {
    let root = std::path::Path::new(&project_path);
    let mut files: Vec<ProjectFile> = Vec::new();

    let dirs = [
        (root.join("scripts"), "scripts"),
        (root.join("scenes"), "scenes"),
        (root.join("assets"), "assets"),
    ];

    for (dir, prefix) in &dirs {
        if !dir.exists() {
            continue;
        }
        collect_files(dir, dir, prefix, &mut files);
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn collect_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    prefix: &str,
    out: &mut Vec<ProjectFile>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let sub_prefix = format!(
                "{}/{}",
                prefix,
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            collect_files(root, &path, &sub_prefix, out);
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let kind = match ext.as_str() {
            "lua" => "script",
            "sindri" => "scene",
            "png" | "jpg" | "jpeg" | "webp" | "bmp" => "image",
            "ogg" | "wav" | "mp3" | "flac" => "audio",
            _ => "other",
        };
        let relative = format!("{}/{}", prefix, name);
        out.push(ProjectFile {
            path: relative,
            kind: kind.to_string(),
            name,
        });
    }
}

#[tauri::command]
pub async fn new_script(project_path: String, relative_path: String) -> Result<String, String> {
    let path = if relative_path.ends_with(".lua") {
        relative_path
    } else {
        format!("{}.lua", relative_path)
    };
    let full = std::path::Path::new(&project_path).join(&path);
    if full.exists() {
        return Err(format!("{} already exists", path));
    }
    std::fs::create_dir_all(full.parent().unwrap()).map_err(|e| e.to_string())?;
    let template = "function on_start(self)\nend\n\nfunction on_update(self, dt)\nend\n";
    std::fs::write(&full, template).map_err(|e| e.to_string())?;
    Ok(path)
}

#[tauri::command]
pub async fn delete_project_file(
    project_path: String,
    relative_path: String,
) -> Result<(), String> {
    let full = std::path::Path::new(&project_path).join(&relative_path);
    let canon_root = std::fs::canonicalize(&project_path).map_err(|e| e.to_string())?;
    let canon_file = std::fs::canonicalize(&full).map_err(|e| e.to_string())?;
    if !canon_file.starts_with(&canon_root) {
        return Err("path escapes project directory".into());
    }
    if full.is_dir() {
        std::fs::remove_dir_all(&full).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&full).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn write_anim_file(
    project_path: String,
    relative_path: String,
    content: String,
) -> Result<String, String> {
    let path = if relative_path.ends_with(".animclips") {
        relative_path
    } else {
        format!("{}.animclips", relative_path)
    };
    let full = std::path::Path::new(&project_path).join(&path);
    std::fs::create_dir_all(full.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&full, &content).map_err(|e| e.to_string())?;
    Ok(path)
}

#[tauri::command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_tile_palette(
    project_path: String,
    image_relative_path: String,
    name: String,
    tileset_cols: u32,
    tileset_rows: u32,
    tile_width: u32,
    tile_height: u32,
    margin: u32,
    spacing: u32,
) -> Result<String, String> {
    let stem = std::path::Path::new(&image_relative_path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let dir = std::path::Path::new(&image_relative_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let rel = if dir.is_empty() {
        format!("{}.tilepallet", stem)
    } else {
        format!("{}/{}.tilepallet", dir, stem)
    };
    let full = std::path::Path::new(&project_path).join(&rel);
    let json = serde_json::json!({
        "name": name,
        "texture_path": image_relative_path,
        "tileset_cols": tileset_cols,
        "tileset_rows": tileset_rows,
        "tile_width": tile_width,
        "tile_height": tile_height,
        "margin": margin,
        "spacing": spacing,
        "solid_tiles": []
    });
    std::fs::write(&full, serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(rel)
}

#[tauri::command]
pub async fn read_tile_palette(
    project_path: String,
    relative_path: String,
) -> Result<serde_json::Value, String> {
    let full = std::path::Path::new(&project_path).join(&relative_path);
    let content = std::fs::read_to_string(&full).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn read_project_file(project_path: String, relative_path: String) -> Result<String, String> {
    let full = std::path::Path::new(&project_path).join(&relative_path);
    std::fs::read_to_string(&full).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_project_file(
    project_path: String,
    relative_path: String,
    new_name: String,
) -> Result<String, String> {
    let full = std::path::Path::new(&project_path).join(&relative_path);
    let parent = full.parent().ok_or("no parent dir")?;
    let new_full = parent.join(&new_name);
    let canon_root = std::fs::canonicalize(&project_path).map_err(|e| e.to_string())?;
    let canon_new_parent = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    if !canon_new_parent.starts_with(&canon_root) {
        return Err("path escapes project directory".into());
    }
    std::fs::rename(&full, &new_full).map_err(|e| e.to_string())?;
    let parent_prefix = relative_path.rsplitn(2, '/').nth(1).unwrap_or("");
    Ok(if parent_prefix.is_empty() {
        new_name
    } else {
        format!("{}/{}", parent_prefix, new_name)
    })
}

// ── Project settings ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub name: String,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub pixel_art_mode: bool,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            name: "My Game".into(),
            resolution_width: 1280,
            resolution_height: 720,
            pixel_art_mode: true,
        }
    }
}

#[tauri::command]
pub async fn get_project_settings(project_path: String) -> Result<ProjectSettings, String> {
    let path = std::path::Path::new(&project_path).join("sindri_project.json");
    if !path.exists() {
        return Ok(ProjectSettings::default());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_project_settings(
    project_path: String,
    settings: ProjectSettings,
) -> Result<(), String> {
    let path = std::path::Path::new(&project_path).join("sindri_project.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

// ── Editor preferences ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorPrefs {
    #[serde(default = "default_auto_save_secs")]
    pub auto_save_interval_secs: u32,
}

fn default_auto_save_secs() -> u32 { 5 }

impl Default for EditorPrefs {
    fn default() -> Self {
        Self { auto_save_interval_secs: default_auto_save_secs() }
    }
}

fn editor_prefs_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("editor_prefs.json"))
}

#[tauri::command]
pub async fn get_editor_prefs(app: tauri::AppHandle) -> Result<EditorPrefs, String> {
    let path = editor_prefs_path(&app)?;
    if !path.exists() {
        return Ok(EditorPrefs::default());
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_editor_prefs(app: tauri::AppHandle, prefs: EditorPrefs) -> Result<(), String> {
    let path = editor_prefs_path(&app)?;
    let json = serde_json::to_string_pretty(&prefs).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

// ── Prefabs ───────────────────────────────────────────────────────────────────

use crate::engine_client::engine_url;

#[derive(Debug, Serialize, Deserialize)]
pub struct PrefabInfo {
    pub name: String,
    pub path: String,
}

fn collect_prefabs_recursive(dir: &std::path::Path, relative: &str, out: &mut Vec<PrefabInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        let rel = if relative.is_empty() { name.clone() } else { format!("{}/{}", relative, name) };
        if path.is_dir() {
            collect_prefabs_recursive(&path, &rel, out);
        } else if path.extension().map(|e| e == "prefab").unwrap_or(false) {
            let display_name = name.strip_suffix(".prefab").unwrap_or(&name).to_string();
            out.push(PrefabInfo { name: display_name, path: rel });
        }
    }
}

#[tauri::command]
pub async fn list_prefabs(project_path: String) -> Result<Vec<PrefabInfo>, String> {
    let prefabs_dir = std::path::Path::new(&project_path).join("prefabs");
    let mut out = Vec::new();
    if prefabs_dir.exists() {
        collect_prefabs_recursive(&prefabs_dir, "prefabs", &mut out);
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn entity_to_prefab_node(entity_id: u64, scene: &serde_json::Value) -> serde_json::Value {
    let entities = &scene["entities"];
    let entity = &entities[entity_id.to_string()];
    let name = entity["name"].as_str().unwrap_or("Entity");
    let components = entity["components"].clone();
    let children_ids: Vec<u64> = entity["children"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
        .unwrap_or_default();
    let children: Vec<serde_json::Value> = children_ids
        .iter()
        .map(|&cid| entity_to_prefab_node(cid, scene))
        .collect();
    serde_json::json!({ "name": name, "components": components, "children": children })
}

fn instantiate_prefab_node(
    node: &serde_json::Value,
    parent_id: Option<u64>,
    scene: &mut serde_json::Value,
    prefab_rel_path: &str,
) -> u64 {
    let next_id = scene["next_id"].as_u64().unwrap_or(1);
    scene["next_id"] = serde_json::json!(next_id + 1);

    let entity = serde_json::json!({
        "id": next_id,
        "name": node["name"],
        "parent": parent_id,
        "children": [],
        "components": node["components"],
        "active": true,
        "staged": false,
        "prefab_source": prefab_rel_path,
    });
    scene["entities"][next_id.to_string()] = entity;

    if let Some(pid) = parent_id {
        if let Some(parent_children) = scene["entities"][pid.to_string()]["children"].as_array_mut() {
            let mut arr = parent_children.clone();
            arr.push(serde_json::json!(next_id));
            scene["entities"][pid.to_string()]["children"] = serde_json::json!(arr);
        }
    }

    let children = node["children"].as_array().cloned().unwrap_or_default();
    for child in &children {
        instantiate_prefab_node(child, Some(next_id), scene, prefab_rel_path);
    }

    next_id
}

#[tauri::command]
pub async fn save_as_prefab(
    project_path: String,
    entity_id: u64,
    prefab_name: String,
) -> Result<String, String> {
    let safe_name = prefab_name.replace(['/', '\\', '.'], "_");
    let prefabs_dir = std::path::Path::new(&project_path).join("prefabs");
    std::fs::create_dir_all(&prefabs_dir).map_err(|e| e.to_string())?;
    let file_path = prefabs_dir.join(format!("{}.prefab", safe_name));
    let relative_path = format!("prefabs/{}.prefab", safe_name);

    let scene: serde_json::Value = reqwest::get(engine_url("/scene"))
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let node = entity_to_prefab_node(entity_id, &scene);
    let json = serde_json::to_string_pretty(&node).map_err(|e| e.to_string())?;
    std::fs::write(&file_path, &json).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    client
        .patch(engine_url(&format!("/scene/entity/{}/prefab_source", entity_id)))
        .json(&serde_json::json!({ "path": relative_path }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(relative_path)
}

#[tauri::command]
pub async fn instantiate_prefab(
    project_path: String,
    prefab_path: String,
    parent_id: Option<u64>,
) -> Result<u64, String> {
    let full = std::path::Path::new(&project_path).join(&prefab_path);
    let text = std::fs::read_to_string(&full).map_err(|e| e.to_string())?;
    let node: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    let mut scene: serde_json::Value = client
        .get(engine_url("/scene"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let entity_id = instantiate_prefab_node(&node, parent_id, &mut scene, &prefab_path);

    client
        .put(engine_url("/scene"))
        .json(&scene)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(entity_id)
}

#[tauri::command]
pub async fn update_prefab(entity_id: u64) -> Result<(), String> {
    let scene: serde_json::Value = reqwest::get(engine_url("/scene"))
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let entity = &scene["entities"][entity_id.to_string()];
    let prefab_source = entity["prefab_source"]
        .as_str()
        .ok_or("entity has no prefab_source")?
        .to_string();

    let scene_path: String = reqwest::get(engine_url("/scene/path"))
        .await
        .map_err(|e| e.to_string())?
        .json::<String>()
        .await
        .map_err(|e| e.to_string())?;

    let project_root = std::path::Path::new(&scene_path)
        .parent()
        .and_then(|p| p.parent())
        .ok_or("could not resolve project root")?;

    let full = project_root.join(&prefab_source);
    let node = entity_to_prefab_node(entity_id, &scene);
    let json = serde_json::to_string_pretty(&node).map_err(|e| e.to_string())?;
    std::fs::write(&full, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_from_prefab(project_path: String, entity_id: u64) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut scene: serde_json::Value = client
        .get(engine_url("/scene"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let prefab_source = scene["entities"][entity_id.to_string()]["prefab_source"]
        .as_str()
        .ok_or("entity has no prefab_source")?
        .to_string();

    let full = std::path::Path::new(&project_path).join(&prefab_source);
    let text = std::fs::read_to_string(&full).map_err(|e| e.to_string())?;
    let node: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let existing_components = scene["entities"][entity_id.to_string()]["components"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let transform = existing_components
        .iter()
        .find(|c| c["type"].as_str() == Some("Transform"))
        .cloned();

    let mut new_components: Vec<serde_json::Value> = node["components"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c["type"].as_str() != Some("Transform"))
        .collect();

    if let Some(t) = transform {
        new_components.insert(0, t);
    }

    scene["entities"][entity_id.to_string()]["components"] = serde_json::json!(new_components);
    scene["entities"][entity_id.to_string()]["name"] = node["name"].clone();

    client
        .put(engine_url("/scene"))
        .json(&scene)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn unlink_from_prefab(entity_id: u64) -> Result<(), String> {
    let client = reqwest::Client::new();
    client
        .patch(engine_url(&format!("/scene/entity/{}/prefab_source", entity_id)))
        .json(&serde_json::json!({ "path": null }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
