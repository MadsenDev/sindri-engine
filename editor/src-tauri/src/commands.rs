use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::SystemTime;
use futures_util::StreamExt;
use tauri::{Emitter, Manager};

const ENGINE_BASE: &str = "http://127.0.0.1:7878";
const KEYCHAIN_SERVICE: &str = "sindri-editor-ai";

pub struct EngineProcess(pub Mutex<Option<tokio::process::Child>>);

fn find_workspace_root() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().unwrap_or(&exe).to_path_buf();
        for _ in 0..8 {
            if dir.join("Cargo.toml").exists() && dir.join("crates").exists() {
                return Some(dir);
            }
            match dir.parent() {
                Some(p) => dir = p.to_path_buf(),
                None => break,
            }
        }
    }

    std::env::current_dir().ok().and_then(|mut dir| {
        loop {
            if dir.join("Cargo.toml").exists() && dir.join("crates").exists() {
                return Some(dir);
            }
            if !dir.pop() {
                return None;
            }
        }
    })
}

fn engine_binary_in_workspace(root: &Path) -> Option<PathBuf> {
    let debug = root.join("target/debug/sindri-server");
    if debug.exists() {
        return Some(debug);
    }
    let release = root.join("target/release/sindri-server");
    if release.exists() {
        return Some(release);
    }
    None
}

fn newest_mtime(path: &Path) -> Option<SystemTime> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.is_file() {
        return metadata.modified().ok();
    }

    let mut newest = metadata.modified().ok();
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path).ok()? {
            let entry = entry.ok()?;
            if let Some(modified) = newest_mtime(&entry.path()) {
                if newest.map_or(true, |current| modified > current) {
                    newest = Some(modified);
                }
            }
        }
    }
    newest
}

fn engine_binary_is_stale(root: &Path, binary: &Path) -> bool {
    let Some(binary_time) = std::fs::metadata(binary)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
    else {
        return true;
    };

    [
        "Cargo.toml",
        "Cargo.lock",
        "crates/sindri-server/Cargo.toml",
        "crates/sindri-server/src",
        "crates/sindri/Cargo.toml",
        "crates/sindri/src",
    ]
    .iter()
    .map(|path| root.join(path))
    .filter_map(|path| newest_mtime(&path))
    .any(|source_time| source_time > binary_time)
}

async fn build_engine_binary(root: &Path, reason: &str) -> Result<PathBuf, String> {
    eprintln!("[start_engine] {reason}; running cargo build -p sindri-server");
    let output = tokio::process::Command::new("cargo")
        .args(["build", "-p", "sindri-server"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| format!("Failed to run cargo build -p sindri-server: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to build sindri-server.\n\nstdout:\n{}\n\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    engine_binary_in_workspace(root).ok_or_else(|| {
        format!(
            "cargo build -p sindri-server succeeded, but no binary was found under {}",
            root.join("target").display()
        )
    })
}

fn find_engine_binary() -> PathBuf {
    if let Ok(path) = std::env::var("SINDRI_ENGINE_BIN") {
        return PathBuf::from(path);
    }
    if let Some(root) = find_workspace_root() {
        if let Some(binary) = engine_binary_in_workspace(&root) {
            return binary;
        }
    }
    PathBuf::from("sindri-server")
}

async fn ensure_engine_binary() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("SINDRI_ENGINE_BIN") {
        let binary = PathBuf::from(path);
        if binary.exists() {
            return Ok(binary);
        }
        return Err(format!(
            "SINDRI_ENGINE_BIN points to a missing file: {}",
            binary.display()
        ));
    }

    if let Some(root) = find_workspace_root() {
        if let Some(binary) = engine_binary_in_workspace(&root) {
            if !engine_binary_is_stale(&root, &binary) {
                return Ok(binary);
            }
            return build_engine_binary(&root, "sindri-server is older than its sources").await;
        }

        return build_engine_binary(&root, "sindri-server is missing").await;
    }

    let fallback = PathBuf::from("sindri-server");
    if fallback.exists() {
        return Ok(fallback);
    }

    Err(
        "Engine binary not found and the Sindri workspace root could not be located. Set SINDRI_ENGINE_BIN to a sindri-server binary.".into(),
    )
}

#[tauri::command]
pub async fn start_engine(
    project_dir: String,
    state: tauri::State<'_, EngineProcess>,
) -> Result<(), String> {
    let binary = ensure_engine_binary().await?;
    eprintln!("[start_engine] binary: {}", binary.display());

    // Take any existing child out before awaiting so the MutexGuard is dropped.
    let old_child = state.0.lock().unwrap().take();
    if let Some(mut child) = old_child {
        let _ = child.kill().await;
    }

    // Kill any orphaned sindri-server from a previous session.
    #[cfg(unix)]
    let _ = tokio::process::Command::new("pkill")
        .args(["-f", "sindri-server"])
        .output()
        .await;
    #[cfg(windows)]
    let _ = tokio::process::Command::new("taskkill")
        .args(["/f", "/im", "sindri-server.exe"])
        .output()
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let child = tokio::process::Command::new(&binary)
        .arg("--project-dir")
        .arg(&project_dir)
        .arg("--headless")
        .spawn()
        .map_err(|e| format!("Failed to spawn engine ({}): {}", binary.display(), e))?;
    *state.0.lock().unwrap() = Some(child);

    // Wait up to 10 seconds for the engine to respond.
    let client = reqwest::Client::new();
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if client
            .get("http://127.0.0.1:7878/health")
            .send()
            .await
            .is_ok()
        {
            return Ok(());
        }
    }

    Err(format!(
        "Engine started ({}) but didn't respond on :7878 within 10s. Check terminal output for errors.",
        binary.display()
    ))
}

#[tauri::command]
pub async fn get_engine_binary_path() -> String {
    find_engine_binary().to_string_lossy().to_string()
}

#[tauri::command]
pub async fn stop_engine(state: tauri::State<'_, EngineProcess>) -> Result<(), String> {
    let old_child = state.0.lock().unwrap().take();
    if let Some(mut child) = old_child {
        child.kill().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

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

fn engine_url(path: &str) -> String {
    format!("{}{}", ENGINE_BASE, path)
}

#[tauri::command]
pub async fn get_scene() -> Result<String, String> {
    reqwest::get(engine_url("/scene"))
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
pub struct EngineStatus {
    pub status: String,
    pub model: String,
    pub paused: bool,
    pub playback: Option<String>,
    pub error_count: Option<usize>,
}

#[tauri::command]
pub async fn get_engine_status() -> Result<EngineStatus, String> {
    reqwest::get(engine_url("/health"))
        .await
        .map_err(|e| e.to_string())?
        .json::<EngineStatus>()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn put_scene(scene_json: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .put(engine_url("/scene"))
        .body(scene_json)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|e| e.to_string()));
    }
    Ok(())
}

#[tauri::command]
pub async fn save_scene() -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(engine_url("/scene/save"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|e| e.to_string()));
    }
    Ok(())
}

#[tauri::command]
pub async fn open_scene_file(project_path: String, relative_path: String) -> Result<(), String> {
    let root = Path::new(&project_path);
    let full = root.join(&relative_path);
    let canon_root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    let canon_file = std::fs::canonicalize(&full).map_err(|e| e.to_string())?;
    if !canon_file.starts_with(&canon_root) {
        return Err("path escapes project directory".into());
    }
    let ext = canon_file
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if ext != "sindri" {
        return Err("only .sindri scene files can be opened".into());
    }

    let scene_json = std::fs::read_to_string(&canon_file).map_err(|e| e.to_string())?;
    // Validate that this is JSON locally; the server performs full Scene parsing.
    serde_json::from_str::<serde_json::Value>(&scene_json).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    let resp = client
        .post(engine_url("/scene/open"))
        .json(&serde_json::json!({ "path": relative_path }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::METHOD_NOT_ALLOWED
        {
            return Err(
                "The running sindri-server does not support scene opening yet. Restart the editor so it can rebuild and relaunch the engine sidecar.".into(),
            );
        }
        return Err(resp.text().await.unwrap_or_else(|e| e.to_string()));
    }
    Ok(())
}

#[tauri::command]
pub async fn patch_transform(
    entity_id: u64,
    x: Option<f32>,
    y: Option<f32>,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
    rotation: Option<f32>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "x": x, "y": y, "scale_x": scale_x, "scale_y": scale_y, "rotation": rotation });
    client
        .patch(engine_url(&format!(
            "/scene/entity/{}/transform",
            entity_id
        )))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_gizmos(enabled: bool) -> Result<(), String> {
    let client = reqwest::Client::new();
    client
        .post(engine_url("/gizmos"))
        .json(&serde_json::json!({ "enabled": enabled }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_screenshot() -> Result<String, String> {
    let resp: serde_json::Value = reqwest::get(engine_url("/screenshot"))
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp["image"].as_str().unwrap_or("").to_string())
}

#[tauri::command]
pub async fn get_runtime_errors() -> Result<Vec<String>, String> {
    let resp: serde_json::Value = reqwest::get(engine_url("/errors"))
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp["errors"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(String::from))
        .collect())
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContextFlags {
    pub include_scene: bool,
    pub include_script: bool,
    pub include_viewport: bool,
    pub include_errors: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenScriptContext {
    pub path: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct AiResponse {
    pub text: String,
    pub actions: Vec<serde_json::Value>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiStreamEvent {
    pub request_id: String,
    pub kind: String,
    pub text: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatus {
    pub provider: String,
    pub configured: bool,
    pub default_model: String,
}

fn default_cloud_model(provider: &str) -> &'static str {
    match provider {
        "openai" => "gpt-5-mini",
        "anthropic" => "claude-sonnet-4-20250514",
        _ => "qwen2.5-coder:7b",
    }
}

fn keychain_entry(provider: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, provider).map_err(|e| e.to_string())
}

fn provider_api_key(provider: &str) -> Result<String, String> {
    match provider {
        "openai" => std::env::var("OPENAI_API_KEY")
            .ok()
            .or_else(|| keychain_entry("openai").ok()?.get_password().ok())
            .ok_or_else(|| "OpenAI API key is not configured".to_string()),
        "anthropic" => std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .or_else(|| keychain_entry("anthropic").ok()?.get_password().ok())
            .ok_or_else(|| "Anthropic API key is not configured".to_string()),
        _ => Err("Local Ollama does not use an API key".to_string()),
    }
}

#[tauri::command]
pub async fn get_ai_provider_status() -> Result<Vec<AiProviderStatus>, String> {
    Ok(vec![
        AiProviderStatus {
            provider: "ollama".into(),
            configured: reqwest::get("http://localhost:11434/api/tags").await.is_ok(),
            default_model: "qwen2.5-coder:7b".into(),
        },
        AiProviderStatus {
            provider: "openai".into(),
            configured: provider_api_key("openai").is_ok(),
            default_model: default_cloud_model("openai").into(),
        },
        AiProviderStatus {
            provider: "anthropic".into(),
            configured: provider_api_key("anthropic").is_ok(),
            default_model: default_cloud_model("anthropic").into(),
        },
    ])
}

#[tauri::command]
pub async fn save_ai_api_key(provider: String, api_key: String) -> Result<(), String> {
    if provider != "openai" && provider != "anthropic" {
        return Err("Only OpenAI and Anthropic keys can be saved".into());
    }
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("API key cannot be empty".into());
    }
    keychain_entry(&provider)?.set_password(api_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_ai_api_key(provider: String) -> Result<(), String> {
    if provider != "openai" && provider != "anthropic" {
        return Err("Only OpenAI and Anthropic keys can be cleared".into());
    }
    match keychain_entry(&provider)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn test_ai_provider(provider: String, model: Option<String>) -> Result<(), String> {
    let client = reqwest::Client::new();
    let model = model.unwrap_or_else(|| default_cloud_model(&provider).to_string());
    let messages = vec![serde_json::json!({
        "role": "user",
        "content": "Reply with exactly: ok"
    })];
    let _ = request_ai_text(&client, &provider, &model, messages).await?;
    Ok(())
}

async fn request_ai_text(
    client: &reqwest::Client,
    provider: &str,
    model: &str,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let text = match provider {
        "openai" => request_openai_text(client, model, messages).await,
        "anthropic" => request_anthropic_text(client, model, messages).await,
        _ => request_ollama_text(client, model, messages).await,
    }?;

    if text.trim().is_empty() {
        Err(format!("{provider} returned an empty response for model `{model}`"))
    } else {
        Ok(text)
    }
}

async fn request_ollama_text(
    client: &reqwest::Client,
    model: &str,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let resp = client
        .post("http://localhost:11434/api/chat")
        .json(&serde_json::json!({ "model": model, "messages": messages, "stream": false }))
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {e}"))?;

    let status = resp.status();
    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Ollama returned invalid JSON: {e}"))?;

    if !status.is_success() {
        let detail = value["error"].as_str().unwrap_or("unknown Ollama error");
        return Err(format!("Ollama error ({status}): {detail}"));
    }
    if let Some(error) = value["error"].as_str() {
        return Err(format!("Ollama error: {error}"));
    }

    value["message"]["content"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("Ollama response did not include message.content: {value}"))
}

async fn request_openai_text(
    client: &reqwest::Client,
    model: &str,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let key = provider_api_key("openai")?;
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }
    let value: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    value["choices"][0]["message"]["content"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("OpenAI response did not include choices[0].message.content: {value}"))
}

fn anthropic_content_from_openai(content: &serde_json::Value) -> serde_json::Value {
    if let Some(text) = content.as_str() {
        return serde_json::json!(text);
    }
    let Some(items) = content.as_array() else {
        return serde_json::json!(content.to_string());
    };
    let mut blocks = Vec::new();
    for item in items {
        match item["type"].as_str() {
            Some("text") => blocks.push(serde_json::json!({
                "type": "text",
                "text": item["text"].as_str().unwrap_or("")
            })),
            Some("image_url") => {
                if let Some(url) = item["image_url"]["url"].as_str() {
                    if let Some(data) = url.strip_prefix("data:image/png;base64,") {
                        blocks.push(serde_json::json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": data
                            }
                        }));
                    }
                }
            }
            _ => {}
        }
    }
    serde_json::json!(blocks)
}

async fn request_anthropic_text(
    client: &reqwest::Client,
    model: &str,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let key = provider_api_key("anthropic")?;
    let mut system = String::new();
    let mut anthropic_messages = Vec::new();
    for message in messages {
        let role = message["role"].as_str().unwrap_or("user");
        if role == "system" {
            system = message["content"].as_str().unwrap_or("").to_string();
            continue;
        }
        anthropic_messages.push(serde_json::json!({
            "role": if role == "assistant" { "assistant" } else { "user" },
            "content": anthropic_content_from_openai(&message["content"])
        }));
    }
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": model,
            "system": system,
            "messages": anthropic_messages,
            "max_tokens": 4096,
            "stream": false
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }
    let value: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let text = value["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .ok_or_else(|| format!("Anthropic response did not include content blocks: {value}"))?;
    Ok(text)
}

fn emit_ai_stream(app: &tauri::AppHandle, request_id: &str, kind: &str, text: impl Into<String>) {
    let _ = app.emit("ai://stream", AiStreamEvent {
        request_id: request_id.to_string(),
        kind: kind.to_string(),
        text: text.into(),
    });
}

async fn stream_ollama_text(
    app: tauri::AppHandle,
    request_id: String,
    client: reqwest::Client,
    model: String,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let resp = client
        .post("http://localhost:11434/api/chat")
        .json(&serde_json::json!({ "model": model, "messages": messages, "stream": true }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }

    let mut full = String::new();
    let mut in_think = false;
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();
            if line.is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(&line).unwrap_or_default();
            let delta = value["message"]["content"].as_str().unwrap_or("");
            if delta.is_empty() {
                continue;
            }
            full.push_str(delta);
            let mut visible = delta.to_string();
            if visible.contains("<think>") {
                in_think = true;
                visible = visible.replace("<think>", "");
            }
            if visible.contains("</think>") {
                in_think = false;
                visible = visible.replace("</think>", "");
            }
            if !visible.is_empty() {
                emit_ai_stream(&app, &request_id, if in_think { "thinking" } else { "delta" }, visible);
            }
        }
    }
    Ok(full)
}

async fn stream_openai_text(
    app: tauri::AppHandle,
    request_id: String,
    client: reqwest::Client,
    model: String,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let key = provider_api_key("openai")?;
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&serde_json::json!({ "model": model, "messages": messages, "stream": true }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }

    let mut full = String::new();
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();
            let Some(data) = line.strip_prefix("data: ") else { continue; };
            if data == "[DONE]" {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(data).unwrap_or_default();
            let delta = value["choices"][0]["delta"]["content"].as_str().unwrap_or("");
            if !delta.is_empty() {
                full.push_str(delta);
                emit_ai_stream(&app, &request_id, "delta", delta);
            }
        }
    }
    Ok(full)
}

async fn stream_anthropic_text(
    app: tauri::AppHandle,
    request_id: String,
    client: reqwest::Client,
    model: String,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let key = provider_api_key("anthropic")?;
    let mut system = String::new();
    let mut anthropic_messages = Vec::new();
    for message in messages {
        let role = message["role"].as_str().unwrap_or("user");
        if role == "system" {
            system = message["content"].as_str().unwrap_or("").to_string();
            continue;
        }
        anthropic_messages.push(serde_json::json!({
            "role": if role == "assistant" { "assistant" } else { "user" },
            "content": anthropic_content_from_openai(&message["content"])
        }));
    }
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": model,
            "system": system,
            "messages": anthropic_messages,
            "max_tokens": 4096,
            "stream": true
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }

    let mut full = String::new();
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();
            let Some(data) = line.strip_prefix("data: ") else { continue; };
            let value: serde_json::Value = serde_json::from_str(data).unwrap_or_default();
            let delta = &value["delta"];
            match delta["type"].as_str() {
                Some("text_delta") => {
                    let text = delta["text"].as_str().unwrap_or("");
                    if !text.is_empty() {
                        full.push_str(text);
                        emit_ai_stream(&app, &request_id, "delta", text);
                    }
                }
                Some("thinking_delta") => {
                    let text = delta["thinking"].as_str().unwrap_or("");
                    if !text.is_empty() {
                        emit_ai_stream(&app, &request_id, "thinking", text);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(full)
}

#[tauri::command]
pub async fn send_ai_message_stream(
    app: tauri::AppHandle,
    request_id: String,
    message: String,
    provider: Option<String>,
    model: Option<String>,
    history: Vec<serde_json::Value>,
) -> Result<(), String> {
    let provider = provider.unwrap_or_else(|| "ollama".to_string());
    let model = model.unwrap_or_else(|| default_cloud_model(&provider).to_string());
    let client = reqwest::Client::new();
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": "You are Sindri's embedded AI assistant. Keep responses concise and practical."
    })];
    let recent_history = if history.len() > 10 { &history[history.len() - 10..] } else { &history[..] };
    messages.extend_from_slice(recent_history);
    messages.push(serde_json::json!({ "role": "user", "content": message }));

    let result = match provider.as_str() {
        "openai" => stream_openai_text(app.clone(), request_id.clone(), client, model, messages).await,
        "anthropic" => stream_anthropic_text(app.clone(), request_id.clone(), client, model, messages).await,
        _ => stream_ollama_text(app.clone(), request_id.clone(), client, model, messages).await,
    };

    match result {
        Ok(full) => emit_ai_stream(&app, &request_id, "done", full),
        Err(err) => emit_ai_stream(&app, &request_id, "error", err),
    }
    Ok(())
}

#[tauri::command]
pub async fn send_ai_message(
    message: String,
    context_flags: ContextFlags,
    provider: Option<String>,
    model: Option<String>,
    history: Vec<serde_json::Value>, // [{role, content}] pairs from prior turns
    open_script: Option<OpenScriptContext>,
) -> Result<AiResponse, String> {
    let client = reqwest::Client::new();

    // Fetch context based on flags
    let scene = if context_flags.include_scene {
        match reqwest::get(engine_url("/scene")).await {
            Ok(r) => r.text().await.ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let screenshot = if context_flags.include_viewport {
        let resp: Option<serde_json::Value> = match reqwest::get(engine_url("/screenshot")).await {
            Ok(r) => r.json().await.ok(),
            Err(_) => None,
        };
        resp.and_then(|v| v["image"].as_str().map(String::from))
    } else {
        None
    };

    let errors = if context_flags.include_errors {
        let resp: Option<serde_json::Value> = match reqwest::get(engine_url("/errors")).await {
            Ok(r) => r.json().await.ok(),
            Err(_) => None,
        };
        resp.and_then(|v| {
            v["errors"].as_array().map(|errors| {
                errors
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        })
    } else {
        None
    };

    let use_vision = screenshot.is_some();
    let provider = provider.unwrap_or_else(|| "ollama".to_string());
    let model = model.unwrap_or_else(|| {
        if provider == "ollama" {
            if use_vision { "qwen2.5-vl:7b" } else { "qwen2.5-coder:7b" }.to_string()
        } else {
            default_cloud_model(&provider).to_string()
        }
    });

    let engine_ref = include_str!("../../../ENGINE_REFERENCE.md");
    let system_prompt = format!(
        r#"{engine_ref}

---

You are an AI assistant embedded in the Sindri editor. You have FULL CONTROL over the editor and the running engine.

You have access to the current scene (as JSON), optionally an open script file, and optionally a screenshot of the game viewport.

You may respond in two ways:
1. Plain text explanation or analysis.
2. A JSON action block wrapped in <action>...</action> tags.

Action block schema — use as many actions as needed in one block:
{{
  "actions": [
    // Scene structure
    {{ "type": "create_entity", "name": "Coin", "parent_id": null }},
    {{ "type": "delete_entity", "entity_id": 3 }},
    {{ "type": "rename_entity", "entity_id": 1, "name": "Player" }},

    // Components — use entity_name when you just created the entity and don't know its ID yet
    {{ "type": "add_component", "entity_name": "Player", "component_type": "Transform" }},
    {{ "type": "add_component", "entity_name": "Player", "component_type": "PhysicsBody" }},
    {{ "type": "patch_component", "entity_name": "Player", "component_type": "PhysicsBody", "data": {{ "body_type": "Dynamic", "lock_rotation": true }} }},
    {{ "type": "patch_component", "entity_name": "Player", "component_type": "Sprite", "data": {{ "width": 48, "height": 48, "color": [0.3, 0.6, 1.0, 1.0] }} }},
    {{ "type": "add_component", "entity_name": "Ground", "component_type": "PhysicsBody" }},
    {{ "type": "patch_component", "entity_name": "Ground", "component_type": "PhysicsBody", "data": {{ "body_type": "Fixed" }} }},
    {{ "type": "add_component", "entity_name": "Player", "component_type": "Collider" }},
    {{ "type": "patch_component", "entity_name": "Player", "component_type": "Collider", "data": {{ "width": 32, "height": 48, "is_trigger": false }} }},
    {{ "type": "add_component", "entity_id": 1, "component_type": "Script" }},
    {{ "type": "remove_component", "entity_id": 1, "component_type": "Script" }},
    {{ "type": "patch_component", "entity_id": 1, "component_idx": 0, "data": {{ "path": "scripts/player.lua" }} }},
    {{ "type": "patch_component", "entity_id": 1, "component_type": "Camera", "data": {{ "active": true, "zoom": 1.0, "follow_entity": null, "offset_x": 0.0, "offset_y": 0.0, "smoothing": 1.0, "dead_zone_width": 0.0, "dead_zone_height": 0.0 }} }},
    {{ "type": "patch_component", "entity_id": 1, "component_type": "AudioSource", "data": {{ "path": "audio/jump.ogg", "volume": 0.8, "looping": false, "play_on_start": false }} }},

    // Transform
    {{ "type": "edit_transform", "entity_name": "Player", "x": 400, "y": 260, "scale_x": 1.0, "scale_y": 1.0, "rotation": 0.0 }},
    {{ "type": "edit_transform", "entity_name": "Ground", "x": 400, "y": 340, "scale_x": 5.0, "scale_y": 1.0, "rotation": 0.0 }},

    // Scripts
    {{ "type": "write_script", "path": "scripts/beacon.lua", "content": "function on_update(self, dt)\n  self.rotation = self.rotation + 360 * dt\nend" }},
    // PREFERRED for new script-driven entities: write script + add Script component + set path in one shot
    // IMPORTANT: also add a Transform if the entity doesn't have one
    {{ "type": "attach_script", "entity_name": "Player", "path": "scripts/player.lua", "content": "function on_update(self, dt)\n  ...\nend" }},
    {{ "type": "attach_script", "entity_id": 1, "path": "scripts/player.lua", "content": "..." }},

    // Suggestion (shown as a card for user approval)
    {{ "type": "suggest_fix", "description": "Collider height doesn't match sprite height", "entity_id": 0 }}
  ]
}}

CRITICAL RULE — entity references:
- For entities already in the scene: use `"entity_id"` with the numeric ID from the scene JSON.
- For entities you are creating in the same action block: you do NOT know the ID yet. Use `"entity_name": "EntityName"` instead — the executor resolves it by looking up the name in the scene after the entity is created.
- NEVER guess an entity_id (like 0) for a newly created entity. Always use entity_name for those.

Rules:
- Coordinate system: +X moves right, +Y moves down. "Below" means a larger y value than the reference entity. "Above" means a smaller y value. "Left of" means smaller x; "right of" means larger x.
- For vertical relationships like "ground below player", keep x aligned unless the user asks for horizontal offset. Example: Player at x=400,y=260 and Ground at x=400,y=340.
- When creating a player and a ground/platform, place the player above the ground, not beside it. Use explicit edit_transform actions for both entities.
- Physics needs both PhysicsBody and Collider. Use PhysicsBody body_type "Dynamic" for moving players/enemies, "Fixed" for ground/walls/platforms, and "Kinematic" for scripted moving platforms. Add Collider for collision shape/trigger data.
- For platformer-style players, set PhysicsBody lock_rotation=true.
- Supported scene component types are exactly: Transform, Sprite, AnimatedSprite, Tilemap, PhysicsBody, Collider, Script, Camera, AudioSource. You may add, remove, and patch these scene components.
- PLAYER ENTITY TEMPLATE — when creating a player, ALWAYS include ALL of these actions: (1) create_entity "Player", (2) add_component Transform, (3) add_component Sprite + patch color/size, (4) add_component PhysicsBody + patch body_type Dynamic lock_rotation true, (5) add_component Collider + patch width/height to match sprite, (6) attach_script with movement Lua code. An entity with only create_entity and no components is useless.
- Do NOT create unsupported engine-only components such as ParticleEmitter, PointLight, DirectionalLight, HUD, PathfindingGrid, or gameplay marker components through AI actions. If asked for one of these, explain that editor/AI scene support is not implemented yet and suggest Lua/scripted or Rust-side alternatives.
- `patch_component` data fields: Sprite supports texture_path, width, height, flip_x, flip_y, color [r,g,b,a]; AnimatedSprite supports texture_path, cols, rows, width, height, flip_x, flip_y, tint, margin, spacing, clips (array of clip objects), default_clip; Tilemap supports palettes (array of TilePalette objects: {{name, texture_path, tileset_cols, tileset_rows, margin, spacing, solid_tiles (0-based tile indices within this palette that are solid)}}), tile_width, tile_height, map_cols, map_rows, tiles (flat u32 array: 0=empty, upper 16 bits=palette_id (1-indexed), lower 16 bits=tile_idx (0-indexed)), tint; solid tiles per palette auto-generate static physics colliders on play; Collider supports width, height, offset_x, offset_y, is_trigger; PhysicsBody supports body_type, lock_rotation, linear_damping, angular_damping, collision_layer, collision_mask; Script supports path; Camera supports active, zoom, follow_entity, offset_x, offset_y, bounds_min_x, bounds_min_y, bounds_max_x, bounds_max_y, smoothing, dead_zone_width, dead_zone_height; AudioSource supports path, volume, looping, play_on_start.
- Scripts are Lua 5.4 with a CUSTOM Sindri engine API. Do NOT use LÖVE2D (`love.*`), Unity, Godot, or any other engine's API.
- `self` is an engine userdata object — NOT a plain table. Access everything through method calls.
- Sindri script API summary: hooks are `on_start(self)` / `on_update(self, dt)`. Key facets: `self:input()` → InputFacet, `self:transform()` → TransformFacet|nil, `self:physics()` → PhysicsFacet|nil, `self:sprite()` → SpriteFacet|nil. Global: `vec2(x,y)`.
- InputFacet: `input:is_key_down("A")`, `input:is_key_pressed("Space")`, `input:axis("A","D")` → -1..1. Key names: single letters `"A"`–`"Z"`, arrows `"Left"` `"Right"` `"Up"` `"Down"`, `"Space"`, `"Enter"`, `"Escape"`.
- TransformFacet: `t:position()` → Vec2, `t:set_position(vec2(x,y))`, `t:set_rotation(r)`, `t:set_scale(vec2(sx,sy))`. Always nil-check: `local t = self:transform(); if t == nil then return end`.
- PhysicsFacet: `p:velocity()` → Vec2, `p:set_velocity(vec2(vx,vy))`, `p:apply_impulse(vec2(ix,iy))`, `p:contacts()` → table of entity name strings currently touching this entity (e.g. `{{"Player","Ground"}}`). Nil if entity has no PhysicsBody component.
- Global `entity_transform(name)` → `{{x, y, rotation}}` or nil. Returns the named entity's transform as a snapshot for this frame. Example: `local t = entity_transform("Drone"); if t then local dx = t.x - self.x end`.
- SpriteFacet: `spr:set_tint({{r,g,b,a}})`, `spr:set_visible(bool)`. Nil if no Sprite component.
- AnimatedSpriteFacet: `local anim = self:animated_sprite()` — nil if no AnimatedSprite component. Methods: `anim:play("clip_name")` (switches clip, no-op if already playing), `anim:set_flip_x(bool)`, `anim:set_flip_y(bool)`, `anim:current_clip()` → string. Use `play` every frame based on state; it is idempotent. Example walk pattern: `local anim = self:animated_sprite(); if anim ~= nil then if h > 0.1 then anim:play("walk_right") elseif h < -0.1 then anim:play("walk_left") else anim:play("idle") end end`
- Correct platformer movement: `local h = input:axis("A","D"); phys:set_velocity(vec2(h * speed, phys:velocity().y))`
- ATTACH/FOLLOW PATTERN — to ride/follow another entity, use `contacts()` to detect the touch, then `entity_transform("Name")` to get its position/rotation: `local contacts = self:physics():contacts(); for _, name in ipairs(contacts) do if name == "Drone" then local dt = entity_transform("Drone"); if dt then self:transform():set_rotation(dt.rotation); self:transform():set_position(vec2(dt.x, dt.y - 32)); end end end`
- WRONG (do not use): `love.keyboard.isDown`, `Input.GetKey`, `key_down()` global, `self.x`/`self.y` field access, `self:move_and_slide()`
- When the user asks you to make an entity do something with scripting, use `attach_script` — it writes the file AND wires up the Script component in one action. Also add a Transform component if the entity doesn't have one.
- When the user explicitly references an existing script file like `#scripts/beacon.lua`, prefer a `write_script` action that edits that file directly instead of unrelated scene actions.
- `patch_component` with `entity_name` + `component_type` is the preferred form. The `entity_id` + `component_idx` form also works but is less readable.
- Always respond with a brief plain-text explanation first, then the action block.
- Only include an action block when making or suggesting a concrete change.
- Keep explanations concise — one or two sentences.
- entity_id values come from the scene JSON's "id" fields (u64 integers)."#,
        engine_ref = engine_ref,
    );

    let mut context_text = String::new();
    if let Some(s) = &scene {
        context_text.push_str(&format!("Scene:\n```json\n{}\n```\n\n", s));
    }
    if let Some(script) = &open_script {
        if context_flags.include_script {
            context_text.push_str(&format!(
                "Open script ({path}):\n```lua\n{content}\n```\n\n",
                path = script.path,
                content = script.content
            ));
        }
    }
    if let Some(errors) = &errors {
        if !errors.is_empty() {
            context_text.push_str(&format!("Errors:\n```\n{}\n```\n\n", errors));
        }
    }
    context_text.push_str(&format!("User message: {}", message));

    let user_content: serde_json::Value = if use_vision {
        serde_json::json!([
            { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{}", screenshot.unwrap_or_default()) } },
            { "type": "text", "text": context_text }
        ])
    } else {
        serde_json::json!(context_text)
    };

    // Build messages: system + history (last N turns) + current user message
    let mut messages: Vec<serde_json::Value> =
        vec![serde_json::json!({ "role": "system", "content": system_prompt })];
    // Include up to the last 10 history messages (5 exchanges)
    let recent_history = if history.len() > 10 {
        &history[history.len() - 10..]
    } else {
        &history[..]
    };
    messages.extend_from_slice(recent_history);
    messages.push(serde_json::json!({ "role": "user", "content": user_content }));

    let raw_text = request_ai_text(&client, &provider, &model, messages).await?;

    // Parse action block
    let (text, mut actions) =
        if let (Some(start), Some(end)) = (raw_text.find("<action>"), raw_text.find("</action>")) {
            let before = raw_text[..start].trim().to_string();
            let action_json = &raw_text[start + 8..end];
            let parsed: serde_json::Value =
                serde_json::from_str(action_json.trim()).unwrap_or(serde_json::json!({}));
            let actions = parsed["actions"].as_array().cloned().unwrap_or_default();
            (before, actions)
        } else {
            (raw_text, vec![])
        };

    normalize_script_actions(&message, open_script.as_ref(), &mut actions);
    normalize_spatial_actions(&message, scene.as_deref(), &mut actions);

    Ok(AiResponse { text, actions })
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScriptBackup {
    pub path: String,
    pub existed: bool,
    pub content: String,
}

#[derive(Serialize)]
pub struct ProposalChange {
    pub id: String,
    pub label: String,
    pub detail: String,
    /// Entity IDs that were newly created (staged=true) by this change — deleted on reject.
    pub staged_entity_ids: Vec<u64>,
    /// Entity IDs of pre-existing entities modified by this change — just unstaged on reject.
    pub modified_entity_ids: Vec<u64>,
    /// Script paths written by this change (for cleanup on reject).
    pub new_script_paths: Vec<String>,
    /// Previous script contents so reject can restore instead of deleting user files.
    pub script_backups: Vec<ScriptBackup>,
}

#[derive(Serialize)]
pub struct ProposalResponse {
    pub prompt: String,
    pub summary: String,
    pub changes: Vec<ProposalChange>,
}

#[tauri::command]
pub async fn generate_proposal(
    message: String,
    context_flags: ContextFlags,
    provider: Option<String>,
    model: Option<String>,
    history: Vec<serde_json::Value>,
    open_script: Option<OpenScriptContext>,
) -> Result<ProposalResponse, String> {
    let client = reqwest::Client::new();

    let scene = if context_flags.include_scene {
        match reqwest::get(engine_url("/scene")).await {
            Ok(r) => r.text().await.ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let screenshot = if context_flags.include_viewport {
        let resp: Option<serde_json::Value> = match reqwest::get(engine_url("/screenshot")).await {
            Ok(r) => r.json().await.ok(),
            Err(_) => None,
        };
        resp.and_then(|v| v["image"].as_str().map(String::from))
    } else {
        None
    };

    let errors = if context_flags.include_errors {
        let resp: Option<serde_json::Value> = match reqwest::get(engine_url("/errors")).await {
            Ok(r) => r.json().await.ok(),
            Err(_) => None,
        };
        resp.and_then(|v| {
            v["errors"].as_array().map(|errs| {
                errs.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n")
            })
        })
    } else {
        None
    };

    let use_vision = screenshot.is_some();
    let provider = provider.unwrap_or_else(|| "ollama".to_string());
    let model = model.unwrap_or_else(|| {
        if provider == "ollama" {
            if use_vision { "qwen2.5-vl:7b" } else { "qwen2.5-coder:7b" }.to_string()
        } else {
            default_cloud_model(&provider).to_string()
        }
    });

    let engine_ref = include_str!("../../../ENGINE_REFERENCE.md");
    let system_prompt = format!(
        r#"{engine_ref}

---

You are an AI assistant embedded in the Sindri editor. Respond with a SINGLE JSON object — no markdown fences, no prose outside the JSON.

Format:
{{
  "summary": "One sentence describing the proposed changes (or your answer if no changes).",
  "changes": [
    {{
      "id": "c1",
      "label": "Short human-readable title (e.g. 'Create Player entity')",
      "detail": "What this specific change does and why.",
      "actions": [
        {{ ... action object ... }},
        {{ ... action object ... }}
      ]
    }}
  ]
}}

If the user is asking a question with no scene changes, use "changes": [].
If the user asks you to add, fix, update, remove, create, or make something, you MUST include at least one concrete action in "changes". Never claim that you changed something unless "changes" contains the action that performs it.

Each change groups ALL actions needed for one logical operation. A "Create Player" change needs multiple actions in its "actions" array — create entity, add every component, set script, etc.

Action types — use as many as needed in one change's "actions" array:
{{ "type": "create_entity", "name": "Coin", "parent_id": null }}
{{ "type": "delete_entity", "entity_id": 3 }}
{{ "type": "rename_entity", "entity_id": 1, "name": "Player" }}
{{ "type": "add_component", "entity_name": "Player", "component_type": "Transform" }}
{{ "type": "patch_component", "entity_name": "Player", "component_type": "Transform", "data": {{ "x": 640, "y": 490 }} }}
{{ "type": "patch_component", "entity_name": "Player", "component_type": "PhysicsBody", "data": {{ "body_type": "Dynamic", "lock_rotation": true }} }}
{{ "type": "patch_component", "entity_name": "Player", "component_type": "Sprite", "data": {{ "width": 28, "height": 40, "color": [0.3, 0.75, 0.4, 1.0] }} }}
{{ "type": "patch_component", "entity_name": "Player", "component_type": "Collider", "data": {{ "width": 28, "height": 40 }} }}
{{ "type": "remove_component", "entity_id": 1, "component_type": "Script" }}
{{ "type": "edit_transform", "entity_name": "Player", "x": 400, "y": 260, "scale_x": 1.0, "scale_y": 1.0, "rotation": 0.0 }}
{{ "type": "write_script", "path": "scripts/player.lua", "content": "..." }}
{{ "type": "attach_script", "entity_name": "Player", "path": "scripts/player.lua", "content": "..." }}

Rules:
- Use entity_name for entities created in the same change; use entity_id for existing entities.
- Coordinate system: +X right, +Y down.
- Physics needs both PhysicsBody and Collider. body_type: "Dynamic" (players/enemies), "Fixed" (ground/walls), "Kinematic" (scripted platforms). lock_rotation=true for platformer players.
- Supported component types: Transform, Sprite, AnimatedSprite, Tilemap, PhysicsBody, Collider, Script, Camera, AudioSource.
- Scripts use the REAL Sindri Lua API — NOT globals like key_down(). Use: self:input():is_key_down("A"), self:transform():set_position(vec2(x,y)), self:physics():set_velocity(vec2(vx,vy)), self:sprite():set_tint({{r,g,b,a}}). Always nil-check facets before use.
- AnimatedSpriteFacet: `local anim = self:animated_sprite()` — nil if no AnimatedSprite component. `anim:play("clip_name")` switches the active clip (idempotent — safe to call every frame). `anim:set_flip_x(bool)`, `anim:set_flip_y(bool)`, `anim:current_clip()` → string. Walk pattern: `local anim = self:animated_sprite(); if anim ~= nil then if h > 0.1 then anim:play("walk_right") elseif h < -0.1 then anim:play("walk_left") else anim:play("idle") end end`
- Cross-entity queries: `entity_transform("Name")` → `{{x, y, rotation}}` or nil (snapshot from this frame). `self:physics():contacts()` → array of entity name strings currently touching this entity's collider.
- ATTACH/FOLLOW PATTERN — to make entity A ride/follow entity B: in A's script, check `self:physics():contacts()` for B's name, then use `entity_transform("B")` to read B's position/rotation, then `self:transform():set_position(...)` and `self:transform():set_rotation(...)` to snap A onto B with an offset. Example: `local contacts = self:physics():contacts(); for _, name in ipairs(contacts) do if name == "Drone" then local dt_b = entity_transform("Drone"); if dt_b then self:transform():set_rotation(dt_b.rotation); self:transform():set_position(vec2(dt_b.x, dt_b.y - 32)); end end end`
- PLAYER TEMPLATE — "actions" must include: create_entity, add+patch Transform, add+patch Sprite (color/size), add+patch PhysicsBody (Dynamic, lock_rotation true), add+patch Collider (same size as sprite), attach_script with full Lua movement code.
- Return ONLY the JSON object."#,
        engine_ref = engine_ref,
    );

    let mut context_text = String::new();
    if let Some(s) = &scene {
        context_text.push_str(&format!("Scene:\n```json\n{}\n```\n\n", s));
    }
    if let Some(script) = &open_script {
        if context_flags.include_script {
            context_text.push_str(&format!(
                "Open script ({path}):\n```lua\n{content}\n```\n\n",
                path = script.path,
                content = script.content
            ));
        }
    }
    if let Some(errors) = &errors {
        if !errors.is_empty() {
            context_text.push_str(&format!("Errors:\n```\n{}\n```\n\n", errors));
        }
    }
    context_text.push_str(&format!("User message: {}", message));

    let user_content: serde_json::Value = if use_vision {
        serde_json::json!([
            { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{}", screenshot.unwrap_or_default()) } },
            { "type": "text", "text": context_text }
        ])
    } else {
        serde_json::json!(context_text)
    };

    let mut messages: Vec<serde_json::Value> =
        vec![serde_json::json!({ "role": "system", "content": system_prompt })];
    let recent_history = if history.len() > 10 { &history[history.len() - 10..] } else { &history[..] };
    messages.extend_from_slice(recent_history);
    messages.push(serde_json::json!({ "role": "user", "content": user_content }));

    let raw_text = request_ai_text(&client, &provider, &model, messages).await?;
    if raw_text.trim().is_empty() {
        return Err(format!("{provider} returned an empty proposal response for model `{model}`"));
    }

    // Strip <think>...</think> reasoning blocks (qwen3 and similar models)
    let raw_after_think = if let Some(end) = raw_text.find("</think>") {
        raw_text[end + 8..].trim().to_string()
    } else {
        raw_text.trim().to_string()
    };

    // Strip markdown fences if the model wraps the JSON
    let stripped = raw_after_think
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Find JSON object bounds, skipping any remaining leading prose
    let json_start = stripped.find('{').unwrap_or(0);
    let json_end = stripped.rfind('}').map(|i| i + 1).unwrap_or(stripped.len());
    let json_slice = if json_start < json_end { &stripped[json_start..json_end] } else { stripped };

    let parsed: serde_json::Value = serde_json::from_str(json_slice).map_err(|err| {
        let preview: String = raw_text.chars().take(500).collect();
        format!("AI response was not valid proposal JSON: {err}. Response preview: {preview}")
    })?;

    let mut summary = parsed["summary"].as_str().unwrap_or("").trim().to_string();
    let changes_raw = parsed["changes"].as_array().cloned().unwrap_or_default();

    // Execute all actions immediately as staged, tracking what each change created.
    let mut changes: Vec<ProposalChange> = Vec::new();
    for (i, c) in changes_raw.iter().enumerate() {
        let id = c["id"].as_str().unwrap_or(&format!("c{}", i + 1)).trim().to_string();
        let label = c["label"].as_str().unwrap_or("Change").trim().to_string();
        let detail = c["detail"].as_str().unwrap_or("").trim().to_string();

        // Accept both "actions" (array) and legacy "action" (single).
        let actions: Vec<serde_json::Value> = if let Some(arr) = c["actions"].as_array() {
            arr.clone()
        } else if !c["action"].is_null() {
            vec![c["action"].clone()]
        } else {
            vec![]
        };
        if actions.is_empty() {
            continue;
        }

        let mut staged_entity_ids: Vec<u64> = Vec::new();
        let mut modified_entity_ids: Vec<u64> = Vec::new();
        let mut new_script_paths: Vec<String> = Vec::new();
        let mut script_backups: Vec<ScriptBackup> = Vec::new();
        let mut applied_any = false;

        for action in &actions {
            let action_type = action["type"].as_str().unwrap_or("");
            if action_type.is_empty() {
                continue;
            }

            if action_type == "create_entity" {
                // Create as staged so it's visible but marked pending.
                let name = action["name"].as_str().unwrap_or("Entity");
                if find_entity_id_by_name(&client, name).await?.is_none() {
                    let resp: serde_json::Value = client
                        .post(engine_url("/scene/entity"))
                        .json(&serde_json::json!({
                            "name": name,
                            "parent_id": action["parent_id"],
                            "staged": true,
                        }))
                        .send()
                        .await
                        .map_err(|e| e.to_string())?
                        .json()
                        .await
                        .map_err(|e| e.to_string())?;
                    if let Some(id_val) = resp["id"].as_u64() {
                        staged_entity_ids.push(id_val);
                        applied_any = true;
                    }
                }
            } else {
                let supported_action = matches!(
                    action_type,
                    "edit_transform" | "delete_entity" | "write_script" | "rename_entity"
                        | "add_component" | "remove_component" | "patch_component" | "attach_script"
                );
                if !supported_action {
                    eprintln!("[generate_proposal] skipped unsupported action type: {action_type}");
                    continue;
                }

                if action_type == "write_script" || action_type == "attach_script" {
                    if let Some(path) = action["path"].as_str() {
                        if !new_script_paths.iter().any(|existing| existing == path) {
                            new_script_paths.push(path.to_string());
                        }
                        if !script_backups.iter().any(|backup| backup.path == path) {
                            script_backups.push(read_script_backup(&client, path).await);
                        }
                    }
                }
                apply_action(action.clone())
                    .await
                    .map_err(|err| format!("failed to apply {action_type}: {err}"))?;
                applied_any = true;

                // Mark modified existing entities as staged for visual indication.
                let modifies_entity = matches!(
                    action_type,
                    "add_component" | "remove_component" | "patch_component"
                        | "edit_transform" | "rename_entity" | "attach_script"
                );
                if modifies_entity {
                    if let Ok(entity_id) = resolve_entity_id(action, &client).await {
                        if !staged_entity_ids.contains(&entity_id)
                            && !modified_entity_ids.contains(&entity_id)
                        {
                            modified_entity_ids.push(entity_id);
                            let _ = client
                                .patch(engine_url(&format!("/scene/entity/{}/staged", entity_id)))
                                .json(&serde_json::json!({ "staged": true }))
                                .send()
                                .await;
                        }
                    }
                }
            }
        }

        if !applied_any
            && staged_entity_ids.is_empty()
            && modified_entity_ids.is_empty()
            && new_script_paths.is_empty()
        {
            continue;
        }

        changes.push(ProposalChange {
            id,
            label,
            detail,
            staged_entity_ids,
            modified_entity_ids,
            new_script_paths,
            script_backups,
        });
    }

    if changes.is_empty() && looks_like_player_jump_request(&message) {
        if let Some(fallback) = stage_player_jump_fallback(&client, scene.as_deref(), open_script.as_ref()).await? {
            summary = "Staged jump controls for the player script.".to_string();
            changes.push(fallback);
        }
    }

    if changes.is_empty() && looks_like_edit_request(&message) {
        return Err(
            "The model did not return any staged actions for that edit request. Nothing was changed. Try again with a more specific target, or switch the Code model in AI settings."
                .to_string(),
        );
    }

    if summary.is_empty() {
        summary = if let Some(change) = changes.first() {
            if !change.label.trim().is_empty() && change.label != "Change" {
                change.label.clone()
            } else if !change.detail.trim().is_empty() {
                change.detail.clone()
            } else {
                format!(
                    "Staged {} proposed change{}.",
                    changes.len(),
                    if changes.len() == 1 { "" } else { "s" }
                )
            }
        } else {
            "No changes proposed.".to_string()
        };
    }

    // Persist proposal metadata alongside the scene for restoration on restart.
    let proposal_json = serde_json::to_string_pretty(&serde_json::json!({
        "prompt": message,
        "summary": summary,
        "changes": changes.iter().map(|c| serde_json::json!({
            "id": c.id,
            "label": c.label,
            "detail": c.detail,
            "staged_entity_ids": c.staged_entity_ids,
            "modified_entity_ids": c.modified_entity_ids,
            "new_script_paths": c.new_script_paths,
            "script_backups": c.script_backups.iter().map(|b| serde_json::json!({
                "path": b.path,
                "existed": b.existed,
                "content": b.content,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    }))
    .unwrap_or_default();
    if let Ok(resp) = reqwest::get(engine_url("/scene/path")).await {
        if let Ok(scene_path) = resp.text().await {
            let proposal_path = format!("{}.proposal.json", scene_path.trim_matches('"'));
            let _ = std::fs::write(&proposal_path, &proposal_json);
        }
    }

    Ok(ProposalResponse { prompt: message, summary, changes })
}

fn normalize_script_actions(
    message: &str,
    open_script: Option<&OpenScriptContext>,
    actions: &mut [serde_json::Value],
) {
    let referenced_paths = referenced_script_paths(message);
    if referenced_paths.is_empty() {
        return;
    }

    for action in actions.iter_mut() {
        if action["type"].as_str() != Some("attach_script") {
            continue;
        }

        let Some(path) = action["path"].as_str() else {
            continue;
        };

        if !referenced_paths.iter().any(|referenced| referenced == path) {
            continue;
        }

        let content = action["content"].clone();
        *action = serde_json::json!({
            "type": "write_script",
            "path": path,
            "content": content,
        });
    }

    if let Some(script) = open_script {
        for action in actions.iter_mut() {
            if action["type"].as_str() != Some("write_script") {
                continue;
            }
            if action["path"].is_null() {
                action["path"] = serde_json::json!(script.path);
            }
        }
    }
}

async fn read_script_backup(client: &reqwest::Client, path: &str) -> ScriptBackup {
    match client.get(engine_url(&format!("/script?path={}", path))).send().await {
        Ok(resp) if resp.status().is_success() => ScriptBackup {
            path: path.to_string(),
            existed: true,
            content: resp.text().await.unwrap_or_default(),
        },
        _ => ScriptBackup {
            path: path.to_string(),
            existed: false,
            content: String::new(),
        },
    }
}

fn looks_like_edit_request(message: &str) -> bool {
    // Any message with an @entity mention is always an edit request.
    if message.contains('@') {
        return true;
    }
    let lower = message.to_lowercase();
    [
        "add ", "make ", "create ", "update ", "change ", "fix ", "remove ", "delete ",
        "attach ", "write ", "set ", "move ", "rename ", "jump", "script",
        "connect", "rotate with", "follow", "can you",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn looks_like_player_jump_request(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("player") && lower.contains("jump")
}

async fn stage_player_jump_fallback(
    client: &reqwest::Client,
    scene_json: Option<&str>,
    open_script: Option<&OpenScriptContext>,
) -> Result<Option<ProposalChange>, String> {
    let scene = match scene_json.and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok()) {
        Some(scene) => scene,
        None => fetch_scene(client).await?,
    };

    let Some(player) = scene["entities"]
        .as_object()
        .and_then(|entities| entities.values().find(|entity| {
            entity["name"].as_str()
                .map(|name| name.eq_ignore_ascii_case("player"))
                .unwrap_or(false)
        }))
    else {
        return Ok(None);
    };

    let Some(player_id) = player["id"].as_u64() else {
        return Ok(None);
    };

    let script_path = player["components"]
        .as_array()
        .and_then(|components| {
            components.iter().find_map(|component| {
                if component["type"].as_str() == Some("Script") {
                    component["path"].as_str().filter(|path| !path.is_empty())
                } else {
                    None
                }
            })
        })
        .map(str::to_string)
        .or_else(|| open_script.map(|script| script.path.clone()))
        .unwrap_or_else(|| "scripts/player.lua".to_string());

    let backup = read_script_backup(client, &script_path).await;
    let content = player_jump_script();
    client
        .put(engine_url(&format!("/script?path={}", script_path)))
        .json(&serde_json::json!({ "content": content }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    ensure_component(client, player_id, "Script").await?;
    let components = entity_components(client, player_id).await?;
    for (idx, component) in components.iter().enumerate() {
        if component["type"].as_str() == Some("Script") {
            client
                .patch(engine_url(&format!("/scene/entity/{}/component/{}", player_id, idx)))
                .json(&serde_json::json!({ "path": script_path }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            break;
        }
    }

    let _ = client
        .patch(engine_url(&format!("/scene/entity/{}/staged", player_id)))
        .json(&serde_json::json!({ "staged": true }))
        .send()
        .await;

    Ok(Some(ProposalChange {
        id: "jump-script".to_string(),
        label: "Add player jump controls".to_string(),
        detail: "Updates the player Lua script with horizontal movement and Space-to-jump using the Sindri input and physics facets.".to_string(),
        staged_entity_ids: Vec::new(),
        modified_entity_ids: vec![player_id],
        new_script_paths: vec![script_path],
        script_backups: vec![backup],
    }))
}

fn player_jump_script() -> &'static str {
    r#"local speed = 220.0
local jump_speed = 420.0
local grounded_y = nil
local was_grounded = false

function on_update(self, dt)
  local input = self:input()
  local transform = self:transform()
  local physics = self:physics()
  if input == nil or transform == nil or physics == nil then
    return
  end

  local pos = transform:position()
  if grounded_y == nil then
    grounded_y = pos.y
  end

  local velocity = physics:velocity()
  local horizontal = input:axis("A", "D")
  local grounded = pos.y >= grounded_y - 2.0 and velocity.y >= -1.0

  physics:set_velocity(vec2(horizontal * speed, velocity.y))

  if grounded and input:is_key_pressed("Space") then
    physics:set_velocity(vec2(horizontal * speed, -jump_speed))
    was_grounded = false
  else
    was_grounded = grounded
  end
end
"#
}

fn referenced_script_paths(message: &str) -> Vec<String> {
    message
        .split_whitespace()
        .filter_map(|token| token.strip_prefix('#'))
        .map(|path| path.trim_matches(|ch: char| ",.!?;:()[]{}<>\"'`".contains(ch)))
        .filter(|path| path.ends_with(".lua"))
        .map(str::to_string)
        .collect()
}

fn normalize_spatial_actions(
    message: &str,
    scene_json: Option<&str>,
    actions: &mut Vec<serde_json::Value>,
) {
    let lower = message.to_lowercase();
    let wants_ground_below_player = lower.contains("ground")
        && lower.contains("player")
        && (lower.contains("below") || lower.contains("under") || lower.contains("beneath"));
    if !wants_ground_below_player {
        return;
    }

    let scene_player_transform = scene_json
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|scene| entity_transform_by_name(&scene, "Player"));
    let scene_player_x = scene_player_transform.map(|(x, _)| x);
    let scene_player_y = scene_player_transform.map(|(_, y)| y);

    let player_x = find_transform_value(actions, "Player", "x")
        .or(scene_player_x)
        .unwrap_or(400.0);
    let player_y = find_transform_value(actions, "Player", "y")
        .or(scene_player_y)
        .unwrap_or(260.0);
    let ground_y = find_transform_value(actions, "Ground", "y")
        .filter(|y| *y > player_y)
        .unwrap_or(player_y + 80.0);

    ensure_edit_transform(actions, "Player", player_x, player_y);
    ensure_edit_transform(actions, "Ground", player_x, ground_y);
}

fn entity_transform_by_name(scene: &serde_json::Value, name: &str) -> Option<(f32, f32)> {
    scene["entities"]
        .as_object()?
        .values()
        .find(|entity| entity["name"].as_str() == Some(name))
        .and_then(|entity| {
            entity["components"]
                .as_array()?
                .iter()
                .find_map(|component| {
                    if component["type"].as_str() == Some("Transform") {
                        Some((
                            component["x"].as_f64()? as f32,
                            component["y"].as_f64()? as f32,
                        ))
                    } else {
                        None
                    }
                })
        })
}

fn find_transform_value(
    actions: &[serde_json::Value],
    entity_name: &str,
    field: &str,
) -> Option<f32> {
    actions.iter().find_map(|action| {
        if action["type"].as_str() == Some("edit_transform")
            && action["entity_name"].as_str() == Some(entity_name)
        {
            action[field].as_f64().map(|value| value as f32)
        } else {
            None
        }
    })
}

fn ensure_edit_transform(actions: &mut Vec<serde_json::Value>, entity_name: &str, x: f32, y: f32) {
    if let Some(action) = actions.iter_mut().find(|action| {
        action["type"].as_str() == Some("edit_transform")
            && action["entity_name"].as_str() == Some(entity_name)
    }) {
        action["x"] = serde_json::json!(x);
        action["y"] = serde_json::json!(y);
        return;
    }

    actions.push(serde_json::json!({
        "type": "edit_transform",
        "entity_name": entity_name,
        "x": x,
        "y": y
    }));
}

#[tauri::command]
pub async fn create_entity(
    name: String,
    parent_id: Option<u64>,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "name": name, "parent_id": parent_id });
    let resp: serde_json::Value = client
        .post(engine_url("/scene/entity"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp)
}

/// Resolve an entity ID from an action: use `entity_id` if present,
/// otherwise look up by `entity_name` in the live scene.
async fn resolve_entity_id(
    action: &serde_json::Value,
    client: &reqwest::Client,
) -> Result<u64, String> {
    if let Some(id) = action["entity_id"].as_u64() {
        return Ok(id);
    }
    if let Some(name) = action["entity_name"].as_str() {
        if let Some(id) = find_entity_id_by_name(client, name).await? {
            return Ok(id);
        }
        return Err(format!("No entity found with name '{}'", name));
    }
    Err("missing entity_id or entity_name".to_string())
}

async fn fetch_scene(client: &reqwest::Client) -> Result<serde_json::Value, String> {
    client
        .get(engine_url("/scene"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn find_entity_id_by_name(
    client: &reqwest::Client,
    name: &str,
) -> Result<Option<u64>, String> {
    let scene = fetch_scene(client).await?;
    Ok(scene["entities"].as_object().and_then(|entities| {
        entities
            .iter()
            .filter_map(|(id_str, entity)| {
                if entity["name"].as_str() == Some(name) {
                    id_str.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .max()
    }))
}

async fn entity_components(
    client: &reqwest::Client,
    entity_id: u64,
) -> Result<Vec<serde_json::Value>, String> {
    let scene = fetch_scene(client).await?;
    let entity_id_str = entity_id.to_string();
    Ok(scene["entities"][&entity_id_str]["components"]
        .as_array()
        .cloned()
        .unwrap_or_default())
}

fn component_type(component: &serde_json::Value) -> Option<&str> {
    component["type"].as_str()
}

async fn has_component(
    client: &reqwest::Client,
    entity_id: u64,
    wanted_type: &str,
) -> Result<bool, String> {
    Ok(entity_components(client, entity_id)
        .await?
        .iter()
        .any(|component| component_type(component) == Some(wanted_type)))
}

async fn ensure_component(
    client: &reqwest::Client,
    entity_id: u64,
    component_type: &str,
) -> Result<(), String> {
    if has_component(client, entity_id, component_type).await? {
        return Ok(());
    }

    client
        .post(engine_url(&format!(
            "/scene/entity/{}/component",
            entity_id
        )))
        .json(&serde_json::json!({ "component_type": component_type }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn apply_action(action: serde_json::Value) -> Result<(), String> {
    let client = reqwest::Client::new();
    let action_type = action["type"].as_str().unwrap_or("");

    match action_type {
        "edit_transform" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let body = serde_json::json!({
                "x": action["x"],
                "y": action["y"],
                "scale_x": action["scale_x"],
                "scale_y": action["scale_y"],
                "rotation": action["rotation"],
            });
            client
                .patch(engine_url(&format!(
                    "/scene/entity/{}/transform",
                    entity_id
                )))
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "create_entity" => {
            let name = action["name"].as_str().ok_or("missing name")?;
            if find_entity_id_by_name(&client, name).await?.is_some() {
                return Ok(());
            }

            let body = serde_json::json!({
                "name": action["name"],
                "parent_id": action["parent_id"],
            });
            client
                .post(engine_url("/scene/entity"))
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "delete_entity" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            client
                .delete(engine_url(&format!("/scene/entity/{}", entity_id)))
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "write_script" => {
            let path = action["path"].as_str().ok_or("missing path")?;
            let body = serde_json::json!({ "content": action["content"] });
            client
                .put(engine_url(&format!("/script?path={}", path)))
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "rename_entity" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let name = action["name"].as_str().ok_or("missing name")?;
            client
                .patch(engine_url(&format!("/scene/entity/{}/name", entity_id)))
                .json(&serde_json::json!({ "name": name }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "add_component" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let component_type = action["component_type"]
                .as_str()
                .ok_or("missing component_type")?;
            ensure_component(&client, entity_id, component_type).await?;
        }
        "remove_component" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            // Prefer index if provided, else fall back to type-based lookup via old route
            if let Some(idx) = action["component_idx"].as_u64() {
                client
                    .delete(engine_url(&format!(
                        "/scene/entity/{}/component/{}",
                        entity_id, idx
                    )))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                let component_type = action["component_type"]
                    .as_str()
                    .ok_or("missing component_type or component_idx")?;
                client
                    .delete(engine_url(&format!(
                        "/scene/entity/{}/component?component_type={}",
                        entity_id, component_type
                    )))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        "patch_component" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let component_idx = if let Some(idx) = action["component_idx"].as_u64() {
                idx as usize
            } else {
                let component_type = action["component_type"]
                    .as_str()
                    .ok_or("missing component_idx or component_type")?;
                entity_components(&client, entity_id)
                    .await?
                    .iter()
                    .position(|component| component["type"].as_str() == Some(component_type))
                    .ok_or_else(|| format!("entity has no {component_type} component"))?
            };
            client
                .patch(engine_url(&format!(
                    "/scene/entity/{}/component/{}",
                    entity_id, component_idx
                )))
                .json(&action["data"])
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        // Composite: write script file + add Script component + set path in one shot
        "attach_script" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let path = action["path"].as_str().ok_or("missing path")?;
            let content = action["content"].as_str().unwrap_or("");

            // Write the file
            client
                .put(engine_url(&format!("/script?path={}", path)))
                .json(&serde_json::json!({ "content": content }))
                .send()
                .await
                .map_err(|e| e.to_string())?;

            ensure_component(&client, entity_id, "Script").await?;

            // Find an existing Script component and set the path.
            let components = entity_components(&client, entity_id).await?;
            {
                for (idx, comp) in components.iter().enumerate() {
                    let script_path = comp["path"].as_str().unwrap_or("");
                    if comp["type"] == "Script" && (script_path.is_empty() || script_path == path) {
                        client
                            .patch(engine_url(&format!(
                                "/scene/entity/{}/component/{}",
                                entity_id, idx
                            )))
                            .json(&serde_json::json!({ "path": path }))
                            .send()
                            .await
                            .map_err(|e| e.to_string())?;
                        break;
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

#[tauri::command]
pub async fn commit_staged_change(
    entity_ids: Vec<u64>,
    modified_entity_ids: Vec<u64>,
    script_paths: Vec<String>,
    script_backups: Vec<ScriptBackup>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let all_ids: Vec<u64> = entity_ids.iter().chain(modified_entity_ids.iter()).copied().collect();
    if !all_ids.is_empty() {
        client
            .post(engine_url("/scene/staged/commit"))
            .json(&serde_json::json!({ "entity_ids": all_ids }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }
    let _ = script_paths; // scripts are already written; nothing extra to do on commit
    let _ = script_backups;
    Ok(())
}

#[tauri::command]
pub async fn revert_staged_change(
    entity_ids: Vec<u64>,
    modified_entity_ids: Vec<u64>,
    script_paths: Vec<String>,
    script_backups: Vec<ScriptBackup>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    // Delete newly created staged entities.
    if !entity_ids.is_empty() {
        client
            .post(engine_url("/scene/staged/revert"))
            .json(&serde_json::json!({ "entity_ids": entity_ids }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }
    // Modified existing entities can't be easily undone — just clear their staged flag.
    if !modified_entity_ids.is_empty() {
        client
            .post(engine_url("/scene/staged/commit"))
            .json(&serde_json::json!({ "entity_ids": modified_entity_ids }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }
    // Restore script files to their previous contents, or delete files that did not exist.
    if !script_backups.is_empty() {
        let scene_path = async {
            let r = reqwest::get(engine_url("/scene/path")).await?;
            r.text().await
        }.await.unwrap_or_default();
        let scene_path = scene_path.trim_matches('"').to_string();
        for backup in &script_backups {
            if let Some(project_root) = std::path::Path::new(&scene_path).parent() {
                let full = project_root.join(&backup.path);
                if backup.existed {
                    if let Some(parent) = full.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(full, &backup.content);
                } else {
                    let _ = std::fs::remove_file(full);
                }
            }
        }
    } else if !script_paths.is_empty() {
        let scene_path = async {
            let r = reqwest::get(engine_url("/scene/path")).await?;
            r.text().await
        }.await.unwrap_or_default();
        let scene_path = scene_path.trim_matches('"').to_string();
        for path in &script_paths {
            if let Some(project_root) = std::path::Path::new(&scene_path).parent() {
                let _ = std::fs::remove_file(project_root.join(path));
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn clear_staged_proposal() -> Result<(), String> {
    if let Ok(resp) = reqwest::get(engine_url("/scene/path")).await {
        if let Ok(scene_path) = resp.text().await {
            let proposal_path = format!("{}.proposal.json", scene_path.trim_matches('"'));
            let _ = std::fs::remove_file(&proposal_path);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_pending_proposal() -> Result<Option<ProposalResponse>, String> {
    let scene_path = match reqwest::get(engine_url("/scene/path")).await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return Ok(None),
    };
    let proposal_path = format!("{}.proposal.json", scene_path.trim_matches('"'));
    let content = match std::fs::read_to_string(&proposal_path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    // Also check that there are actually staged entities in the scene.
    let scene_json = match reqwest::get(engine_url("/scene")).await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return Ok(None),
    };
    let scene: serde_json::Value = serde_json::from_str(&scene_json).unwrap_or_default();
    let has_staged = scene["entities"].as_object()
        .map(|ents| ents.values().any(|e| e["staged"].as_bool().unwrap_or(false)))
        .unwrap_or(false);
    let has_script_changes = parsed["changes"].as_array()
        .map(|changes| {
            changes.iter().any(|change| {
                change["new_script_paths"].as_array()
                    .map(|paths| !paths.is_empty())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    if !has_staged && !has_script_changes {
        // No staged entities; stale proposal file — clean it up.
        let _ = std::fs::remove_file(&proposal_path);
        return Ok(None);
    }
    let prompt = parsed["prompt"].as_str().unwrap_or("").to_string();
    let summary = parsed["summary"].as_str().unwrap_or("").to_string();
    let changes = parsed["changes"].as_array().cloned().unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(i, c)| ProposalChange {
            id: c["id"].as_str().unwrap_or(&format!("c{}", i + 1)).to_string(),
            label: c["label"].as_str().unwrap_or("Change").to_string(),
            detail: c["detail"].as_str().unwrap_or("").to_string(),
            staged_entity_ids: c["staged_entity_ids"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
                .unwrap_or_default(),
            modified_entity_ids: c["modified_entity_ids"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
                .unwrap_or_default(),
            new_script_paths: c["new_script_paths"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            script_backups: c["script_backups"].as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| serde_json::from_value::<ScriptBackup>(v.clone()).ok())
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect();
    Ok(Some(ProposalResponse { prompt, summary, changes }))
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

const SUGGESTION_MODEL: &str = "qwen2.5:0.5b";

fn is_small_model(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["0.5b", "135m", "360m", ":1b", "-1b", ":1.5b", "-1.5b", ":2b", "-2b"]
        .iter()
        .any(|pat| lower.contains(pat))
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

    // No small model found — pull qwen2.5:0.5b
    let resp = client
        .post("http://localhost:11434/api/pull")
        .json(&serde_json::json!({ "name": SUGGESTION_MODEL, "stream": false }))
        .timeout(std::time::Duration::from_secs(900))
        .send()
        .await
        .map_err(|e| format!("pull request failed: {}", e))?;

    // Ollama may return NDJSON even with stream:false — grab the last line
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

    // Find JSON array bounds to handle any leading prose from the model
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

#[tauri::command]
pub async fn rename_entity(entity_id: u64, name: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    client
        .patch(engine_url(&format!("/scene/entity/{}/name", entity_id)))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn add_component(entity_id: u64, component_type: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(engine_url(&format!(
            "/scene/entity/{}/component",
            entity_id
        )))
        .json(&serde_json::json!({ "component_type": component_type }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(resp.text().await.unwrap_or_default())
    }
}

#[tauri::command]
pub async fn remove_component(entity_id: u64, component_idx: usize) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .delete(engine_url(&format!(
            "/scene/entity/{}/component/{}",
            entity_id, component_idx
        )))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(resp.text().await.unwrap_or_default())
    }
}

#[tauri::command]
pub async fn patch_component(
    entity_id: u64,
    component_idx: usize,
    data: serde_json::Value,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .patch(engine_url(&format!(
            "/scene/entity/{}/component/{}",
            entity_id, component_idx
        )))
        .json(&data)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(resp.text().await.unwrap_or_default())
    }
}

#[tauri::command]
pub async fn get_script(path: String) -> Result<String, String> {
    reqwest::get(engine_url(&format!("/script?path={}", path)))
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_script(path: String, content: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "content": content });
    client
        .put(engine_url(&format!("/script?path={}", path)))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_engine_paused(paused: bool) -> Result<(), String> {
    let client = reqwest::Client::new();
    client
        .post(engine_url("/control"))
        .json(&serde_json::json!({ "paused": paused }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_engine_playback(action: String) -> Result<(), String> {
    match action.as_str() {
        "play" | "pause" | "stop" => {}
        _ => return Err("playback action must be play, pause, or stop".into()),
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(engine_url("/control"))
        .json(&serde_json::json!({ "action": action }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|e| e.to_string()));
    }
    Ok(())
}

#[tauri::command]
pub async fn list_scripts() -> Result<Vec<String>, String> {
    let resp: serde_json::Value = reqwest::get(engine_url("/scripts"))
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let paths = resp
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(paths)
}

#[derive(Serialize, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub kind: String, // "dir" | "script" | "scene" | "image" | "audio" | "other"
    pub children: Vec<FileNode>,
}

fn file_kind_from_ext(ext: &str) -> &'static str {
    match ext {
        "lua" => "script",
        "sindri" => "scene",
        "animclips" => "animclips",
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
    pub path: String, // relative to project root, e.g. "scripts/player.lua"
    pub kind: String, // "script" | "scene" | "image" | "audio" | "other"
    pub name: String, // filename only, e.g. "player.lua"
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

// ─── Project Settings ────────────────────────────────────────────────────────

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

// ─── Editor Preferences ──────────────────────────────────────────────────────

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
