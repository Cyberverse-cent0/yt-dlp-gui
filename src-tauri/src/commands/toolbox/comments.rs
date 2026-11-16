//! 视频评论查询。

use crate::commands::support;
use serde_json::Value;
use tauri::AppHandle;

/// 视频评论
#[derive(serde::Serialize, Clone)]
pub struct VideoComment {
    pub id: String,
    /// 父评论 ID（顶级评论为 "root"）
    pub parent: String,
    pub author: String,
    pub author_id: String,
    pub text: String,
    /// Unix 时间戳（秒）
    pub timestamp: i64,
    pub like_count: i64,
    pub is_favorited: bool,
    pub author_is_uploader: bool,
}

/// 评论条数上限，与前端滑块的最大值保持一致。
///
/// yt-dlp 会按该值逐页拉取，后端不设上限时一个超大值（u32 上限约 42 亿）
/// 会让任务长时间不返回并持续占用内存。
const MAX_COMMENTS: u32 = 5000;

/// 评论排序方式
fn comment_sort_value(sort: &str) -> &'static str {
    match sort {
        "top" => "top",
        _ => "new",
    }
}

/// 拉取并解析视频评论的共享实现。
/// `run_id` 非空时把 yt-dlp 子进程登记到进程注册表，供工具任务取消时终止；
/// 观看页等一次性调用传 None。
pub(crate) async fn fetch_comments(
    app: &AppHandle,
    run_id: Option<&str>,
    url: &str,
    max_comments: u32,
    sort: &str,
    cookie_file: Option<&str>,
    cookie_browser: Option<&str>,
    proxy: Option<&str>,
) -> Result<Value, String> {
    let max_str = max_comments.clamp(1, MAX_COMMENTS).to_string();
    let extractor_arg = format!(
        "youtube:max_comments={};comment_sort={}",
        max_str,
        comment_sort_value(sort)
    );

    let info = support::run_ytdlp_json_tracked(
        app,
        run_id,
        url,
        &[
            "--no-check-formats",
            "--no-playlist",
            "--write-comments",
            "--extractor-args",
            &extractor_arg,
        ],
        cookie_file,
        cookie_browser,
        proxy,
    )
    .await?;

    let comments_raw = info
        .get("comments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let comments: Vec<VideoComment> = comments_raw
        .into_iter()
        .map(|c| VideoComment {
            id: c
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            parent: c
                .get("parent")
                .and_then(Value::as_str)
                .unwrap_or("root")
                .to_string(),
            author: c
                .get("author")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            author_id: c
                .get("author_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            text: c
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            timestamp: c.get("timestamp").and_then(Value::as_i64).unwrap_or(0),
            like_count: c.get("like_count").and_then(Value::as_i64).unwrap_or(0),
            is_favorited: c
                .get("is_favorited")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            author_is_uploader: c
                .get("author_is_uploader")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
        .collect();

    Ok(serde_json::json!({
        "title": info.get("title").cloned().unwrap_or(Value::Null),
        "comment_count": info.get("comment_count").cloned().unwrap_or(Value::Null),
        "comments": comments,
    }))
}

/// 获取视频评论（仅支持 YouTube；其他站点可能没有 comments 字段）
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn tool_fetch_comments(
    app: AppHandle,
    run_id: String,
    url: String,
    max_comments: u32,
    sort: String,
    cookie_file: Option<String>,
    cookie_browser: Option<String>,
    proxy: Option<String>,
) -> Result<Value, String> {
    fetch_comments(
        &app,
        Some(&run_id),
        &url,
        max_comments,
        &sort,
        cookie_file.as_deref(),
        cookie_browser.as_deref(),
        proxy.as_deref(),
    )
    .await
}

/// 观看页评论加载：一次性拉取 Top 评论（含回复），不走工具任务系统，
/// 避免与工具箱 Comments 页共享同一 toolId 状态。
#[tauri::command]
pub async fn fetch_video_comments(
    app: AppHandle,
    url: String,
    cookie_file: Option<String>,
    cookie_browser: Option<String>,
    proxy: Option<String>,
) -> Result<Value, String> {
    fetch_comments(
        &app,
        None,
        &url,
        100,
        "top",
        cookie_file.as_deref(),
        cookie_browser.as_deref(),
        proxy.as_deref(),
    )
    .await
}
