/**
 * 主题基座。
 *
 * 以 Vuetify 主题表（vuetify.theme.themes）为唯一数据源，本模块只是薄封装：
 * 自定义主题在运行时注入同一张表，切换即 vuetify.theme.change()。
 * 自定义主题通过 saveCustomTheme 注册、removeCustomTheme 移除、
 * exportTheme / importTheme 分享。
 *
 * 样式消费方式（Vuetify 组件自动生效，自定义组件任选）：
 * - CSS：:root 上的 --v-theme-* 变量，如 color: rgb(var(--v-theme-primary))
 * - 工具类：bg-* / text-* / border-*
 * - JS：vuetify.theme.current.value.colors（响应式，画布等场景）
 */
import { computed } from "vue";
import { vuetify } from "@/vuetify";
import { t, te } from "@/i18n";
import { snackbarText } from "@/composables/use-snackbar";
import { loadPreference, storePreference } from "@/preferences";
import { builtinThemeNames, DEFAULT_THEME_NAME } from "./builtin";
import { isAppThemeDefinition, themeValidationErrors } from "./types";
import type { AppThemeDefinition } from "./types";
import { loadCustomThemes, saveCustomThemes } from "./storage";

const theme = vuetify.theme;

/** 主题偏好名称在 preference 数据库中的键名 */
const THEME_NAME_PREFERENCE_KEY = "themeName";

/**
 * 注册主题（写入 Vuetify 主题表，同名覆盖）。仅模块内部使用。
 * @param def 主题定义
 */
function registerTheme(def: AppThemeDefinition): void {
  (theme.themes.value as Record<string, unknown>)[def.name] = def;
}

/** 持久化当前全部自定义主题 */
async function persistCustomThemes(): Promise<void> {
  const customs = Object.entries(theme.themes.value)
    .filter(([name]) => !builtinThemeNames.has(name))
    .map(([, def]) => def as unknown as AppThemeDefinition);
  await saveCustomThemes(customs);
}


/** 当前主题名（响应式） */
export const currentThemeName = theme.global.name;

/** 当前主题是否为暗色（基于 currentThemeName 计算，主题切换时自动更新） */
export const currentThemeIsDark = computed(() => {
  const def = theme.themes.value[currentThemeName.value] as unknown as
    | AppThemeDefinition
    | undefined;
  return def?.dark ?? false;
});

/** 主题列表（name + displayName + builtin，供主题切换器与管理界面展示） */
export const themeList = computed(() =>
  Object.entries(theme.themes.value).map(([name, def]) => {
    const builtin = builtinThemeNames.has(name);
    const fallback = (def as unknown as AppThemeDefinition).displayName;
    // 内置主题的显示名称取自国际化文本，缺失时回退到定义中的 displayName
    const i18nKey = `themes.builtin.${name}`;
    return {
      name,
      builtin,
      displayName:
        builtin && te(i18nKey) ? t(i18nKey) : fallback,
    };
  }),
);

/**
 * 初始化主题基座：加载自定义主题、应用持久化的偏好主题（挂载前调用）。
 */
export async function initThemes(): Promise<void> {
  for (const def of await loadCustomThemes()) {
    if (!builtinThemeNames.has(def.name)) {
      registerTheme(def);
    }
  }
  const saved = await loadPreference(THEME_NAME_PREFERENCE_KEY);
  if (saved) {
    setTheme(saved, false);
  }
}

/**
 * 切换当前主题。
 * @param name 主题名
 * @param persistent 是否持久化偏好（默认 true；挂载时由持久化数据还原应传 false）
 * @returns 主题是否存在
 */
export function setTheme(name: string, persistent = true): boolean {
  if (!(name in theme.themes.value)) return false;
  theme.change(name);
  if (persistent) {
    void storePreference(THEME_NAME_PREFERENCE_KEY, name);
  }
  return true;
}

/**
 * 判断主题是否存在。
 * @param name 主题名
 * @returns 主题是否存在
 */
export function hasTheme(name: string): boolean {
  return name in theme.themes.value;
}

/**
 * 获取主题定义的深拷贝（用于编辑预填，返回值与主题表无引用关系）。
 * @param name 主题名
 * @returns 主题定义；主题不存在时返回 null
 */
export function getThemeDefinition(name: string): AppThemeDefinition | null {
  const def = theme.themes.value[name];
  if (!def) return null;
  // 主题表中的对象是 Vue 响应式代理，structuredClone 无法克隆；
  // 主题定义是经 JSON Schema 校验的纯 JSON 数据，通过 JSON 往返完成深拷贝
  return JSON.parse(JSON.stringify(def)) as AppThemeDefinition;
}

/**
 * 保存自定义主题（新增或同名覆盖：校验 + 注册 + 持久化）。
 *
 * 校验失败属于用户操作错误，通过 snackbar 告知用户（i18n 消息）并返回 false。
 * @param def 主题定义
 * @returns 是否保存成功
 */
export function saveCustomTheme(def: AppThemeDefinition): boolean {
  if (!isAppThemeDefinition(def)) {
    snackbarText(
      t("themes.invalid-format", {
        detail: themeValidationErrors(),
      }),
      "error",
    );
    return false;
  }
  if (builtinThemeNames.has(def.name)) {
    snackbarText(
      t("themes.name-reserved", { name: def.name }),
      "error",
    );
    return false;
  }
  registerTheme(def);
  void persistCustomThemes();
  return true;
}

/**
 * 移除自定义主题。
 *
 * 被移除的恰好是当前主题时回退到默认主题——这也属于一次主题变动，
 * 同步保存主题偏好。
 * @param name 主题名
 * @returns 是否移除成功（内置主题或不存在的主题返回 false）
 */
export function removeCustomTheme(name: string): boolean {
  if (builtinThemeNames.has(name) || !(name in theme.themes.value)) {
    return false;
  }
  delete theme.themes.value[name];
  if (currentThemeName.value === name) {
    setTheme(DEFAULT_THEME_NAME);
  }
  void persistCustomThemes();
  return true;
}

/**
 * 导出主题为 JSON 字符串（用于分享）。
 * @param name 主题名
 * @returns JSON 字符串；主题不存在时返回 null
 */
export function exportTheme(name: string): string | null {
  if (!(name in theme.themes.value)) return null;
  return JSON.stringify(theme.themes.value[name], null, 2);
}

/**
 * 从 JSON 字符串导入主题（解析 + 校验 + 注册 + 持久化）。
 *
 * 数据格式错误与名称冲突属于用户操作错误，
 * 通过 snackbar 告知用户（i18n 消息）并返回 false，不抛出异常。
 * @param json 主题 JSON 字符串
 * @returns 是否导入成功
 */
export function importTheme(json: string): boolean {
  /** 通过 snackbar 告知用户主题导入失败（i18n 消息） */
  function notifyImportFailure(
    key: "import-invalid-format" | "import-name-reserved",
    params: Record<string, string>,
  ): void {
    snackbarText(t(`themes.${key}`, params), "error");
  }

  let data: unknown;
  try {
    data = JSON.parse(json);
  } catch (error) {
    notifyImportFailure("import-invalid-format", {
      detail: error instanceof Error ? error.message : String(error),
    });
    return false;
  }
  if (!isAppThemeDefinition(data)) {
    notifyImportFailure("import-invalid-format", {
      detail: themeValidationErrors(),
    });
    return false;
  }
  if (builtinThemeNames.has(data.name)) {
    notifyImportFailure("import-name-reserved", { name: data.name });
    return false;
  }
  registerTheme(data);
  void persistCustomThemes();
  return true;
}
