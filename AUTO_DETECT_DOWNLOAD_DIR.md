# Auto-Detect Download Directory Feature

## Overview

The auto-detect download directory feature automatically detects the platform's default download folder for Windows, Linux, and macOS. This feature helps users quickly set up their download location without manually browsing for folders.

## Features

### Platform-Specific Detection

- **Windows**: Uses Tauri's `download_dir()` API to detect the system Downloads folder
- **macOS**: Uses `$HOME/Downloads` as the default location
- **Linux**: 
  - First checks `XDG_DOWNLOAD_DIR` environment variable
  - Falls back to `$HOME/Downloads` if not set

### Implementation

#### Backend (Rust)

Located in `src-tauri/src/commands/system.rs`:

```rust
#[tauri::command]
pub fn get_default_download_dir(_app: AppHandle) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::path::PathBuf;
        let downloads = app
            .path()
            .download_dir()
            .map_err(|e| format!("Failed to get download directory: {}", e))?;
        Ok(downloads.to_string_lossy().to_string())
    }

    #[cfg(target_os = "macos")]
    {
        use std::path::PathBuf;
        let home = std::env::var("HOME").map_err(|e| format!("Failed to get HOME: {}", e))?;
        let downloads = PathBuf::from(home).join("Downloads");
        Ok(downloads.to_string_lossy().to_string())
    }

    #[cfg(target_os = "linux")]
    {
        use std::path::PathBuf;
        // Try XDG_DOWNLOAD_DIR first
        if let Ok(xdg) = std::env::var("XDG_DOWNLOAD_DIR") {
            return Ok(xdg);
        }
        // Fallback to HOME/Downloads
        let home = std::env::var("HOME").map_err(|e| format!("Failed to get HOME: {}", e))?;
        let downloads = PathBuf::from(home).join("Downloads");
        Ok(downloads.to_string_lossy().to_string())
    }
}
```

#### Frontend (Vue)

The feature is integrated in two places:

1. **DownloadDirCard.vue**: Manual auto-detect button
2. **Home.vue**: Automatic detection when download directory is not set

##### DownloadDirCard.vue

```vue
<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";

/** 自动检测默认下载目录 */
const handleAutoDetect = async () => {
  try {
    const defaultDir = await invoke<string>("get_default_download_dir");
    settingStore.downloadDir = defaultDir;
    window.$message.success(t("downloadDir.autoDetectSuccess"));
  } catch (e: unknown) {
    window.$message.error(t("downloadDir.autoDetectFailed", { e }));
  }
};
</script>

<template>
  <n-button size="small" @click="handleAutoDetect">
    <template #icon>
      <n-icon>
        <icon-mdi-folder-heart-outline />
      </n-icon>
    </template>
    {{ $t("downloadDir.autoDetect") }}
  </n-button>
</template>
```

##### Home.vue (Automatic Detection)

```vue
<script setup lang="ts">
/**
 * 校验快速下载模式所需配置（如下载路径是否已设置）
 * 如果未设置，自动检测并提示用户
 */
const ensureQuickDownloadConfigured = async (): Promise<boolean> => {
  if (settingStore.homeDownloadBehavior !== "quick" || settingStore.downloadDir) return true;

  // 尝试自动检测默认下载目录
  try {
    const defaultDir = await invoke<string>("get_default_download_dir");
    settingStore.downloadDir = defaultDir;
    window.$message.success(translate("downloadDir.autoDetectSuccess"));
    return true;
  } catch (e: unknown) {
    window.$message.warning(translate("detail.setDownloadDirFirst"));
    showQuickSettings.value = true;
    return false;
  }
};
</script>
```

## Usage

### Manual Auto-Detect

1. Go to Settings → Download Directory
2. Click the "Auto-detect" button (folder heart icon)
3. The application will automatically detect and set your platform's default Downloads folder

### Automatic Detection

When using Quick Download mode:
- If no download directory is set, the app will automatically detect the default Downloads folder
- If detection succeeds, the directory is set automatically
- If detection fails, the user is prompted to manually select a directory

## Localization

The feature includes localization support:

### English (en-US)
```json
{
  "downloadDir": {
    "autoDetect": "Auto-detect",
    "autoDetectSuccess": "Default download directory detected successfully",
    "autoDetectFailed": "Failed to detect default download directory: {e}"
  }
}
```

### Chinese (zh-CN)
```json
{
  "downloadDir": {
    "autoDetect": "自动检测",
    "autoDetectSuccess": "已成功检测默认下载目录",
    "autoDetectFailed": "检测默认下载目录失败：{e}"
  }
}
```

## Testing

### Windows
- Default: `%USERPROFILE%\Downloads`
- Uses Tauri's built-in `download_dir()` API

### macOS
- Default: `$HOME/Downloads`
- Falls back to user home directory

### Linux
- Default: `$HOME/Downloads` or `XDG_DOWNLOAD_DIR` if set
- Respects XDG Base Directory specification

## Error Handling

The feature includes comprehensive error handling:
- Windows: API failure messages
- macOS/Linux: HOME environment variable not set
- All platforms: Graceful fallback to manual selection

## Future Enhancements

Potential improvements:
- Support for custom platform-specific paths (e.g., `$HOME/Рабочий стол` on Russian systems)
- Detection of multiple download locations (e.g., secondary drives)
- User preference for auto-detect on first launch
