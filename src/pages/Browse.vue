<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { useVideoStore } from "@/stores/video";
import { usePendingStore } from "@/stores/pending";
import { useSettingStore } from "@/stores/setting";
import { formatDuration, formatViewCount } from "@/utils/format";
import type { BrowseEntry, BrowseResults } from "@/types";

const { t } = useI18n();
const router = useRouter();
const videoStore = useVideoStore();
const pendingStore = usePendingStore();
const settingStore = useSettingStore();

const input = ref("");
const loading = ref(false);
const results = ref<BrowseResults | null>(null);
const selectedUrls = ref<Set<string>>(new Set());
const openingUrl = ref("");
const addingAll = ref(false);

const hasSearched = computed(() => results.value !== null);

const toggleSelected = (entry: BrowseEntry) => {
  const next = new Set(selectedUrls.value);
  if (next.has(entry.url)) {
    next.delete(entry.url);
  } else {
    next.add(entry.url);
  }
  selectedUrls.value = next;
};

const isSelected = (entry: BrowseEntry) => selectedUrls.value.has(entry.url);

const handleSearch = async () => {
  const keyword = input.value.trim();
  if (!keyword || loading.value) return;
  loading.value = true;
  selectedUrls.value = new Set();
  try {
    const { cookieFile, cookieBrowser } = await videoStore.getCookieArgs();
    results.value = await invoke<BrowseResults>("fetch_browse_results", {
      input: keyword,
      cookieFile,
      cookieBrowser,
      proxy: settingStore.proxy || null,
    });
  } catch (error: unknown) {
    window.$message.error(
      error instanceof Error ? error.message : String(error) || t("browse.searchFailed"),
    );
    results.value = null;
  } finally {
    loading.value = false;
  }
};

/** 拉取单个视频的完整信息并加入待下载队列，返回是否成功 */
const addEntryToPending = async (entry: BrowseEntry): Promise<boolean> => {
  const data = await videoStore.fetchVideoInfo(entry.url, { silent: true });
  if (!data) return false;
  pendingStore.add(data);
  return true;
};

/** 点击卡片：进入常规详情流程（拉取完整格式后进入待下载页配置） */
const handleOpen = async (entry: BrowseEntry) => {
  if (openingUrl.value) return;
  openingUrl.value = entry.url;
  try {
    const data = await videoStore.fetchVideoInfo(entry.url);
    if (data) {
      pendingStore.add(data);
      router.push({ name: "pending" });
    }
  } finally {
    openingUrl.value = "";
  }
};

/** 批量把勾选的视频加入待下载队列（顺序拉取完整信息） */
const handleAddSelected = async () => {
  if (addingAll.value || selectedUrls.value.size === 0) return;
  const entries = (results.value?.entries ?? []).filter((entry) =>
    selectedUrls.value.has(entry.url),
  );
  addingAll.value = true;
  let added = 0;
  try {
    for (const entry of entries) {
      if (await addEntryToPending(entry)) added++;
    }
    if (added > 0) {
      window.$message.success(t("browse.addedToPending", { n: added }));
      selectedUrls.value = new Set();
    } else {
      window.$message.warning(t("browse.addAllFailed"));
    }
  } finally {
    addingAll.value = false;
  }
};

const handleSelectAll = () => {
  selectedUrls.value = new Set((results.value?.entries ?? []).map((entry) => entry.url));
};

const handleClearSelection = () => {
  selectedUrls.value = new Set();
};
</script>

<template>
  <div class="browse-page">
    <n-flex vertical :size="16" class="browse-container">
      <n-flex align="center" :size="8" :wrap="false">
        <n-input
          v-model:value="input"
          size="large"
          round
          clearable
          :placeholder="$t('browse.placeholder')"
          @keyup.enter="handleSearch"
        >
          <template #prefix>
            <n-icon><icon-mdi-magnify /></n-icon>
          </template>
        </n-input>
        <n-button type="primary" size="large" round :loading="loading" @click="handleSearch">
          {{ $t("browse.search") }}
        </n-button>
      </n-flex>

      <n-text depth="3" style="font-size: 12px">
        {{ $t("browse.hint") }}
      </n-text>

      <n-spin :show="loading" class="browse-spin">
        <template v-if="hasSearched">
          <n-empty
            v-if="!results?.entries.length"
            :description="$t('browse.empty')"
            class="browse-empty"
          />
          <template v-else>
            <n-flex align="center" :size="8" style="margin-bottom: 12px">
              <n-text depth="3" style="font-size: 13px">
                {{ results?.title || $t("browse.results") }}
              </n-text>
              <n-tag size="small" round :bordered="false" type="info">
                {{ results?.entries.length }}
              </n-tag>
              <n-flex :size="8" style="margin-left: auto">
                <n-button size="small" secondary @click="handleSelectAll">
                  {{ $t("common.selectAll") }}
                </n-button>
                <n-button size="small" secondary @click="handleClearSelection">
                  {{ $t("common.deselectAll") }}
                </n-button>
                <n-button
                  size="small"
                  type="primary"
                  :disabled="selectedUrls.size === 0"
                  :loading="addingAll"
                  @click="handleAddSelected"
                >
                  {{ $t("browse.addSelected", { n: selectedUrls.size }) }}
                </n-button>
              </n-flex>
            </n-flex>

            <div class="results-grid">
              <div
                v-for="entry in results?.entries"
                :key="entry.url"
                class="result-card"
                :class="{ selected: isSelected(entry) }"
                @click="handleOpen(entry)"
              >
                <div class="thumb-wrap">
                  <img
                    v-if="entry.thumbnail"
                    :src="entry.thumbnail"
                    class="thumb-img"
                    loading="lazy"
                    alt=""
                  />
                  <div v-else class="thumb-placeholder">
                    <icon-mdi-image-outline />
                  </div>
                  <div v-if="entry.duration != null" class="duration-badge">
                    {{ formatDuration(entry.duration, "0:00") }}
                  </div>
                  <div v-if="entry.isLive" class="live-badge">
                    <span class="live-dot" />{{ $t("detail.liveNow") }}
                  </div>
                  <n-checkbox
                    class="select-box"
                    :checked="isSelected(entry)"
                    @update:checked="toggleSelected(entry)"
                    @click.stop
                  />
                  <div v-if="openingUrl === entry.url" class="card-overlay">
                    <n-spin size="small" />
                  </div>
                </div>
                <div class="card-meta">
                  <n-ellipsis :line-clamp="2" class="card-title">{{ entry.title }}</n-ellipsis>
                  <n-flex align="center" :size="8" :wrap="false">
                    <n-text depth="3" style="font-size: 12px" class="meta-text">
                      {{ entry.uploader || $t("common.unknown") }}
                    </n-text>
                    <n-text v-if="entry.viewCount" depth="3" style="font-size: 12px" class="meta-text">
                      {{ formatViewCount(entry.viewCount) }}{{ $t("detail.views") }}
                    </n-text>
                  </n-flex>
                </div>
              </div>
            </div>
          </template>
        </template>
        <n-empty
          v-else
          :description="$t('browse.initialHint')"
          class="browse-empty"
        />
      </n-spin>
    </n-flex>
  </div>
</template>

<style scoped lang="scss">
.browse-page {
  height: 100%;
  overflow-y: auto;
  padding: 24px;

  .browse-container {
    max-width: 1200px;
    margin: 0 auto;
  }

  .browse-spin {
    min-height: 200px;
  }

  .browse-empty {
    margin-top: 80px;
  }

  .results-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }

  .result-card {
    cursor: pointer;
    border-radius: 8px;
    overflow: hidden;
    background: var(--n-color-modal);
    border: 2px solid transparent;
    transition:
      border-color 0.15s,
      box-shadow 0.15s;

    &:hover {
      box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
    }

    &.selected {
      border-color: var(--n-primary-color);
    }

    .thumb-wrap {
      position: relative;
      width: 100%;
      aspect-ratio: 16 / 9;
      background: rgba(127, 127, 127, 0.12);

      .thumb-img {
        width: 100%;
        height: 100%;
        object-fit: cover;
        display: block;
      }

      .thumb-placeholder {
        width: 100%;
        height: 100%;
        display: grid;
        place-items: center;
        font-size: 32px;
        opacity: 0.4;
      }

      .duration-badge {
        position: absolute;
        bottom: 6px;
        right: 6px;
        background: rgba(0, 0, 0, 0.7);
        color: #fff;
        font-size: 11px;
        padding: 1px 5px;
        border-radius: 3px;
        font-variant-numeric: tabular-nums;
      }

      .live-badge {
        position: absolute;
        bottom: 6px;
        left: 6px;
        background: rgba(220, 38, 38, 0.85);
        color: #fff;
        font-size: 11px;
        padding: 1px 6px;
        border-radius: 3px;
        display: flex;
        align-items: center;
        gap: 4px;
        font-weight: 600;
      }

      .live-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: #fff;
        animation: live-pulse 1.5s ease-in-out infinite;
      }

      @keyframes live-pulse {
        0%,
        100% {
          opacity: 1;
        }
        50% {
          opacity: 0.3;
        }
      }

      .select-box {
        position: absolute;
        top: 6px;
        left: 6px;
        background: rgba(255, 255, 255, 0.85);
        border-radius: 4px;
        padding: 2px;
      }

      .card-overlay {
        position: absolute;
        inset: 0;
        display: grid;
        place-items: center;
        background: rgba(0, 0, 0, 0.35);
      }
    }

    .card-meta {
      padding: 10px 12px 12px;

      .card-title {
        font-size: 13px;
        font-weight: 600;
        line-height: 1.4;
        margin-bottom: 6px;
      }

      .meta-text {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        min-width: 0;
      }
    }
  }
}
</style>
