/**
 * 通用 Snackbar 消息队列。
 *
 * snackbarText 由任意模块调用（含 setup 之外），
 * 消息进入队列后由 AppSnackbarQueue 组件统一展示。
 * snackbarErrorCode 为后端 ErrorCode 错误提供国际化的错误提示。
 * DataCorruption* 系列错误改走受控崩溃（use-fatal-error），不再入 Snackbar 队列。
 */
import { ref } from "vue";
import { t, te } from "@/i18n";
import { isErrorCode } from "@/error-code";
import { triggerFatalError } from "@/composables/use-fatal-error";

/** Snackbar 消息（VSnackbarQueue 的 modelValue 元素） */
interface SnackbarMessage {
  /** 标题文本 */
  title?: string;
  /** 正文文本 */
  text?: string;
  /** 颜色（Vuetify 颜色名或 CSS 颜色） */
  color?: string;
  /** 前置图标 */
  prependIcon?: string;
  /** 显示时长（毫秒） */
  timeout?: number;
}

/** Snackbar 消息队列（AppSnackbarQueue 组件消费） */
export const snackbarMessages = ref<SnackbarMessage[]>([]);

/** Snackbar 提示级别 */
type SnackbarLevel = "success" | "info" | "warning" | "error";

const levelConfig: Record<SnackbarLevel, { color: string; icon: string }> = {
  success: { color: "success", icon: "$success" },
  info: { color: "info", icon: "$info" },
  warning: { color: "warning", icon: "$warning" },
  error: { color: "error", icon: "$error" },
};

/**
 * 显示普通文本 Snackbar。
 * @param text 提示文本
 * @param level 提示级别，默认为 "info"
 */
export function snackbarText(text: string, level: SnackbarLevel = "info") {
  const config = levelConfig[level];
  snackbarMessages.value.push({
    title: text,
    color: config.color,
    prependIcon: config.icon,
    timeout: 3000,
  });
}

/**
 * 根据后端返回的 ErrorCode 显示错误 Snackbar（文案取自 error-code 国际化模块）。
 * DataCorruption* 系列错误改走受控崩溃流程（use-fatal-error）。
 * 未识别的错误以文本形式直接展示。
 * @param raw 后端返回的原始错误对象
 */
export function snackbarErrorCode(raw: unknown): void {
  if (isErrorCode(raw)) {
    if (raw.variant.startsWith("DataCorruption")) {
      triggerFatalError(raw);
      return;
    }
    const titleKey = `error-code.${raw.variant}.title`;
    const textKey = `error-code.${raw.variant}.text`;
    const title = te(titleKey) ? t(titleKey) : raw.variant;
    const data = raw.data ?? {};
    const text = te(textKey)
      ? t(textKey, data)
      : undefined;
    snackbarMessages.value.push({
      title,
      text,
      color: levelConfig.error.color,
      prependIcon: levelConfig.error.icon,
      timeout: 5000,
    });
    return;
  }
  if (raw instanceof Error) {
    snackbarText(raw.message, "error");
    console.error(raw);
    return;
  }
  snackbarText(JSON.stringify(raw), "error");
}
