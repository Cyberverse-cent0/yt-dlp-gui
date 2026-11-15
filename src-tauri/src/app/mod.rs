//! Tauri 应用壳层：CLI、应用级命令与系统托盘。

pub(crate) mod browser_bridge;
pub(crate) mod cli;
pub(crate) mod commands;
#[cfg(target_os = "linux")]
pub(crate) mod linux_fix;
mod tray;

pub(crate) use tray::setup_tray;
