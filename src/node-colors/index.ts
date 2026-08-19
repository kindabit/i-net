/**
 * 节点自定义颜色的纯逻辑契约层。
 *
 * 存储格式：后端 Node/Canvas 实体的 color 字段。空串表示全部使用默认值；
 * 非空时为 JSON 对象 {"light": {...}, "dark": {...}}，每个主题对象只含有自定义值的键，
 * 缺失键表示该属性使用默认值。色值一律由 parseVuetifyColor 产生，为小写 #rrggbbaa。
 *
 * 数据节点（DataNode* 类型）与画布节点（CanvasNode* 类型）是两套独立的概念，
 * 即使函数实现相似也不做合并。反序列化不做色值正规化（色值只能由
 * parseVuetifyColor 或本模块预设字面量产生，格式已保证闭合）。
 */

import { isObjectLike, isString } from "lodash";
import { t } from "@/i18n";
import {
  userDatabaseNodeColorList,
  userDatabaseCanvasColorList,
} from "@/api";

/** 数据节点的可自定义颜色属性：键存在即自定义，缺失即使用默认值 */
export type DataNodeColorProperties = {
  icon?: string;
  title?: string;
  subtitle?: string;
  background?: string;
  borderUnselected?: string;
  borderSelected?: string;
  handle?: string;
  action?: string;
};

/** 数据节点亮色/暗色双主题颜色方案 */
export type DataNodeColorScheme = {
  light: DataNodeColorProperties;
  dark: DataNodeColorProperties;
};

/** 数据节点历史颜色组合 */
export type DataNodeHistoryColor = {
  /** 使用该组合的节点名称（去重，最多 3 个，一个一行） */
  description: string;
  scheme: DataNodeColorScheme;
};

/** 画布节点的可自定义颜色属性：键存在即自定义，缺失即使用默认值 */
export type CanvasNodeColorProperties = {
  icon?: string;
  title?: string;
  background?: string;
  borderUnselected?: string;
  borderSelected?: string;
  action?: string;
};

/** 画布节点亮色/暗色双主题颜色方案 */
export type CanvasNodeColorScheme = {
  light: CanvasNodeColorProperties;
  dark: CanvasNodeColorProperties;
};

/** 画布节点历史颜色组合 */
export type CanvasNodeHistoryColor = {
  /** 使用该组合的画布名称（去重，最多 3 个，一个一行） */
  description: string;
  scheme: CanvasNodeColorScheme;
};

/** 数据节点颜色键的固定顺序（deserialize 遍历与 normalize 序列化共用） */
const DATA_NODE_COLOR_KEYS = [
  "icon",
  "title",
  "subtitle",
  "background",
  "borderUnselected",
  "borderSelected",
  "handle",
  "action",
] as const satisfies readonly (keyof DataNodeColorProperties)[];

/** 画布节点颜色键的固定顺序（deserialize 遍历与 normalize 序列化共用） */
const CANVAS_NODE_COLOR_KEYS = [
  "icon",
  "title",
  "background",
  "borderUnselected",
  "borderSelected",
  "action",
] as const satisfies readonly (keyof CanvasNodeColorProperties)[];

/**
 * 解析 node 实体的 color 字段为颜色方案（node 侧唯一反序列化点）。
 *
 * 空串或 JSON 解析失败返回 { light: {}, dark: {} }；只读取已知键，
 * 值必须是非空字符串，否则忽略；light/dark 缺失或非对象按空主题处理。
 * @param color 后端 color 字段原值
 * @returns 颜色方案（键缺失即默认值）
 */
export function deserializeNodeColor(color: string): DataNodeColorScheme {
  const scheme: DataNodeColorScheme = { light: {}, dark: {} };
  if (!color.trim()) return scheme;
  let parsed: unknown;
  try {
    parsed = JSON.parse(color);
  } catch {
    return scheme;
  }
  for (const theme of ["light", "dark"] as const) {
    const themeObj = (parsed as Record<string, unknown> | null)?.[theme];
    if (!isObjectLike(themeObj)) continue;
    for (const key of DATA_NODE_COLOR_KEYS) {
      const value = (themeObj as Record<string, unknown>)[key];
      if (isString(value) && value) scheme[theme][key] = value;
    }
  }
  return scheme;
}

/**
 * 解析 canvas 实体的 color 字段为颜色方案（canvas 侧唯一反序列化点）。
 * 规则与 deserializeNodeColor 相同。
 * @param color 后端 color 字段原值
 * @returns 颜色方案（键缺失即默认值）
 */
export function deserializeCanvasColor(color: string): CanvasNodeColorScheme {
  const scheme: CanvasNodeColorScheme = { light: {}, dark: {} };
  if (!color.trim()) return scheme;
  let parsed: unknown;
  try {
    parsed = JSON.parse(color);
  } catch {
    return scheme;
  }
  for (const theme of ["light", "dark"] as const) {
    const themeObj = (parsed as Record<string, unknown> | null)?.[theme];
    if (!isObjectLike(themeObj)) continue;
    for (const key of CANVAS_NODE_COLOR_KEYS) {
      const value = (themeObj as Record<string, unknown>)[key];
      if (isString(value) && value) scheme[theme][key] = value;
    }
  }
  return scheme;
}

/**
 * 判断属性对象是否存在任意已定义的键。
 * @param props 颜色属性对象
 * @returns 存在至少一个非 undefined 的键时返回 true
 */
function hasAnyProperty(props: Record<string, string | undefined>): boolean {
  return Object.values(props).some((value) => value !== undefined);
}

/**
 * 序列化 node 颜色方案为存储字符串（node 侧唯一序列化点）。
 * light 与 dark 均无任何已定义键时返回空串（全部恢复默认）。
 * @param color 颜色方案
 * @returns 存储字符串
 */
export function serializeNodeColor(color: DataNodeColorScheme): string {
  if (!hasAnyProperty(color.light) && !hasAnyProperty(color.dark)) return "";
  return JSON.stringify(color);
}

/**
 * 序列化 canvas 颜色方案为存储字符串（canvas 侧唯一序列化点）。
 * 规则与 serializeNodeColor 相同。
 * @param color 颜色方案
 * @returns 存储字符串
 */
export function serializeCanvasColor(color: CanvasNodeColorScheme): string {
  if (!hasAnyProperty(color.light) && !hasAnyProperty(color.dark)) return "";
  return JSON.stringify(color);
}

/**
 * 正规化 node 颜色方案为唯一规范键：同一 scheme 无论键序如何都产出相同字符串。
 * @param scheme 颜色方案
 * @returns 规范键字符串
 */
function normalizeNodeColor(scheme: DataNodeColorScheme): string {
  return JSON.stringify({
    light: DATA_NODE_COLOR_KEYS.map((k) => [k, scheme.light[k] ?? null]),
    dark: DATA_NODE_COLOR_KEYS.map((k) => [k, scheme.dark[k] ?? null]),
  });
}

/**
 * 正规化 canvas 颜色方案为唯一规范键：同一 scheme 无论键序如何都产出相同字符串。
 * @param scheme 颜色方案
 * @returns 规范键字符串
 */
function normalizeCanvasColor(scheme: CanvasNodeColorScheme): string {
  return JSON.stringify({
    light: CANVAS_NODE_COLOR_KEYS.map((k) => [k, scheme.light[k] ?? null]),
    dark: CANVAS_NODE_COLOR_KEYS.map((k) => [k, scheme.dark[k] ?? null]),
  });
}

/** 历史聚合的中间分组 */
interface HistoryGroup<TScheme> {
  scheme: TScheme;
  /** 使用该组合的名称（去重） */
  names: string[];
  count: number;
}

/**
 * 将分组结果组装为历史颜色数组：按 count 降序（相同 count 保持首次出现顺序），
 * description 取去重后的前 3 个名称，一个一行。
 * @param groups 规范键 → 中间分组
 * @returns 历史颜色数组
 */
function buildHistoryList<TScheme>(
  groups: Map<string, HistoryGroup<TScheme>>,
): { description: string; scheme: TScheme }[] {
  return Array.from(groups.values())
    .sort((a, b) => b.count - a.count)
    .map((group) => ({
      description: group.names.slice(0, 3).join("\n"),
      scheme: group.scheme,
    }));
}

/**
 * 向分组表中计入一条记录。
 * @param groups 规范键 → 中间分组
 * @param key 规范键
 * @param scheme 该记录解析出的方案
 * @param name 节点/画布名称
 */
function addToGroups<TScheme>(
  groups: Map<string, HistoryGroup<TScheme>>,
  key: string,
  scheme: TScheme,
  name: string,
): void {
  const group = groups.get(key);
  if (group) {
    group.count++;
    if (!group.names.includes(name)) group.names.push(name);
  } else {
    groups.set(key, { scheme, names: [name], count: 1 });
  }
}

/**
 * 收集全部未删除 node 的颜色组合历史。
 *
 * 调 node_color_list 获取数据，过滤 color 为空的项，反序列化后经
 * normalizeNodeColor 分组计数，按使用次数从大到小排序返回。
 * @returns 数据节点历史颜色数组
 */
export async function collectNodeColorList(): Promise<DataNodeHistoryColor[]> {
  const entries = await userDatabaseNodeColorList();
  const groups = new Map<string, HistoryGroup<DataNodeColorScheme>>();
  for (const entry of entries) {
    if (!entry.color.trim()) continue;
    const scheme = deserializeNodeColor(entry.color);
    addToGroups(groups, normalizeNodeColor(scheme), scheme, entry.title);
  }
  return buildHistoryList(groups);
}

/**
 * 收集全部未删除 canvas 的颜色组合历史。
 * 规则与 collectNodeColorList 相同；根画布名称固定为本地化"根画布"文案。
 * @returns 画布节点历史颜色数组
 */
export async function collectCanvasColorList(): Promise<
  CanvasNodeHistoryColor[]
> {
  const entries = await userDatabaseCanvasColorList();
  const groups = new Map<string, HistoryGroup<CanvasNodeColorScheme>>();
  for (const entry of entries) {
    if (!entry.color.trim()) continue;
    const scheme = deserializeCanvasColor(entry.color);
    const name =
      entry.parent_id === null ? t("database.canvas.root-canvas") : entry.name;
    addToGroups(groups, normalizeCanvasColor(scheme), scheme, name);
  }
  return buildHistoryList(groups);
}
