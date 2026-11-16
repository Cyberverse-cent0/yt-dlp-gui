<script setup lang="ts">
import { storeToRefs } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useI18n } from "vue-i18n";
import { useWatchStore } from "@/stores/watch";
import { usePendingStore, createPendingItem } from "@/stores/pending";
import { useSettingStore } from "@/stores/setting";
import { useDownloadLauncher } from "@/composables/useDownloadLauncher";
import { isValidUrl } from "@/utils/validate";
import { formatDuration, formatViewCount } from "@/utils/format";
import { avatarColor, avatarInitial } from "@/utils/avatar";
import type { BrowseEntry, Chapter, CommentNode, VideoInfo } from "@/types";

const { t } = useI18n();
const router = useRouter();
const route = useRoute();
const watchStore = useWatchStore();
const pendingStore = usePendingStore();
const settingStore = useSettingStore();
const { launchDownload } = useDownloadLauncher();

const {
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
} = storeToRefs(watchStore);

const urlInput = ref("");
const downloading = ref(false);
const descExpanded = ref(false);
const autoplayNext = ref(false);
const videoEl = ref<HTMLVideoElement | null>(null);

const videoInfo = computed<VideoInfo | null>(() => current.value?.videoInfo ?? null);
const chapters = computed<Chapter[]>(() => videoInfo.value?.chapters ?? []);

/** 描述框顶部元信息行：播放量 · 日期 · 时长 · 点赞 · 评论（YouTube 样式） */
const metaText = computed(() => {
  const info = videoInfo.value;
  if (!info) return "";
  const parts: string[] = [];
  if (info.view_count) parts.push(`${formatViewCount(info.view_count)}${t("detail.views")}`);
  if (info.upload_date && info.upload_date.length === 8) {
    parts.push(
      `${info.upload_date.slice(0, 4)}-${info.upload_date.slice(4, 6)}-${info.upload_date.slice(6, 8)}`,
    );
  }
  if (info.duration) parts.push(formatDuration(info.duration, ""));
  if (info.like_count) parts.push(`${formatViewCount(info.like_count)} ${t("watch.likes")}`);
  return parts.join(" · ");
});

const channelName = computed(
  () => videoInfo.value?.channel || videoInfo.value?.uploader || "",
);

/** 评论总数：优先元数据 comment_count，未加载时退回已拉取条数 */
const commentTotal = computed(
  () => videoInfo.value?.comment_count ?? comments.value?.length ?? 0,
);

/** 评论树：parent 指向已存在评论 id 的归为其回复，parent=root 或悬空的为顶层 */
const commentTree = computed<CommentNode[]>(() => {
  const list = comments.value;
  if (!list?.length) return [];
  const nodes = new Map<string, CommentNode>();
  list.forEach((comment, index) => {
    nodes.set(comment.id || `__idx_${index}`, { ...comment, replies: [] });
  });
  const roots: CommentNode[] = [];
  list.forEach((comment, index) => {
    const node = nodes.get(comment.id || `__idx_${index}`);
    if (!node) return;
    const parent =
      comment.parent && comment.parent !== "root" ? nodes.get(comment.parent) : undefined;
    if (parent) parent.replies.push(node);
    else roots.push(node);
  });
  return roots;
});

/** 按需加载评论（Top 排序） */
const handleLoadComments = (): void => {
  void watchStore.loadComments();
};

/** 根据路由 query 中的 url 参数加载视频 */
const loadFromQuery = (rawUrl: unknown): void => {
  if (typeof rawUrl === "string" && rawUrl.trim()) {
    urlInput.value = rawUrl;
    descExpanded.value = false;
    void watchStore.open(rawUrl);
  }
};

onMounted(() => loadFromQuery(route.query.url));

watch(
  () => route.query.url,
  (newUrl) => loadFromQuery(newUrl),
);

/** 输入框播放：同步到路由 query，由 watcher 统一触发加载 */
const handlePlay = (): void => {
  const trimmed = urlInput.value.trim();
  if (!trimmed) return;
  if (!isValidUrl(trimmed)) {
    window.$message.warning(t("home.enterValidUrl"));
    return;
  }
  void router.replace({ name: "watch", query: { url: trimmed } });
};

/** 相关视频点击：同页切换播放 */
const handleRelated = (entry: BrowseEntry): void => {
  void router.replace({ name: "watch", query: { url: entry.url } });
};

const handleBack = (): void => {
  if (window.history.length > 1) router.back();
  else router.push({ name: "browse" });
};

/** 分享：复制当前视频链接到剪贴板 */
const handleShare = async (): Promise<void> => {
  const data = watchStore.current;
  if (!data) return;
  try {
    await writeText(data.url);
    window.$message.success(t("watch.linkCopied"));
  } catch {
    window.$message.warning(t("clipboard.readFailed"));
  }
};

/** 章节点击：仅直连流（<video>）支持跳转，YouTube iframe 无法跨域控制 */
const handleChapterClick = (chapter: Chapter): void => {
  const el = videoEl.value;
  if (!el) return;
  el.currentTime = chapter.start_time;
  void el.play().catch(() => {});
};

/** 直连流播放结束时按 YouTube 自动播放逻辑切换到第一个相关视频 */
const handleVideoEnded = (): void => {
  if (!autoplayNext.value || !related.value.length) return;
  handleRelated(related.value[0]);
};

/** 快速下载：使用快速下载预设配置，就地启动不跳转页面 */
const handleQuickDownload = async (): Promise<void> => {
  const data = watchStore.current;
  if (!data || downloading.value) return;

  if (!settingStore.downloadDir) {
    try {
      settingStore.downloadDir = await invoke<string>("get_default_download_dir");
    } catch {
      window.$message.warning(t("detail.setDownloadDirFirst"));
      return;
    }
  }

  downloading.value = true;
  try {
    const result = await launchDownload(createPendingItem(data, true));
    if (result === "started" || result === "queued") {
      window.$message.success(
        result === "queued" ? t("watch.downloadQueued") : t("watch.downloadStarted"),
      );
    }
  } finally {
    downloading.value = false;
  }
};

/** 高级下载：携带完整解析数据进入待下载配置页 */
const handleAdvanced = (): void => {
  const data = watchStore.current;
  if (!data) return;
  pendingStore.add(data);
  void router.push({ name: "pending" });
};
</script>

<template>
  <div class="watch-page">
    <n-flex vertical :size="16" class="watch-container">
      <n-flex align="center" :size="8" :wrap="false">
        <n-button size="small" strong secondary circle @click="handleBack">
          <template #icon>
            <n-icon><icon-mdi-arrow-left /></n-icon>
          </template>
        </n-button>
        <n-input
          v-model:value="urlInput"
          size="small"
          round
          clearable
          :placeholder="$t('watch.inputPlaceholder')"
          @keyup.enter="handlePlay"
        >
          <template #prefix>
            <n-icon><icon-mdi-play-circle-outline /></n-icon>
          </template>
        </n-input>
        <n-button type="primary" size="small" round :disabled="!urlInput.trim()" @click="handlePlay">
          {{ $t("watch.play") }}
        </n-button>
      </n-flex>

      <n-spin :show="loading" class="watch-spin">
        <template v-if="current">
          <div class="watch-layout">
            <div class="watch-main">
              <div class="player-box">
                <iframe
                  v-if="watchStore.embedUrl"
                  :src="watchStore.embedUrl"
                  class="player-iframe"
                  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                  allowfullscreen
                  referrerpolicy="strict-origin-when-cross-origin"
                />
                <video
                  v-else-if="streamUrl"
                  ref="videoEl"
                  :src="streamUrl"
                  class="player-video"
                  controls
                  autoplay
                  playsinline
                  @ended="handleVideoEnded"
                />
                <div v-else class="player-status">
                  <n-spin v-if="streamLoading || loading" size="large" />
                  <n-empty
                    v-else-if="streamFailed"
                    :description="$t('watch.streamUnavailable')"
                    size="small"
                  >
                    <template #extra>
                      <n-button size="small" @click="watchStore.retryStream()">
                        {{ $t("watch.retry") }}
                      </n-button>
                    </template>
                  </n-empty>
                </div>
              </div>

              <div class="watch-info">
                <n-text class="watch-title">{{ videoInfo?.title }}</n-text>

                <div class="channel-row">
                  <div class="channel-avatar" :style="{ background: avatarColor(channelName || '?') }">
                    {{ avatarInitial(channelName || "?") }}
                  </div>
                  <div class="channel-meta">
                    <n-text class="channel-name">{{ channelName || $t("common.unknown") }}</n-text>
                    <n-text v-if="metaText" depth="3" class="channel-sub">{{ metaText }}</n-text>
                  </div>
                </div>

                <div class="action-row">
                  <n-button
                    type="primary"
                    size="medium"
                    round
                    :loading="downloading"
                    @click="handleQuickDownload"
                  >
                    <template #icon>
                      <n-icon size="18"><icon-mdi-youtube /></n-icon>
                    </template>
                    {{ $t("watch.download") }}
                  </n-button>
                  <n-button size="medium" round secondary @click="handleAdvanced">
                    <template #icon>
                      <n-icon><icon-mdi-tune-variant /></n-icon>
                    </template>
                    {{ $t("watch.advanced") }}
                  </n-button>
                  <n-button size="medium" round secondary @click="handleShare">
                    <template #icon>
                      <n-icon><icon-mdi-share-variant /></n-icon>
                    </template>
                    {{ $t("watch.share") }}
                  </n-button>
                </div>

                <div v-if="metaText || videoInfo?.description" class="desc-box">
                  <n-text v-if="metaText" class="desc-meta">{{ metaText }}</n-text>
                  <template v-if="videoInfo?.description">
                    <n-ellipsis
                      v-if="!descExpanded"
                      :line-clamp="3"
                      :tooltip="false"
                      class="watch-desc"
                    >
                      {{ videoInfo.description }}
                    </n-ellipsis>
                    <div v-else class="watch-desc watch-desc-full">
                      {{ videoInfo.description }}
                    </div>
                    <button type="button" class="desc-toggle" @click="descExpanded = !descExpanded">
                      {{ descExpanded ? $t("watch.showLess") : $t("watch.showMore") }}
                    </button>
                  </template>
                </div>

                <n-card v-if="chapters.length" size="small" class="chapters-card">
                  <template #header>
                    <n-flex align="center" :size="8">
                      <n-icon size="16"><icon-mdi-format-list-numbered /></n-icon>
                      <span>{{ $t("watch.chapters") }}</span>
                      <n-tag size="small" round :bordered="false" type="info">
                        {{ chapters.length }}
                      </n-tag>
                    </n-flex>
                  </template>
                  <div class="chapters-list">
                    <div
                      v-for="(chapter, index) in chapters"
                      :key="index"
                      class="chapter-item"
                      @click="handleChapterClick(chapter)"
                    >
                      <n-text class="chapter-index">{{ index + 1 }}</n-text>
                      <n-text class="chapter-title">{{ chapter.title }}</n-text>
                      <n-text depth="3" class="chapter-time">
                        {{ formatDuration(chapter.start_time, "0:00") }}
                      </n-text>
                    </div>
                  </div>
                </n-card>

                <div class="comments-section">
                  <n-flex align="center" :size="8">
                    <n-icon size="18"><icon-mdi-comment-text-outline /></n-icon>
                    <n-text strong>{{ $t("watch.comments") }}</n-text>
                    <n-tag
                      v-if="commentTotal > 0"
                      size="small"
                      round
                      :bordered="false"
                      type="info"
                    >
                      {{ formatViewCount(commentTotal) }}
                    </n-tag>
                  </n-flex>

                  <template v-if="comments === null && !commentsFailed && !commentsLoading">
                    <n-button size="small" secondary round @click="handleLoadComments">
                      <template #icon>
                        <n-icon><icon-mdi-comment-plus-outline /></n-icon>
                      </template>
                      {{ $t("watch.loadComments") }}
                    </n-button>
                  </template>
                  <n-empty
                    v-else-if="commentsFailed"
                    :description="$t('watch.commentsFailed')"
                    size="small"
                  >
                    <template #extra>
                      <n-button size="small" @click="handleLoadComments">
                        {{ $t("watch.retry") }}
                      </n-button>
                    </template>
                  </n-empty>
                  <n-empty
                    v-else-if="commentTree.length === 0 && !commentsLoading"
                    :description="$t('watch.noComments')"
                    size="small"
                  />
                  <n-spin v-else :show="commentsLoading">
                    <CommentItem
                      v-for="comment in commentTree"
                      :key="comment.id || comment.timestamp"
                      :comment="comment"
                    />
                  </n-spin>
                </div>
              </div>
            </div>

            <div class="watch-side">
              <n-flex align="center" justify="space-between" style="margin-bottom: 12px">
                <n-flex align="center" :size="8">
                  <n-icon size="18"><icon-mdi-fire /></n-icon>
                  <n-text strong>{{ $t("watch.related") }}</n-text>
                </n-flex>
                <n-flex align="center" :size="6">
                  <n-text depth="3" style="font-size: 12px">{{ $t("watch.autoplay") }}</n-text>
                  <n-switch v-model:value="autoplayNext" size="small" />
                </n-flex>
              </n-flex>
              <n-spin :show="relatedLoading">
                <n-empty
                  v-if="!related.length && !relatedLoading"
                  :description="$t('watch.noRelated')"
                  size="small"
                />
                <div v-else class="related-list">
                  <div
                    v-for="entry in related"
                    :key="entry.url"
                    class="related-item"
                    @click="handleRelated(entry)"
                  >
                    <div class="related-thumb">
                      <img
                        v-if="entry.thumbnail"
                        :src="entry.thumbnail"
                        loading="lazy"
                        alt=""
                      />
                      <div v-else class="related-thumb-placeholder">
                        <icon-mdi-image-outline />
                      </div>
                      <span v-if="entry.duration != null" class="related-duration">
                        {{ formatDuration(entry.duration, "0:00") }}
                      </span>
                    </div>
                    <div class="related-meta">
                      <n-ellipsis :line-clamp="2" class="related-title">{{ entry.title }}</n-ellipsis>
                      <n-flex align="center" :size="6" :wrap="false">
                        <n-text depth="3" class="related-sub">
                          {{ entry.uploader || $t("common.unknown") }}
                        </n-text>
                        <n-text v-if="entry.viewCount" depth="3" class="related-sub">
                          · {{ formatViewCount(entry.viewCount) }}{{ $t("detail.views") }}
                        </n-text>
                      </n-flex>
                    </div>
                  </div>
                </div>
              </n-spin>
            </div>
          </div>
        </template>
        <n-empty
          v-else-if="!loading"
          :description="$t('watch.emptyHint')"
          class="watch-empty"
        />
      </n-spin>
    </n-flex>
  </div>
</template>

<style scoped lang="scss">
.watch-page {
  height: 100%;
  overflow-y: auto;
  padding: 24px;

  .watch-container {
    max-width: 1400px;
    margin: 0 auto;
  }

  .watch-spin {
    min-height: 300px;
  }

  .watch-empty {
    margin-top: 120px;
  }

  .watch-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 400px;
    gap: 24px;
    align-items: start;
  }

  .player-box {
    position: relative;
    width: 100%;
    aspect-ratio: 16 / 9;
    background: #000;
    border-radius: 12px;
    overflow: hidden;

    .player-iframe,
    .player-video {
      width: 100%;
      height: 100%;
      border: 0;
      display: block;
      object-fit: contain;
    }

    .player-status {
      position: absolute;
      inset: 0;
      display: grid;
      place-items: center;
    }
  }

  .watch-info {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .watch-title {
    font-size: 20px;
    font-weight: 700;
    line-height: 1.4;
  }

  .channel-row {
    display: flex;
    align-items: center;
    gap: 12px;

    .channel-avatar {
      width: 40px;
      height: 40px;
      border-radius: 50%;
      display: grid;
      place-items: center;
      color: #fff;
      font-size: 18px;
      font-weight: 700;
      flex-shrink: 0;
      user-select: none;
    }

    .channel-meta {
      min-width: 0;
      display: flex;
      flex-direction: column;

      .channel-name {
        font-size: 15px;
        font-weight: 600;
      }

      .channel-sub {
        font-size: 12px;
      }
    }
  }

  .action-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .desc-box {
    background: rgba(127, 127, 127, 0.12);
    border-radius: 12px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;

    .desc-meta {
      font-size: 13px;
      font-weight: 600;
    }

    .watch-desc {
      font-size: 13px;
      white-space: pre-wrap;
    }

    .watch-desc-full {
      white-space: pre-wrap;
    }

    .desc-toggle {
      align-self: flex-start;
      background: none;
      border: 0;
      padding: 0;
      font: inherit;
      font-size: 13px;
      font-weight: 600;
      color: var(--n-text-color);
      cursor: pointer;

      &:hover {
        color: var(--n-primary-color);
      }
    }
  }

  .chapters-card {
    .chapters-list {
      display: flex;
      flex-direction: column;
    }

    .chapter-item {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 6px 4px;
      border-radius: 6px;
      cursor: pointer;

      &:hover {
        background: rgba(127, 127, 127, 0.1);
      }

      .chapter-index {
        flex: 0 0 20px;
        text-align: center;
        font-size: 12px;
        font-weight: 700;
        color: var(--n-primary-color);
      }

      .chapter-title {
        flex: 1;
        min-width: 0;
        font-size: 13px;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .chapter-time {
        font-size: 12px;
        font-variant-numeric: tabular-nums;
        flex-shrink: 0;
      }
    }
  }

  .related-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .related-item {
    display: flex;
    gap: 10px;
    cursor: pointer;
    border-radius: 8px;
    padding: 4px;

    &:hover {
      background: rgba(127, 127, 127, 0.1);
    }

    .related-thumb {
      position: relative;
      flex: 0 0 168px;
      aspect-ratio: 16 / 9;
      border-radius: 8px;
      overflow: hidden;
      background: rgba(127, 127, 127, 0.15);

      img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
      }

      .related-thumb-placeholder {
        width: 100%;
        height: 100%;
        display: grid;
        place-items: center;
        font-size: 24px;
        opacity: 0.4;
      }

      .related-duration {
        position: absolute;
        bottom: 4px;
        right: 4px;
        background: rgba(0, 0, 0, 0.75);
        color: #fff;
        font-size: 11px;
        padding: 0 4px;
        border-radius: 3px;
        font-variant-numeric: tabular-nums;
      }
    }

    .related-meta {
      flex: 1;
      min-width: 0;
      display: flex;
      flex-direction: column;
      gap: 4px;
      padding-top: 2px;

      .related-title {
        font-size: 13px;
        font-weight: 600;
        line-height: 1.35;
      }

      .related-sub {
        font-size: 12px;
        white-space: nowrap;
      }
    }
  }

  @media (max-width: 1100px) {
    .watch-layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }
}
</style>
