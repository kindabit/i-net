/**
 * 自定义主题库持久化。
 *
 * 存储于后端 preference 数据库：preference 表中键为 "customThemes" 的一行，
 * 值为全部自定义主题定义组成的 JSON 数组。
 */
import { loadPreference, storePreference } from "@/preferences";
import { isAppThemeDefinition } from "./types";
import type { AppThemeDefinition } from "./types";

/** 自定义主题库在 preference 表中的键名（值为 JSON 数组） */
const CUSTOM_THEMES_PREFERENCE_NAME = "customThemes";

/**
 * 加载自定义主题（读取失败或存储损坏时返回空列表）。
 * @returns 自定义主题列表
 */
export async function loadCustomThemes(): Promise<AppThemeDefinition[]> {
  const raw = await loadPreference(CUSTOM_THEMES_PREFERENCE_NAME);
  if (raw === null) return [];
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isAppThemeDefinition);
  } catch {
    return [];
  }
}

/**
 * 保存自定义主题并落盘。
 * @param themes 自定义主题列表
 */
export async function saveCustomThemes(
  themes: AppThemeDefinition[],
): Promise<void> {
  await storePreference(
    CUSTOM_THEMES_PREFERENCE_NAME,
    JSON.stringify(themes),
  );
}
