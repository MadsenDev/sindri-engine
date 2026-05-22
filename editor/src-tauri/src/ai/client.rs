use futures_util::StreamExt;
use tauri::Emitter;

use super::{AiProviderStatus, AiStreamEvent};

const KEYCHAIN_SERVICE: &str = "sindri-editor-ai";

pub(crate) fn default_cloud_model(provider: &str) -> &'static str {
    match provider {
        "openai" => "gpt-5-mini",
        "anthropic" => "claude-sonnet-4-20250514",
        "openrouter" => "meta-llama/llama-3.1-8b-instruct:free",
        _ => "qwen2.5-coder:7b",
    }
}

fn keychain_entry(provider: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, provider).map_err(|e| e.to_string())
}

pub(crate) fn provider_api_key(provider: &str) -> Result<String, String> {
    match provider {
        "openai" => std::env::var("OPENAI_API_KEY")
            .ok()
            .or_else(|| keychain_entry("openai").ok()?.get_password().ok())
            .ok_or_else(|| "OpenAI API key is not configured".to_string()),
        "anthropic" => std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .or_else(|| keychain_entry("anthropic").ok()?.get_password().ok())
            .ok_or_else(|| "Anthropic API key is not configured".to_string()),
        "openrouter" => std::env::var("OPENROUTER_API_KEY")
            .ok()
            .or_else(|| keychain_entry("openrouter").ok()?.get_password().ok())
            .ok_or_else(|| "OpenRouter API key is not configured".to_string()),
        _ => Err("Local Ollama does not use an API key".to_string()),
    }
}

pub(crate) fn emit_ai_stream(app: &tauri::AppHandle, request_id: &str, kind: &str, text: impl Into<String>) {
    let _ = app.emit("ai://stream", AiStreamEvent {
        request_id: request_id.to_string(),
        kind: kind.to_string(),
        text: text.into(),
    });
}

pub(crate) async fn request_ai_text(
    client: &reqwest::Client,
    provider: &str,
    model: &str,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let text = match provider {
        "openai" => request_openai_text(client, model, messages).await,
        "anthropic" => request_anthropic_text(client, model, messages).await,
        "openrouter" => request_openrouter_text(client, model, messages).await,
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

/// Extract a readable message from an OpenRouter error response.
/// Priority: error.metadata.raw > error.message > raw text.
async fn openrouter_error(resp: reqwest::Response) -> String {
    let text = resp.text().await.unwrap_or_default();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(raw) = v["error"]["metadata"]["raw"].as_str() {
            return raw.to_string();
        }
        if let Some(msg) = v["error"]["message"].as_str() {
            return msg.to_string();
        }
    }
    text
}

async fn request_openrouter_text(
    client: &reqwest::Client,
    model: &str,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let key = provider_api_key("openrouter")?;
    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(key)
        .header("HTTP-Referer", "https://sindri.gg")
        .header("X-Title", "Sindri Engine")
        .json(&serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(openrouter_error(resp).await);
    }
    let value: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    value["choices"][0]["message"]["content"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("OpenRouter response did not include choices[0].message.content: {value}"))
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
                    let prefixes = [
                        ("data:image/png;base64,", "image/png"),
                        ("data:image/jpeg;base64,", "image/jpeg"),
                        ("data:image/webp;base64,", "image/webp"),
                        ("data:image/gif;base64,", "image/gif"),
                    ];
                    for (prefix, mime) in &prefixes {
                        if let Some(data) = url.strip_prefix(prefix) {
                            blocks.push(serde_json::json!({
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": mime,
                                    "data": data
                                }
                            }));
                            break;
                        }
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

pub(crate) async fn stream_ollama_text(
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

pub(crate) async fn stream_openai_text(
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

pub(crate) async fn stream_openrouter_text(
    app: tauri::AppHandle,
    request_id: String,
    client: reqwest::Client,
    model: String,
    messages: Vec<serde_json::Value>,
) -> Result<String, String> {
    let key = provider_api_key("openrouter")?;
    let resp = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(key)
        .header("HTTP-Referer", "https://sindri.gg")
        .header("X-Title", "Sindri Engine")
        .json(&serde_json::json!({ "model": model, "messages": messages, "stream": true }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(openrouter_error(resp).await);
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
            if data == "[DONE]" { continue; }
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

pub(crate) async fn stream_anthropic_text(
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
        AiProviderStatus {
            provider: "openrouter".into(),
            configured: provider_api_key("openrouter").is_ok(),
            default_model: default_cloud_model("openrouter").into(),
        },
    ])
}

#[tauri::command]
pub async fn save_ai_api_key(provider: String, api_key: String) -> Result<(), String> {
    if !["openai", "anthropic", "openrouter"].contains(&provider.as_str()) {
        return Err("Only cloud provider keys can be saved".into());
    }
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err("API key cannot be empty".into());
    }
    keychain_entry(&provider)?.set_password(api_key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_ai_api_key(provider: String) -> Result<(), String> {
    if !["openai", "anthropic", "openrouter"].contains(&provider.as_str()) {
        return Err("Only cloud provider keys can be cleared".into());
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
