/**
 * YouTube URL 识别与播放器嵌入地址工具
 */

const YOUTUBE_HOST_PATTERN = /(^|\.)(youtube\.com|youtu\.be|youtube-nocookie\.com)$/i;

/** 判断 URL 是否指向 YouTube（watch、shorts、live、频道等页面均算） */
export const isYouTubeUrl = (targetUrl: string): boolean => {
  try {
    return YOUTUBE_HOST_PATTERN.test(new URL(targetUrl).hostname);
  } catch {
    return /youtube\.com|youtu\.be/i.test(targetUrl);
  }
};

/**
 * 从 YouTube URL 中提取视频 ID。
 * 支持 watch?v=、youtu.be 短链、/shorts/、/live/、/embed/、/v/ 等形式。
 *
 * @param targetUrl YouTube 视频页面 URL
 * @returns 视频 ID，无法识别时返回 null
 */
export const getYouTubeVideoId = (targetUrl: string): string | null => {
  try {
    const parsed = new URL(targetUrl);
    if (!YOUTUBE_HOST_PATTERN.test(parsed.hostname)) return null;

    const fromQuery = parsed.searchParams.get("v");
    if (fromQuery) return fromQuery;

    const matched = parsed.pathname.match(
      /^\/(?:shorts|live|embed|v|watch)\/([\w-]{6,})/,
    );
    if (matched) return matched[1];

    // youtu.be 短链：路径首段即视频 ID
    if (/(^|\.)youtu\.be$/i.test(parsed.hostname)) {
      const id = parsed.pathname.split("/").filter(Boolean)[0];
      if (id) return id;
    }
  } catch {
    // ignore
  }
  return null;
};

/**
 * 生成 youtube-nocookie 嵌入播放器地址（增强隐私、不携带 Cookie）
 *
 * @param videoId YouTube 视频 ID
 * @param autoplay 是否自动播放
 * @returns 可直接用于 iframe src 的嵌入地址
 */
export const getYouTubeEmbedUrl = (videoId: string, autoplay = true): string => {
  const params = new URLSearchParams({ rel: "0" });
  if (autoplay) params.set("autoplay", "1");
  return `https://www.youtube-nocookie.com/embed/${videoId}?${params.toString()}`;
};
