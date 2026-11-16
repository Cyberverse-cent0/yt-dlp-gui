<script setup lang="ts">
import { isValidUrl } from "@/utils/validate";
import { formatDuration } from "@/utils/format";
import { useVideoStore } from "@/stores/video";
import { usePendingStore } from "@/stores/pending";
import { goToolList } from "@/utils/toolbox";
import { useI18n } from "vue-i18n";
import type { FetchedVideoData } from "@/types";

useI18n();
const router = useRouter();
const videoStore = useVideoStore();
const pendingStore = usePendingStore();

const goBack = () => goToolList(router);

const url = ref("");
const data = ref<FetchedVideoData | null>(null);
/** 勾选的播放列表序号（1 起始，与 --playlist-items 对齐） */
const selectedItems = ref<number[]>([]);

const urlValid = computed(() => isValidUrl(url.value.trim()));
const isPlaylist = computed(() => data.value?.isPlaylist === true);
const entries = computed(() => data.value?.playlistEntries ?? []);

const handleFetch = async () => {
  const result = await videoStore.fetchVideoInfo(url.value.trim());
  if (!result) return;
  data.value = result;
  selectedItems.value = result.isPlaylist
    ? result.playlistEntries.map((_, i) => i + 1)
    : [];
};

const handleSelectAll = () => {
  selectedItems.value = entries.value.map((_, i) => i + 1);
};

const handleDeselectAll = () => {
  selectedItems.value = [];
};

/** 把勾选的条目作为一个待下载项送入 Pending 页，由用户确认格式后开始下载 */
const handleSendToPending = () => {
  const current = data.value;
  if (!current) return;
  const id = pendingStore.add(current);
  const item = pendingStore.items.find((i) => i.id === id);
  if (item && current.isPlaylist) {
    item.selectedPlaylistItems = [...selectedItems.value].sort((a, b) => a - b);
  }
  void router.push({ name: "pending" });
};
</script>

<template>
  <n-flex vertical :size="12">
    <n-flex align="center" :size="8">
      <n-button strong secondary size="small" @click="goBack">
        <template #icon>
          <n-icon><icon-mdi-arrow-left /></n-icon>
        </template>
        {{ $t("common.back") }}
      </n-button>
      <n-text strong style="font-size: 15px">{{ $t("toolbox.playlistTitle") }}</n-text>
    </n-flex>

    <n-card size="small">
      <n-flex vertical :size="12">
        <n-text depth="3" style="font-size: 13px">
          {{ $t("toolbox.playlistPageDesc") }}
        </n-text>
        <ToolUrlInput v-model="url" @submit="handleFetch" />
        <n-button
          type="primary"
          :loading="videoStore.fetching"
          :disabled="!urlValid"
          @click="handleFetch"
        >
          <template #icon>
            <n-icon><icon-mdi-playlist-play /></n-icon>
          </template>
          {{ $t("toolbox.fetchPlaylist") }}
        </n-button>
      </n-flex>
    </n-card>

    <n-card v-if="data" size="small">
      <template #header>
        <n-flex align="center" :size="8">
          <n-icon size="16"><icon-mdi-playlist-play /></n-icon>
          <span>{{ data.videoInfo.title || $t("toolbox.playlistTitle") }}</span>
          <n-tag v-if="isPlaylist" size="small" round :bordered="false" type="info">
            {{ $t("toolbox.playlistVideoCount", { n: entries.length }) }}
          </n-tag>
        </n-flex>
      </template>
      <n-flex vertical :size="10">
        <n-text v-if="!isPlaylist" depth="3" style="font-size: 13px">
          {{ $t("toolbox.notAPlaylist") }}
        </n-text>
        <template v-else>
          <n-flex align="center" :size="8">
            <n-text depth="3" style="font-size: 13px">
              {{ $t("toolbox.selectedCount", { n: selectedItems.length }) }}
            </n-text>
            <n-flex :size="8" style="margin-left: auto">
              <n-button size="tiny" secondary @click="handleSelectAll">
                {{ $t("common.selectAll") }}
              </n-button>
              <n-button size="tiny" secondary @click="handleDeselectAll">
                {{ $t("common.deselectAll") }}
              </n-button>
            </n-flex>
          </n-flex>
          <n-checkbox-group v-model:value="selectedItems">
            <n-flex vertical :size="6">
              <n-checkbox
                v-for="(entry, index) in entries"
                :key="entry.id || index"
                :value="index + 1"
              >
                <n-flex align="center" :size="8" :wrap="false">
                  <n-text style="font-size: 13px">
                    P{{ index + 1 }} {{ entry.title }}
                  </n-text>
                  <n-text
                    v-if="entry.duration != null"
                    depth="3"
                    style="font-size: 12px; flex-shrink: 0"
                  >
                    {{ formatDuration(entry.duration, "0:00") }}
                  </n-text>
                </n-flex>
              </n-checkbox>
            </n-flex>
          </n-checkbox-group>
        </template>
        <n-button
          type="primary"
          :disabled="isPlaylist && selectedItems.length === 0"
          @click="handleSendToPending"
        >
          <template #icon>
            <n-icon><icon-mdi-download /></n-icon>
          </template>
          {{ $t("toolbox.sendToPending") }}
        </n-button>
      </n-flex>
    </n-card>
  </n-flex>
</template>
