/**
 * 全局国际化模块：应用内一切国际化需求的唯一入口。
 *
 * 语言文本由 @intlify/unplugin-vue-i18n 自动收集 src/i18n/<模块>/<locale>.json
 * 并按 locale 深合并（文件名即 locale 键），通过虚拟模块
 * @intlify/unplugin-vue-i18n/messages 一次性导入。
 *
 * 约定：每个模块文件的内容必须挂在与模块同名的唯一顶级键下
 * （如 common/zh-CN.json → { "common": { ... } }），避免合并时键名冲突。
 *
 * 注意：消息经插件 AOT 预编译，叶子节点是 AST 对象而非字符串，
 * 因此不能从 messages 中直接读取具体文案，须通过 i18n 实例解析（te / t）。
 *
 * 边界：vue-i18n 仅是本模块的内部实现，应用代码不得直接 import vue-i18n。
 * 全部对外能力（t / te / d 不依赖组件上下文，setup、模板、普通模块中均可
 * 直接使用；模板中调用会自动追踪当前语言，随语言切换重渲染）：
 * - mountI18n：加载持久化语言偏好并挂载 i18n 至 Vue 应用（异步，应用启动时调用一次）
 * - t / te / d：文案翻译 / 文案键存在性检查 / 日期时间格式化
 * - currentLocale：当前语言（响应式、只读）
 * - setLocale：设置当前语言（唯一修改入口，候选语言代码经内部匹配后生效）
 * - supportedLocales：支持的语言列表（只读）
 */
import { computed } from "vue";
import type { App } from "vue";
import { createI18n } from "vue-i18n";
import messages from "@intlify/unplugin-vue-i18n/messages";
import { loadPreference, storePreference } from "@/preferences";
import { setVuetifyLocale } from "@/vuetify";

/** 回退语言 */
const DEFAULT_LOCALE = "en-US";

/** "short" 日期时间格式的字段选项（不含秒，与此前 toLocaleString 的精度一致） */
const SHORT_DATETIME_FORMAT: Intl.DateTimeFormatOptions = {
  year: "numeric",
  month: "numeric",
  day: "numeric",
  hour: "numeric",
  minute: "numeric",
};

/** 支持的语言代码列表（即合并后消息的顶级键） */
const LOCALE_CODES = Object.keys(messages as Record<string, unknown>);

/**
 * 各 locale 的日期时间格式（供 vue-i18n 的 d() 按 "short" 键使用）。
 *
 * 每个支持的 locale 共用同一组字段选项，具体本地样式
 * （字段顺序、分隔符、12/24 小时制等）由 Intl.DateTimeFormat
 * 按当前 locale 决定。键保持 string 索引签名，与 messages 的
 * 宽松类型一致，避免 createI18n 把 locale 推断为字面量联合类型。
 */
const datetimeFormats: Record<string, { short: Intl.DateTimeFormatOptions }> =
  Object.fromEntries(
    LOCALE_CODES.map((code) => [code, { short: SHORT_DATETIME_FORMAT }]),
  );

/**
 * 将候选语言代码匹配到支持的语言（精确匹配失败后按语言前缀匹配）。
 * @param candidate 候选语言代码（如 navigator.language）
 * @returns 匹配到的支持语言代码，未匹配返回 undefined
 */
function matchLocale(candidate: string | undefined): string | undefined {
  if (!candidate) return undefined;
  const lower = candidate.toLowerCase();
  return (
    LOCALE_CODES.find((code) => code.toLowerCase() === lower) ??
    LOCALE_CODES.find((code) =>
      code.toLowerCase().startsWith(lower.split("-")[0] + "-"),
    )
  );
}

/**
 * 全局 vue-i18n 实例（模块内部实现）。
 *
 * 初始语言取浏览器语言；持久化的偏好语言在 main.ts 中加载后
 * 通过 setLocale 生效。
 */
const i18n = createI18n({
  legacy: false,
  locale: matchLocale(navigator.language) ?? DEFAULT_LOCALE,
  fallbackLocale: DEFAULT_LOCALE,
  messages,
  datetimeFormats,
});

/** 全局 composer（t / te / d 为闭包实现，不依赖 this，可直接别名导出） */
const composer = i18n.global;

/** 在 preference 数据库中的键名 */
const LOCALE_PREFERENCE_KEY = "locale";

/**
 * 将全局 i18n 挂载至 Vue 应用：先加载持久化的语言偏好并应用，再挂载。
 * @param app Vue 应用实例
 */
export async function mountI18n(app: App): Promise<void> {
  const saved = await loadPreference(LOCALE_PREFERENCE_KEY);
  setLocale(saved ?? undefined, false);
  app.use(i18n);
}

/** 文案翻译（任意上下文可用；模板中随语言切换自动重渲染） */
export const t = composer.t;

/** 文案键存在性检查（任意上下文可用） */
export const te = composer.te;

/** 日期时间格式化（任意上下文可用；模板中随语言切换自动重渲染） */
export const d = composer.d;

/** 当前语言（响应式、只读；修改语言请使用 setLocale） */
export const currentLocale = computed(() => composer.locale.value);

/**
 * 设置当前语言。
 * @param candidate 候选语言代码；无法匹配到支持的语言时保持现状
 * @param persistent 是否持久化偏好（默认 true；挂载时由持久化数据还原应传 false）
 */
export function setLocale(
  candidate: string | undefined,
  persistent = true,
): void {
  const matched = matchLocale(candidate);
  if (matched) {
    composer.locale.value = matched;
    setVuetifyLocale(matched);
    if (persistent) {
      void storePreference(LOCALE_PREFERENCE_KEY, matched);
    }
  }
}

/** 支持的语言选项 */
interface LocaleOption {
  /** 语言代码 */
  code: string;
  /** 语言的自我描述名称（即 common.locale-label） */
  label: string;
}

/**
 * 支持的语言列表（label 为各语言的自我描述名称，即 common.locale-label）。
 *
 * 列表由受支持的 locale 静态决定，不随运行变化；computed 仅提供只读访问。
 * common.locale-label 是每个语言必须提供的硬性约定，
 * 缺失时报错（console.error + alert）并抛出异常，不做静默回退。
 */
export const supportedLocales = computed<readonly LocaleOption[]>(() =>
  LOCALE_CODES.map((code) => {
    if (!composer.te("common.locale-label", code)) {
      const message = `i18n: locale "${code}" is missing required common["locale-label"]`;
      console.error(message);
      alert(message);
      throw new Error(message);
    }
    // 注意：t 的第二个参数是插值参数，locale 必须放在第三个参数中
    return { code, label: composer.t("common.locale-label", {}, { locale: code }) };
  }),
);
