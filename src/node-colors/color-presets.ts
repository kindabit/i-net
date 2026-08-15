/**
 * 数据节点与画布节点的预设颜色组合。
 *
 * 预设只定义自己需要的属性（目前仅 background），其余键缺失即默认值。
 * 色值字面量必须遵守存储格式约定：小写 #rrggbbaa，
 * 与 parseVuetifyColor 的输出格式保持一致，否则历史计数会被打破。
 */

import type {
  DataNodeColorScheme,
  CanvasNodeColorScheme,
} from "./index";

/** 数据节点预设 */
export interface DataNodeColorPreset {
  /** 预设名称的 i18n key（如 "database.color-dialog.preset-blue"） */
  nameKey: string;
  scheme: DataNodeColorScheme;
}

/** 画布节点预设 */
export interface CanvasNodeColorPreset {
  /** 预设名称的 i18n key（如 "database.color-dialog.preset-blue"） */
  nameKey: string;
  scheme: CanvasNodeColorScheme;
}

/** 数据节点预设列表（蓝/绿/紫/橙/青/粉/灰） */
export const DATA_NODE_COLOR_PRESETS: DataNodeColorPreset[] = [
  {
    nameKey: "database.color-dialog.preset-blue",
    scheme: { light: { background: "#e3f2fdff" }, dark: { background: "#1e3a5fff" } },
  },
  {
    nameKey: "database.color-dialog.preset-green",
    scheme: { light: { background: "#e8f5e9ff" }, dark: { background: "#1b5e20ff" } },
  },
  {
    nameKey: "database.color-dialog.preset-purple",
    scheme: { light: { background: "#f3e5f5ff" }, dark: { background: "#4a148cff" } },
  },
  {
    nameKey: "database.color-dialog.preset-orange",
    scheme: { light: { background: "#fff3e0ff" }, dark: { background: "#bf360cff" } },
  },
  {
    nameKey: "database.color-dialog.preset-teal",
    scheme: { light: { background: "#e0f2f1ff" }, dark: { background: "#004d40ff" } },
  },
  {
    nameKey: "database.color-dialog.preset-pink",
    scheme: { light: { background: "#fce4ecff" }, dark: { background: "#880e4fff" } },
  },
  {
    nameKey: "database.color-dialog.preset-grey",
    scheme: { light: { background: "#f5f5f5ff" }, dark: { background: "#424242ff" } },
  },
];

/** 画布节点预设列表（蓝/绿/紫/橙/青/粉/灰） */
export const CANVAS_NODE_COLOR_PRESETS: CanvasNodeColorPreset[] = [
  {
    nameKey: "database.color-dialog.preset-blue",
    scheme: { light: { background: "#e3f2fdff" }, dark: { background: "#1e3a5fff" } },
  },
  {
    nameKey: "database.color-dialog.preset-green",
    scheme: { light: { background: "#e8f5e9ff" }, dark: { background: "#1b5e20ff" } },
  },
  {
    nameKey: "database.color-dialog.preset-purple",
    scheme: { light: { background: "#f3e5f5ff" }, dark: { background: "#4a148cff" } },
  },
  {
    nameKey: "database.color-dialog.preset-orange",
    scheme: { light: { background: "#fff3e0ff" }, dark: { background: "#bf360cff" } },
  },
  {
    nameKey: "database.color-dialog.preset-teal",
    scheme: { light: { background: "#e0f2f1ff" }, dark: { background: "#004d40ff" } },
  },
  {
    nameKey: "database.color-dialog.preset-pink",
    scheme: { light: { background: "#fce4ecff" }, dark: { background: "#880e4fff" } },
  },
  {
    nameKey: "database.color-dialog.preset-grey",
    scheme: { light: { background: "#f5f5f5ff" }, dark: { background: "#424242ff" } },
  },
];
