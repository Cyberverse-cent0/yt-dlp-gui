<script setup lang="ts">
import { formatRelativeTime, formatViewCount } from "@/utils/format";
import { avatarColor, avatarInitial } from "@/utils/avatar";
import type { CommentNode } from "@/types";

const props = defineProps<{
  comment: CommentNode;
  /** 评论者名称（用于回复缩进中的“回复 @xxx”展示） */
  depth?: number;
}>();

const repliesExpanded = ref(false);

const hasReplies = computed(() => props.comment.replies.length > 0);
</script>

<template>
  <div class="comment-item" :style="{ marginLeft: `${(depth ?? 0) * 28}px` }">
    <div class="comment-avatar" :style="{ background: avatarColor(comment.author || '?') }">
      {{ avatarInitial(comment.author || "?") }}
    </div>
    <div class="comment-body">
      <div class="comment-head">
        <n-text class="comment-author">{{ comment.author || $t("common.unknown") }}</n-text>
        <n-tag
          v-if="comment.author_is_uploader"
          size="tiny"
          round
          :bordered="false"
          type="info"
          class="uploader-tag"
        >
          {{ $t("watch.channelOwner") }}
        </n-tag>
        <n-text depth="3" class="comment-time">
          {{ formatRelativeTime(comment.timestamp) }}
        </n-text>
      </div>
      <div class="comment-text">{{ comment.text }}</div>
      <n-flex align="center" :size="10" class="comment-actions">
        <n-flex align="center" :size="4" class="comment-like">
          <n-icon size="14"><icon-mdi-thumb-up-outline /></n-icon>
          <n-text v-if="comment.like_count > 0" depth="3" style="font-size: 12px">
            {{ formatViewCount(comment.like_count) }}
          </n-text>
        </n-flex>
        <n-button
          v-if="hasReplies"
          size="tiny"
          text
          type="primary"
          @click="repliesExpanded = !repliesExpanded"
        >
          {{
            repliesExpanded
              ? $t("watch.hideReplies")
              : $t("watch.viewReplies", { n: comment.replies.length })
          }}
        </n-button>
      </n-flex>
      <template v-if="repliesExpanded">
        <CommentItem
          v-for="reply in comment.replies"
          :key="reply.id || reply.timestamp"
          :comment="reply"
          :depth="(depth ?? 0) + 1"
        />
      </template>
    </div>
  </div>
</template>

<style scoped lang="scss">
.comment-item {
  display: flex;
  gap: 12px;
  padding: 10px 0;

  .comment-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    font-size: 15px;
    font-weight: 700;
    flex-shrink: 0;
    user-select: none;
  }

  .comment-body {
    flex: 1;
    min-width: 0;
  }

  .comment-head {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;

    .comment-author {
      font-size: 13px;
      font-weight: 600;
    }

    .comment-time {
      font-size: 12px;
    }
  }

  .comment-text {
    margin-top: 2px;
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .comment-actions {
    margin-top: 6px;
  }

  .comment-like {
    color: var(--n-text-color-3);
  }
}
</style>
