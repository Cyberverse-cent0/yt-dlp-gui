//! Linux 显示服务器/合成器兼容 workaround：窗口隐藏后再次显示时，
//! GTK/WebkitGTK 在部分发行版与合成器上会出现两类失效
//! （tauri#11856、tauri#15460、wry#637）：
//!
//! - 窗口装饰按钮（最小化/最大化/关闭）不再响应悬停与点击；
//! - 整个 webview 输入区域失效，页面内按钮同样无响应。
//! 两者都只能通过全屏、最大化或手动缩放窗口来恢复。
//!
//! 本模块在窗口每次获得焦点时执行一组与显示服务器无关的修复序列：
//! 显式聚焦、切换 resizable 属性、±1px 伪 resize，覆盖
//! X11、GNOME/KDE Wayland 以及主流平铺合成器（请求会被忽略，无副作用）。

use std::time::Duration;
use tauri::{PhysicalSize, Size, WebviewWindow, WindowEvent};

/// webview realize / GTK 主循环处理完事件所需的保守延迟。
const REPAIR_DELAY: Duration = Duration::from_millis(50);
/// 伪 resize 两步之间的间隔，避免合成器将两次 resize 合并为一次。
const RESIZE_GAP: Duration = Duration::from_millis(100);

/// 在窗口每次获得焦点时执行显示服务器无关的 surface 修复。
/// 需要在窗口隐藏→显示之后触发才有效，因此挂在 Focused 事件上，
/// 覆盖启动 show、托盘唤起、单实例与深链接等所有显示路径。
pub(crate) fn install(window: WebviewWindow) {
    log_display_session();
    let handler_window = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(true)) {
            repair_window_surfaces(handler_window.clone());
        }
    });
}

/// 输出一次运行时显示环境信息，便于排查特定发行版/合成器问题。
fn log_display_session() {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into());
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let gdk_backend = std::env::var("GDK_BACKEND").unwrap_or_default();
    let wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let x11 = std::env::var("DISPLAY").is_ok();
    eprintln!(
        "[linux-fix] session={session_type} desktop={desktop} gdk_backend={gdk_backend} \
         wayland={wayland} x11={x11}"
    );
}

/// fire-and-forget 修复序列：立即返回，不阻塞窗口事件线程。
fn repair_window_surfaces(window: WebviewWindow) {
    tauri::async_runtime::spawn(async move {
        // 失效模式 A（tauri#10746 / wry#637）：webview 未获得键盘焦点，
        // 首次点击被合成器用作 click-to-activate。
        let _ = window.set_focus();
        tokio::time::sleep(REPAIR_DELAY).await;

        // 失效模式 B：装饰按钮失效。切换 resizable 强制 GTK 重新协商。
        let _ = window.set_resizable(false);
        tokio::time::sleep(REPAIR_DELAY).await;
        let _ = window.set_resizable(true);

        // 失效模式 C：webview 输入区域尺寸协商失败。±1px 伪 resize
        // 模拟用户手动缩放窗口；平铺 Wayland 合成器会忽略该请求。
        nudge_window_size(&window).await;
    });
}

/// 读取当前物理尺寸，加 1px 后再还原，触发一次 size_allocate。
async fn nudge_window_size(window: &WebviewWindow) {
    let Ok(size) = window.inner_size() else {
        return;
    };
    let nudged = PhysicalSize::new(size.width + 1, size.height + 1);
    if window.set_size(Size::Physical(nudged)).is_err() {
        return;
    }
    tokio::time::sleep(RESIZE_GAP).await;
    let _ = window.set_size(Size::Physical(size));
}
