<!-- 全局配置 -->
<template>
  <n-config-provider
    :locale="naiveLocale"
    :date-locale="naiveDateLocale"
    :theme="theme"
    :theme-overrides="themeOverrides"
    abstract
    inline-theme-disabled
    preflight-style-disabled
  >
    <n-global-style />
    <n-loading-bar-provider>
      <n-dialog-provider>
        <n-notification-provider>
          <n-message-provider :max="1" placement="bottom">
            <n-modal-provider>
              <slot />
              <NaiveProviderContent />
            </n-modal-provider>
          </n-message-provider>
        </n-notification-provider>
      </n-dialog-provider>
    </n-loading-bar-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import {
  zhCN,
  dateZhCN,
  enUS,
  dateEnUS,
  darkTheme,
  useLoadingBar,
  useModal,
  useDialog,
  useMessage,
  useNotification,
  GlobalThemeOverrides,
} from "naive-ui";
import { useI18n } from "vue-i18n";
import { useSettingStore } from "@/stores/setting";
import {
  SYSTEM_THEME_CACHE_KEY,
  syncWindowTheme,
  useSystemTheme,
} from "@/composables/useSystemTheme";

// 设置
const settingStore = useSettingStore();
const { locale } = useI18n();

// Naive UI 语言：中文用中文，其他一律英文
const naiveLocale = computed(() => (locale.value.startsWith("zh") ? zhCN : enUS));
const naiveDateLocale = computed(() => (locale.value.startsWith("zh") ? dateZhCN : dateEnUS));

// 操作系统主题（原生检测，WebKitGTK/WebView2 的 prefers-color-scheme 不可靠）
const { systemTheme, refreshSystemTheme } = useSystemTheme();

// 全局主题
const themeOverrides = shallowRef<GlobalThemeOverrides>({
  common: {
    borderRadius: "8px",
  },
});

// 获取明暗模式
const theme = computed(() => {
  return settingStore.themeMode === "auto"
    ? // 跟随系统
      systemTheme.value === "dark"
      ? darkTheme
      : null
    : // 自定义
      settingStore.themeMode === "dark"
      ? darkTheme
      : null;
});

watch(
  () => settingStore.themeMode,
  (mode) => {
    void (async () => {
      // 必须先清除/重设窗口主题覆盖，再探测系统主题，
      // 否则 window.theme() 仍返回显式覆盖值。
      await syncWindowTheme();
      // 显式主题期间探测到的只是覆盖值，切回 auto 时重新探测真实系统主题。
      if (mode === "auto") await refreshSystemTheme();
    })();
  },
  { immediate: true },
);

// 系统主题变化时重新同步原生窗口主题：
// Linux 上 portal 主题变更会写入 preferred_theme 覆盖 set_theme 的
// 显式选择，需要在事件后重申一次。
watch(systemTheme, () => void syncWindowTheme());

// index.html 启动闪屏在应用挂载前读取该缓存决定背景色，
// 仅在 auto 模式下记录，避免手动主题污染系统主题快照。
watch(
  [() => settingStore.themeMode, systemTheme],
  ([mode, theme]) => {
    if (mode === "auto") {
      try {
        localStorage.setItem(SYSTEM_THEME_CACHE_KEY, theme);
      } catch {
        // localStorage 不可用时忽略
      }
    }
  },
  { immediate: true },
);

// 挂载工具
const NaiveProviderContent = defineComponent({
  setup() {
    // 进度条
    window.$loadingBar = useLoadingBar();
    // 通知
    window.$notification = useNotification();
    // 信息
    window.$message = useMessage();
    // 对话框
    window.$dialog = useDialog();
    // 模态框
    window.$modal = useModal();
  },
  render() {
    return h("div", { className: "main-tools" });
  },
});
</script>
