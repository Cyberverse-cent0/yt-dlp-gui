//! 视频信息获取与 Cookie 管理

use crate::utils;
use serde_json::Value;
use tauri::AppHandle;

use super::support;

// Cookie 管理

/// 保存 Cookie 文本（Netscape 格式）到应用数据目录
#[tauri::command]
pub async fn save_cookie_text(app: AppHandle, text: String) -> Result<String, String> {
    let cookie_path = utils::get_cookie_path(&app)?;
    let normalized = normalize_netscape_cookie_file(&text)?;
    let temporary = cookie_path.with_extension("txt.tmp");
    tokio::fs::write(&temporary, normalized.as_bytes())
        .await
        .map_err(|e| format!("err_save_cookie:{}", e))?;
    if tokio::fs::rename(&temporary, &cookie_path).await.is_err() {
        let _ = tokio::fs::remove_file(&cookie_path).await;
        tokio::fs::rename(&temporary, &cookie_path)
            .await
            .map_err(|e| format!("err_save_cookie:{}", e))?;
    }
    Ok(cookie_path.to_string_lossy().to_string())
}

fn normalize_netscape_cookie_file(text: &str) -> Result<String, String> {
    let mut lines = Vec::new();
    let mut has_header = false;
    let mut cookie_count = 0usize;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if index == 0 && matches!(line, "# Netscape HTTP Cookie File" | "# HTTP Cookie File") {
            has_header = true;
        }
        if line.is_empty() || (line.starts_with('#') && !line.starts_with("#HttpOnly_")) {
            lines.push(line.to_string());
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 7
            || fields[0].is_empty()
            || !matches!(fields[1], "TRUE" | "FALSE")
            || !fields[2].starts_with('/')
            || !matches!(fields[3], "TRUE" | "FALSE")
            || fields[4].parse::<i64>().is_err()
            || fields[5].is_empty()
        {
            return Err(format!("err_invalid_cookie_line:{}", index + 1));
        }
        cookie_count += 1;
        lines.push(line.to_string());
    }
    if !has_header {
        return Err("err_invalid_cookie_header".into());
    }
    if cookie_count == 0 {
        return Err("err_empty_cookie_file".into());
    }
    Ok(format!("{}\n", lines.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::normalize_netscape_cookie_file;

    #[test]
    fn validates_and_normalizes_cookie_file() {
        let input = "# Netscape HTTP Cookie File\r\n.example.com\tTRUE\t/\tTRUE\t0\tsid\tvalue\r\n";
        let normalized = normalize_netscape_cookie_file(input).unwrap();
        assert_eq!(
            normalized,
            "# Netscape HTTP Cookie File\n.example.com\tTRUE\t/\tTRUE\t0\tsid\tvalue\n"
        );
    }

    #[test]
    fn rejects_incomplete_cookie_rows() {
        let input = "# Netscape HTTP Cookie File\n.example.com\tTRUE\t/\tTRUE\t0\tsid";
        assert!(normalize_netscape_cookie_file(input).is_err());
    }
}

// 视频信息

/// 浏览/搜索结果中的单个视频条目（yt-dlp --flat-playlist 输出的轻量字段）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseEntry {
    pub id: String,
    pub title: String,
    pub url: String,
    pub thumbnail: String,
    pub duration: Option<f64>,
    pub uploader: String,
    pub view_count: Option<u64>,
    pub is_live: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseResults {
    pub title: String,
    pub entries: Vec<BrowseEntry>,
}

/// 从 yt-dlp flat-playlist JSON 中提取浏览条目。
/// 兼容三种形态：播放列表/搜索结果（entries 数组）、单个视频（顶层即视频信息）。
fn extract_browse_entries(value: &Value) -> Vec<BrowseEntry> {
    let pick_thumbnail = |entry: &Value| -> String {
        if let Some(thumbnails) = entry.get("thumbnails").and_then(|v| v.as_array()) {
            if let Some(best) = thumbnails
                .iter()
                .filter_map(|t| t.get("url").and_then(|u| u.as_str()))
                .last()
            {
                return best.to_string();
            }
        }
        entry
            .get("thumbnail")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string()
    };

    let to_entry = |entry: &Value| -> Option<BrowseEntry> {
        let url = entry
            .get("webpage_url")
            .or_else(|| entry.get("url"))
            .and_then(|u| u.as_str())
            .unwrap_or_default()
            .to_string();
        if url.is_empty() {
            return None;
        }
        let live_status = entry.get("live_status").and_then(|v| v.as_str()).unwrap_or("");
        Some(BrowseEntry {
            id: entry
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            title: entry
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            url,
            thumbnail: pick_thumbnail(entry),
            duration: entry.get("duration").and_then(|v| v.as_f64()),
            uploader: entry
                .get("uploader")
                .or_else(|| entry.get("channel"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            view_count: entry.get("view_count").and_then(|v| v.as_u64()),
            is_live: live_status == "is_live",
        })
    };

    if let Some(entries) = value.get("entries").and_then(|v| v.as_array()) {
        return entries.iter().filter_map(to_entry).collect();
    }
    // 单个视频页：flat-playlist 对 watch URL 直接返回视频信息而非 entries
    to_entry(value).into_iter().collect()
}

/// 浏览 YouTube：关键词走 ytsearch 搜索，URL 走 flat-playlist（支持频道/播放列表/单视频）。
/// 返回轻量条目列表，前端选中后再按需拉取完整格式信息。
#[tauri::command]
pub async fn fetch_browse_results(
    app: AppHandle,
    input: String,
    cookie_file: Option<String>,
    cookie_browser: Option<String>,
    proxy: Option<String>,
) -> Result<BrowseResults, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("err_browse_empty_input".to_string());
    }
    let is_url = trimmed.starts_with("http://") || trimmed.starts_with("https://");
    // 搜索一次性拉取 200 条相关结果（yt-dlp ytsearch 支持大 N，实测 300 约 20s）；
    // URL 形态（频道/播放列表）同样放宽到 200 条，flat-playlist 按页拉取开销可控。
    let target = if is_url {
        trimmed.to_string()
    } else {
        format!("ytsearch200:{}", trimmed)
    };
    let value = support::run_ytdlp_json(
        &app,
        &target,
        &["--flat-playlist", "--playlist-end", "200"],
        cookie_file.as_deref(),
        cookie_browser.as_deref(),
        proxy.as_deref(),
    )
    .await?;

    let entries = extract_browse_entries(&value);
    if entries.is_empty() {
        return Err("err_browse_no_results".to_string());
    }
    Ok(BrowseResults {
        title: value
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        entries,
    })
}

/// 使用 yt-dlp -J 获取视频元信息（标题、格式列表、字幕等）
#[tauri::command]
pub async fn fetch_video_info(
    app: AppHandle,
    url: String,
    cookie_file: Option<String>,
    cookie_browser: Option<String>,
    proxy: Option<String>,
) -> Result<Value, String> {
    support::run_ytdlp_json(
        &app,
        &url,
        &[],
        cookie_file.as_deref(),
        cookie_browser.as_deref(),
        proxy.as_deref(),
    )
    .await
}
