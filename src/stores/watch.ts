import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useVideoStore } from "@/stores/video";
import { useSettingStore } from "@/stores/setting";
import { getYouTubeEmbedUrl, getYouTubeVideoId, isYouTubeUrl } from "@/utils/youtube";
import type { BrowseEntry, BrowseResults, CommentsInfo, FetchedVideoData, VideoComment } from "@/types";

/** 相关视频条数上限 */
const RELATED_LIMIT = 20;

/**
 * 观看页 Store
 *
 * 保存当前播放的视频数据、流地址状态与相关视频列表，
 * 支持在观看页内直接切换视频而无需离开页面。
 */
export const useWatchStore = defineStore("watch", () => {
  /** 当前播放的视频完整数据 */
  const current = ref<FetchedVideoData | null>(null);
  /** 视频信息拉取中 */
  const loading = ref(false);
  /** 非 YouTube 站点的直连流地址 */
  const streamUrl = ref<string | null>(null);
  const streamLoading = ref(false);
  const streamFailed = ref(false);
  /** 相关视频列表 */
  const related = ref<BrowseEntry[]>([]);
  const relatedLoading = ref(false);
  /** 视频评论（null 表示尚未加载） */
  const comments = ref<VideoComment[] | null>(null);
  const commentsLoading = ref(false);
  const commentsFailed = ref(false);

  /** YouTube 视频 ID（非 YouTube 或无法识别时为 null） */
  const youtubeId = computed(() =>
    current.value ? getYouTubeVideoId(current.value.url) : null,
  );

  /** YouTube 嵌入播放器地址；非 YouTube 时为 null */
  const embedUrl = computed(() =>
    youtubeId.value ? getYouTubeEmbedUrl(youtubeId.value) : null,
  );

  /** 当前是否走 YouTube 嵌入播放 */
  const isYouTube = computed(() => current.value != null && isYouTubeUrl(current.value.url));

  /**
   * 拉取非 YouTube 站点的可播放流地址。
   * YouTube 走嵌入播放器，无需直连流；失败时置 streamFailed 由页面展示重试入口。
   */
  const loadStream = async (data: FetchedVideoData): Promise<void> => {
    streamUrl.value = null;
    streamFailed.value = false;
    if (isYouTubeUrl(data.url)) return;

    streamLoading.value = true;
    try {
      const { cookieFile, cookieBrowser } = await useVideoStore().getCookieArgs();
      streamUrl.value = await invoke<string>("fetch_stream_url", {
        url: data.url,
        cookieFile,
        cookieBrowser,
        proxy: useSettingStore().proxy || null,
      });
    } catch {
      streamFailed.value = true;
    } finally {
      streamLoading.value = false;
    }
  };

  /**
   * 拉取相关视频：优先使用频道主页 URL 浏览该频道视频，
   * 无频道链接时退回到上传者名称 / 标题关键词搜索。
   */
  const loadRelated = async (data: FetchedVideoData): Promise<void> => {
    related.value = [];
    relatedLoading.value = true;
    try {
      const info = data.videoInfo;
      const channelUrl = info.channel_url || info.uploader_url;
      const isChannelUrl =
        typeof channelUrl === "string" &&
        /^https?:\/\//.test(channelUrl) &&
        isYouTubeUrl(channelUrl) &&
        !getYouTubeVideoId(channelUrl);
      const query = isChannelUrl
        ? channelUrl
        : info.uploader || info.channel || info.title;
      if (!query) return;

      const { cookieFile, cookieBrowser } = await useVideoStore().getCookieArgs();
      const results = await invoke<BrowseResults>("fetch_browse_results", {
        input: query,
        cookieFile,
        cookieBrowser,
        proxy: useSettingStore().proxy || null,
      });
      related.value = results.entries
        .filter((entry) => entry.url !== data.url)
        .slice(0, RELATED_LIMIT);
    } catch {
      // 相关视频拉取失败不阻塞播放，仅清空列表
      related.value = [];
    } finally {
      relatedLoading.value = false;
    }
  };

  /**
   * 按需加载视频评论（Top 排序，最多 100 条，含回复）。
   * 走独立的 fetch_video_comments 命令，不占用工具箱任务系统。
   */
  const loadComments = async (): Promise<void> => {
    const data = current.value;
    if (!data || commentsLoading.value || comments.value !== null) return;

    commentsLoading.value = true;
    commentsFailed.value = false;
    try {
      const { cookieFile, cookieBrowser } = await useVideoStore().getCookieArgs();
      const info = await invoke<CommentsInfo>("fetch_video_comments", {
        url: data.url,
        cookieFile,
        cookieBrowser,
        proxy: useSettingStore().proxy || null,
      });
      comments.value = info.comments ?? [];
    } catch {
      commentsFailed.value = true;
    } finally {
      commentsLoading.value = false;
    }
  };

  /**
   * 打开并加载指定 URL 的视频到观看页
   *
   * @param targetUrl 视频或播放页 URL
   * @returns 是否成功加载
   */
  const open = async (targetUrl: string): Promise<boolean> => {
    const trimmed = targetUrl.trim();
    if (!trimmed || loading.value) return false;

    loading.value = true;
    current.value = null;
    streamUrl.value = null;
    streamFailed.value = false;
    related.value = [];
    comments.value = null;
    commentsFailed.value = false;
    try {
      const data = await useVideoStore().fetchVideoInfo(trimmed);
      if (!data) return false;
      current.value = data;
      // 流地址与相关视频并行拉取，互不阻塞
      void loadStream(data);
      void loadRelated(data);
      return true;
    } finally {
      loading.value = false;
    }
  };

  /** 重试拉取直连流地址 */
  const retryStream = async (): Promise<void> => {
    if (current.value) await loadStream(current.value);
  };

  return {
    current,
    loading,
    streamUrl,
    streamLoading,
    streamFailed,
    related,
    relatedLoading,
    comments,
    commentsLoading,
    commentsFailed,
    loadComments,
    youtubeId,
    embedUrl,
    isYouTube,
    open,
    retryStream,
  };
});
