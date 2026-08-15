/**
 * Vuetify VColorPicker 输出值的处理模块。
 *
 * VColorPicker 的输出有多种形态（hex 字符串、rgb/rgba 字符串、{r, g, b, a?} 对象），
 * parseVuetifyColor 将其统一正规化为小写 #rrggbbaa（输出无透明度时视为不透明，补 ff）。
 * 无法解析时返回空字符串。
 */

/** 匹配 #RGB / #RGBA / #RRGGBB / #RRGGBBAA */
const HEX_REGEX =
  /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/;

/** 匹配 rgb(...) / rgba(...) */
const RGB_REGEX =
  /^rgba?\(\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^,)]+)(?:\s*,\s*([^)]+))?\s*\)$/;

/**
 * 将短格式 hex（#RGB / #RGBA）展开为长格式。
 * @param value 去掉 # 后的 3 或 4 位 hex 字符串
 * @returns 展开后的 6 或 8 位 hex 字符串
 */
function expandShortHex(value: string): string {
  return value
    .split("")
    .map((c) => c + c)
    .join("");
}

/**
 * 规范化 6 位或 8 位 hex：固定输出 8 位（6 位视为不透明，补 ff），结果转为小写。
 * @param hex 6 或 8 位 hex 字符串（不含 #）
 * @returns 规范化后的 8 位 hex 字符串（含 #，小写）
 */
function normalizeHexExpanded(hex: string): string {
  if (hex.length === 6) {
    return "#" + hex.toLowerCase() + "ff";
  }
  return "#" + hex.toLowerCase();
}

/**
 * 规范化 hex 字符串（#RGB / #RGBA / #RRGGBB / #RRGGBBAA）。
 * @param hex hex 字符串
 * @returns 规范化后的小写 #rrggbbaa；无法解析返回 ""
 */
function normalizeHex(hex: string): string {
  const match = hex.match(HEX_REGEX);
  if (!match) return "";
  const value = match[1];
  if (value.length <= 4) {
    return normalizeHexExpanded(expandShortHex(value));
  }
  return normalizeHexExpanded(value);
}

/**
 * 将 0-255 的整数转为 2 位小写 hex。
 * @param n 0-255 的整数
 * @returns 2 位小写 hex
 */
function toHex2(n: number): string {
  return Math.max(0, Math.min(255, Math.round(n)))
    .toString(16)
    .padStart(2, "0");
}

/**
 * 规范化 rgb/rgba 字符串。
 * @param rgb rgb(...) 或 rgba(...) 字符串
 * @returns 规范化后的小写 #rrggbbaa；无法解析返回 ""
 */
function normalizeRgb(rgb: string): string {
  const match = rgb.match(RGB_REGEX);
  if (!match) return "";
  const r = parseInt(match[1].trim(), 10);
  const g = parseInt(match[2].trim(), 10);
  const b = parseInt(match[3].trim(), 10);
  const aStr = match[4];
  if (isNaN(r) || isNaN(g) || isNaN(b)) return "";
  if (aStr === undefined) {
    return `#${toHex2(r)}${toHex2(g)}${toHex2(b)}ff`;
  }
  const a = parseFloat(aStr.trim());
  if (isNaN(a)) return "";
  if (a >= 1) return `#${toHex2(r)}${toHex2(g)}${toHex2(b)}ff`;
  return `#${toHex2(r)}${toHex2(g)}${toHex2(b)}${toHex2(a * 255)}`;
}

/**
 * 规范化 {r, g, b, a?} 对象（r/g/b 为 0-255，a 为 0-1 或 0-255）。
 * @param obj 颜色对象
 * @returns 规范化后的小写 #rrggbbaa；无法解析返回 ""
 */
function normalizeColorObject(obj: Record<string, unknown>): string {
  const r = obj.r;
  const g = obj.g;
  const b = obj.b;
  if (
    typeof r !== "number" ||
    typeof g !== "number" ||
    typeof b !== "number" ||
    isNaN(r) ||
    isNaN(g) ||
    isNaN(b)
  ) {
    return "";
  }
  const a = obj.a;
  if (a === undefined || a === null) {
    return `#${toHex2(r)}${toHex2(g)}${toHex2(b)}ff`;
  }
  if (typeof a !== "number" || isNaN(a)) return "";
  let alpha255: number;
  if (a <= 1) {
    alpha255 = Math.round(a * 255);
  } else {
    alpha255 = Math.round(a);
  }
  if (alpha255 >= 255) return `#${toHex2(r)}${toHex2(g)}${toHex2(b)}ff`;
  return `#${toHex2(r)}${toHex2(g)}${toHex2(b)}${toHex2(alpha255)}`;
}

/**
 * 将 VColorPicker 的各种输出正规化为小写 #rrggbbaa。
 *
 * 支持 hex 字符串、rgb/rgba 字符串、{r, g, b, a?} 对象；
 * 无透明度一律视为不透明（alpha 段补 ff）。
 * @param color VColorPicker 输出值
 * @returns 正规化后的小写 #rrggbbaa 字符串，或 ""（无法解析）
 */
export function parseVuetifyColor(color: unknown): string {
  if (color === null || color === undefined) return "";
  if (typeof color === "string") {
    const str = color.trim();
    if (!str) return "";
    if (str[0] === "#") return normalizeHex(str);
    if (str.toLowerCase().startsWith("rgb")) return normalizeRgb(str);
    return "";
  }
  if (typeof color === "object") {
    return normalizeColorObject(color as Record<string, unknown>);
  }
  return "";
}
