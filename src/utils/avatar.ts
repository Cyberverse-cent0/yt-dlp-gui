/**
 * 用户头像占位工具：首字符 + 按名称哈希出的稳定底色
 */

/** 取名称首字符作为头像文字 */
export const avatarInitial = (name: string): string =>
  name.trim().charAt(0).toUpperCase() || "?";

/** 按名称哈希生成稳定的 HSL 颜色，同一用户始终同色 */
export const avatarColor = (name: string): string => {
  let hash = 0;
  for (let i = 0; i < name.length; i++) hash = (hash * 31 + name.charCodeAt(i)) >>> 0;
  return `hsl(${hash % 360} 55% 45%)`;
};
