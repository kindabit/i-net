/**
 * 内置主题定义。
 *
 * 新增内置主题只需在数组中追加一个 AppThemeDefinition。
 */
import type { AppThemeDefinition } from "../types";

/** 默认主题名（无偏好时使用） */
export const DEFAULT_THEME_NAME = "light";

/** 全部内置主题 */
export const builtinThemes: AppThemeDefinition[] = [
  {
    name: "light",
    displayName: "Light",
    dark: false,
    colors: {
      background: "#FFFFFF",
      surface: "#FFFFFF",
      primary: "#1976D2",
      secondary: "#424242",
      success: "#4CAF50",
      warning: "#FB8C00",
      error: "#F44336",
      info: "#2196F3",
    },
  },
  {
    name: "dark",
    displayName: "Dark",
    dark: true,
    colors: {
      background: "#121212",
      surface: "#212121",
      primary: "#BB86FC",
      secondary: "#03DAC5",
      success: "#4CAF50",
      warning: "#FB8C00",
      error: "#CF6679",
      info: "#2196F3",
    },
  },
  {
    name: "ocean",
    displayName: "Ocean",
    dark: false,
    colors: {
      background: "#F0F6FA",
      surface: "#FFFFFF",
      primary: "#0277BD",
      secondary: "#00838F",
      success: "#2E7D32",
      warning: "#F9A825",
      error: "#C62828",
      info: "#29B6F6",
    },
  },
  {
    name: "forest",
    displayName: "Forest",
    dark: true,
    colors: {
      background: "#121A14",
      surface: "#1E2B21",
      primary: "#66BB6A",
      secondary: "#26A69A",
      success: "#81C784",
      warning: "#FFB74D",
      error: "#E57373",
      info: "#4FC3F7",
    },
  },
];

/** 内置主题名集合（自定义主题禁止占用，亦不可移除） */
export const builtinThemeNames: ReadonlySet<string> = new Set(
  builtinThemes.map((t) => t.name),
);
