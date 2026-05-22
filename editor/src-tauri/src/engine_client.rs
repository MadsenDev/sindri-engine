use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) const ENGINE_BASE: &str = "http://127.0.0.1:7878";

pub(crate) fn engine_url(path: &str) -> String {
    format!("{}{}", ENGINE_BASE, path)
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
pub async fn get_scene() -> Result<String, String> {
    reqwest::get(engine_url("/scene"))
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
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
    z_index: Option<i32>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "x": x, "y": y, "scale_x": scale_x, "scale_y": scale_y, "rotation": rotation, "z_index": z_index });
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
