use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

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

    let old_child = state.0.lock().unwrap().take();
    if let Some(mut child) = old_child {
        let _ = child.kill().await;
    }

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
