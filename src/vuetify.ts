/**
 * 全局 Vuetify 实例（模块作用域单例）。
 *
 * 主题基座以 Vuetify 自身的主题表（vuetify.theme.themes）为唯一数据源：
 * 内置主题在创建时注册；自定义主题与偏好主题由 @/themes 在挂载前注入、应用。
 * setup 中通过 useTheme() 访问（同一实例），setup 之外直接使用 vuetify.theme。
 */
import { createVuetify } from "vuetify";
import * as components from "vuetify/components";
import * as directives from "vuetify/directives";
import { en, zhHans } from "vuetify/locale";
import { builtinThemes, DEFAULT_THEME_NAME } from "@/themes/builtin";

/** 全局 Vuetify 实例 */
export const vuetify = createVuetify({
  components,
  directives,
  defaults: {
    VTextField: { autocomplete: "off" },
    VTextarea: { autocomplete: "off" },
    VCombobox: { autocomplete: "off" },
    VSelect: { autocomplete: "off" },
    VNumberInput: { autocomplete: "off" },
  },
  locale: {
    locale: "zhHans",
    fallback: "en",
    messages: { zhHans, en },
  },
  theme: {
    defaultTheme: DEFAULT_THEME_NAME,
    themes: Object.fromEntries(builtinThemes.map((t) => [t.name, t])),
  },
});

/**
 * 同步 Vuetify 语言到当前 i18n 语言。
 * @param lang i18n locale code（如 "zh-CN" / "en-US"）
 */
export function setVuetifyLocale(lang: string): void {
  if (lang.startsWith("zh")) {
    vuetify.locale.current.value = "zhHans";
  } else {
    vuetify.locale.current.value = "en";
  }
}
