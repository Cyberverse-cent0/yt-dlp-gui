import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useVideoStore } from "@/stores/video";
import { useSettingStore } from "@/stores/setting";
import type { BrowseResults } from "@/types";

/**
 * 浏览/搜索页状态 Store
 *
 * 保存搜索关键词、结果列表与勾选状态，
 * 使用户在页面间导航后返回时不丢失已加载的缓冲结果。
 */
export const useBrowseStore = defineStore("browse", () => {
  /** 搜索输入框内容 */
  const input = ref("");
  /** 最近一次搜索结果（null 表示尚未搜索） */
  const results = ref<BrowseResults | null>(null);
  /** 已勾选待加入下载队列的视频 URL */
  const selectedUrls = ref<Set<string>>(new Set());
  /** 结果列表滚动位置，导航返回时恢复 */
  const scrollTop = ref(0);

  const hasSearched = computed(() => results.value !== null);

  /**
   * 执行搜索：清空旧勾选并拉取新结果，错误向上抛给调用方处理
   *
   * @returns Promise<void>
   */
  const search = async (): Promise<void> => {
    const keyword = input.value.trim();
    if (!keyword) return;
    selectedUrls.value = new Set();
    const { cookieFile, cookieBrowser } = await useVideoStore().getCookieArgs();
    results.value = await invoke<BrowseResults>("fetch_browse_results", {
      input: keyword,
      cookieFile,
      cookieBrowser,
      proxy: useSettingStore().proxy || null,
    });
  };

  return {
    input,
    results,
    selectedUrls,
    scrollTop,
    hasSearched,
    search,
  };
});
