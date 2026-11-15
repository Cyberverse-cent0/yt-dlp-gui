//! 系统相关命令

use tauri::{AppHandle, Manager, Theme};

/// 获取操作系统当前的明暗主题（"light" / "dark"）。
///
/// WebKitGTK / WebView2 的 prefers-color-scheme 在部分发行版与环境下
/// 无法跟随系统主题，这里改由原生窗口主题检测（Linux 走
/// xdg-desktop-portal，Windows 走 WM_SETTINGCHANGE，macOS 走 NSAppearance）。
///
/// 注意：窗口设置过显式主题（set_theme）时返回的是覆盖值而非系统值，
/// 因此前端只应在 auto 模式下采信该结果。
#[tauri::command]
pub fn get_system_theme(app: AppHandle) -> Result<String, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    match window.theme().map_err(|e| e.to_string())? {
        Theme::Dark => Ok("dark".to_string()),
        _ => Ok("light".to_string()),
    }
}

/// 检测平台默认下载目录
#[tauri::command]
pub fn get_default_download_dir(app: AppHandle) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let downloads = app
            .path()
            .download_dir()
            .map_err(|e| format!("Failed to get download directory: {}", e))?;
        Ok(downloads.to_string_lossy().to_string())
    }

    #[cfg(target_os = "macos")]
    {
        use std::path::PathBuf;
        let _ = app;
        let home = std::env::var("HOME").map_err(|e| format!("Failed to get HOME: {}", e))?;
        let downloads = PathBuf::from(home).join("Downloads");
        Ok(downloads.to_string_lossy().to_string())
    }

    #[cfg(target_os = "linux")]
    {
        use std::path::PathBuf;
        let _ = app;
        // Try XDG_DOWNLOAD_DIR first
        if let Ok(xdg) = std::env::var("XDG_DOWNLOAD_DIR") {
            return Ok(xdg);
        }
        // Fallback to HOME/Downloads
        let home = std::env::var("HOME").map_err(|e| format!("Failed to get HOME: {}", e))?;
        let downloads = PathBuf::from(home).join("Downloads");
        Ok(downloads.to_string_lossy().to_string())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = app;
        Err("Unsupported platform".to_string())
    }
}
