use crate::engine_client::engine_url;

pub struct SpritesheetAnalysis {
    pub tex_path: String,
    pub cols: u32,
    pub rows: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    /// Number of non-empty frames detected in each row.
    pub frames_per_row: Vec<u32>,
}

/// Locate and pixel-analyse a spritesheet relevant to the user's message.
pub async fn analyze_spritesheet_for_message(
    message: &str,
    scene: Option<&str>,
) -> Option<SpritesheetAnalysis> {
    let scene_path = reqwest::get(engine_url("/scene/path")).await.ok()?.text().await.ok()?;
    let scene_path = scene_path.trim_matches('"').to_string();
    let project_root = std::path::Path::new(&scene_path).parent()?.to_path_buf();

    let mut candidates: Vec<String> = Vec::new();
    if let Some(scene_json) = scene {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(scene_json) {
            if let Some(entities) = v["entities"].as_object() {
                for ent in entities.values() {
                    for comp in ent["components"].as_array().into_iter().flatten() {
                        if comp["type"].as_str() == Some("AnimatedSprite") {
                            if let Some(p) = comp["texture_path"].as_str() {
                                if !p.is_empty() && !candidates.contains(&p.to_string()) {
                                    candidates.push(p.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let keywords: Vec<String> = message
        .split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| w.len() >= 3)
        .collect();

    let textures_dir = project_root.join("textures");
    if let Ok(entries) = std::fs::read_dir(&textures_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let is_image = name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".jpeg");
            if is_image && keywords.iter().any(|kw| name.contains(kw.as_str())) {
                let rel = format!("textures/{}", entry.file_name().to_string_lossy());
                if !candidates.contains(&rel) {
                    candidates.insert(0, rel);
                }
            }
        }
    }

    let best = if candidates.len() == 1 {
        candidates.into_iter().next()?
    } else {
        candidates.into_iter().find(|p| {
            let lower = p.to_lowercase();
            keywords.iter().any(|kw| lower.contains(kw.as_str()))
        })?
    };

    let file_path = project_root.join(&best);
    analyse_spritesheet_pixels(&file_path, best)
}

fn analyse_spritesheet_pixels(path: &std::path::Path, tex_path: String) -> Option<SpritesheetAnalysis> {
    let img = image::open(path).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 { return None; }

    let mut edge_counts: std::collections::HashMap<[u8; 4], u32> = std::collections::HashMap::new();
    let sample = |x: u32, y: u32| -> [u8; 4] { img.get_pixel(x, y).0 };
    for x in 0..w { *edge_counts.entry(sample(x, 0)).or_default() += 1; }
    for x in 0..w { *edge_counts.entry(sample(x, h - 1)).or_default() += 1; }
    for y in 0..h { *edge_counts.entry(sample(0, y)).or_default() += 1; }
    for y in 0..h { *edge_counts.entry(sample(w - 1, y)).or_default() += 1; }
    let bg = edge_counts.into_iter().max_by_key(|(_, c)| *c).map(|(c, _)| c).unwrap_or([0, 0, 0, 0]);
    let has_alpha = img.pixels().any(|p| p.0[3] < 255);

    let is_bg = |p: [u8; 4]| -> bool {
        if has_alpha {
            p[3] < 16
        } else {
            let dr = (p[0] as i32 - bg[0] as i32).abs();
            let dg = (p[1] as i32 - bg[1] as i32).abs();
            let db = (p[2] as i32 - bg[2] as i32).abs();
            dr + dg + db < 30
        }
    };

    let empty_col: Vec<bool> = (0..w).map(|x| (0..h).all(|y| is_bg(img.get_pixel(x, y).0))).collect();
    let empty_row: Vec<bool> = (0..h).map(|y| (0..w).all(|x| is_bg(img.get_pixel(x, y).0))).collect();

    let col_runs = span_lengths(&empty_col);
    let row_runs = span_lengths(&empty_row);

    if col_runs.is_empty() || row_runs.is_empty() { return None; }

    let frame_width = mode_u32(&col_runs)?;
    let frame_height = mode_u32(&row_runs)?;

    let cols = (w / frame_width).max(1);
    let rows = (h / frame_height).max(1);

    let mut frames_per_row: Vec<u32> = Vec::new();
    for row in 0..rows {
        let y0 = row * frame_height;
        let mut count = 0u32;
        for col in 0..cols {
            let x0 = col * frame_width;
            let has_content = (y0..y0 + frame_height).any(|y|
                (x0..x0 + frame_width).any(|x| {
                    if x < w && y < h { !is_bg(img.get_pixel(x, y).0) } else { false }
                })
            );
            if has_content { count += 1; }
        }
        if count > 0 {
            frames_per_row.push(count);
        }
    }

    let actual_rows = frames_per_row.len() as u32;
    if actual_rows == 0 { return None; }

    Some(SpritesheetAnalysis { tex_path, cols, rows: actual_rows, frame_width, frame_height, frames_per_row })
}

/// Returns lengths of non-empty runs in a boolean slice where true = empty.
fn span_lengths(empty: &[bool]) -> Vec<u32> {
    let mut runs = Vec::new();
    let mut len = 0u32;
    for &e in empty {
        if !e { len += 1; }
        else if len > 0 { runs.push(len); len = 0; }
    }
    if len > 0 { runs.push(len); }
    runs
}

/// Mode of a u32 slice (most common value).
fn mode_u32(vals: &[u32]) -> Option<u32> {
    let mut counts: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    for &v in vals { *counts.entry(v).or_default() += 1; }
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(v, _)| v)
}
