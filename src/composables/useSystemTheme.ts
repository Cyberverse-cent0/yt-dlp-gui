import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useSettingStore } from "@/stores/setting";

/** index.html 启动闪屏在应用挂载前读取的系统主题缓存键 */
export const SYSTEM_THEME_CACHE_KEY = "ydl-system-theme";

type SystemTheme = "light" | "dark";

const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");

/** 系统明暗主题快照（模块级单例，整个应用共享同一信号源） */
const systemTheme = ref<SystemTheme>(mediaQuery.matches ? "dark" : "light");

/** 原生主题通道是否可用：不可用时（纯浏览器 dev）回退到 matchMedia */
let nativeThemeAvailable = false;

const applySystemTheme = (theme: unknown): void => {
  if (theme === "dark" || theme === "light") {
    systemTheme.value = theme;
  }
};

/** 向原生端重新探测系统主题 */
const refreshSystemTheme = async (): Promise<void> => {
  if (!nativeThemeAvailable) return;
  try {
    applySystemTheme(await invoke<string>("get_system_theme"));
  } catch {
    // 探测失败时保留上一次结果
  }
};

/**
 * 将主题模式同步到原生窗口（标题栏/窗口装饰）。
 * auto 模式传 null 让窗口跟随系统，显式模式固定为 light/dark。
 * 需要在 pinia 激活的上下文中调用（组件 setup / onMounted）。
 */
export const syncWindowTheme = async (): Promise<void> => {
  try {
    const { themeMode } = useSettingStore();
    await getCurrentWindow().setTheme(themeMode === "auto" ? null : themeMode);
  } catch (error) {
    console.warn("[YDL GUI] failed to sync native window theme:", error);
  }
};

let initialized = false;

/**
 * 操作系统明暗主题探测。
 *
 * WebKitGTK / WebView2 的 prefers-color-scheme 在部分发行版与环境下
 * 无法跟随系统主题实时变化，因此优先使用 Tauri 原生窗口主题
 * （Linux 走 xdg-desktop-portal，Windows 走 WM_SETTINGCHANGE，
 * macOS 走 NSAppearance），仅在非 Tauri 环境回退到 matchMedia。
 *
 * @returns 系统主题快照 ref 与手动刷新方法
 */
export const useSystemTheme = () => {
  if (!initialized) {
    initialized = true;

    void invoke<string>("get_system_theme")
      .then((theme) => {
        nativeThemeAvailable = true;
        applySystemTheme(theme);
      })
      .catch(() => {
        // 非 Tauri 环境（纯浏览器 dev server）：跟随 matchMedia
        mediaQuery.addEventListener("change", (event) => {
          applySystemTheme(event.matches ? "dark" : "light");
        });
      });

    // 原生 ThemeChanged 事件：覆盖各平台系统主题的运行时切换
    void listen<string>("system-theme-changed", (event) => {
      applySystemTheme(event.payload);
    }).catch(() => {});

    // 部分平台不派发 ThemeChanged，窗口重新聚焦时兜底重查一次
    window.addEventListener("focus", () => void refreshSystemTheme());
  }

  return { systemTheme, refreshSystemTheme };
};
