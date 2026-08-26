/**
 * 数据损坏受控崩溃的全局状态。
 *
 * snackbarErrorCode 识别到 DataCorruption* 系列错误时调用 triggerFatalError，
 * 状态由 App.vue 挂载的 FatalErrorDialog 消费（阻塞式展示，确认后退出应用）。
 */
import { ErrorCode } from "@/error-code";
import { ref } from "vue";

/** 待展示的致命错误（DataCorruption* 错误的 variant 与 data）；null 表示无 */
export const fatalError = ref<ErrorCode | null>(null);

/**
 * 触发数据损坏受控崩溃流程。
 * @param variant 错误码变体名（DataCorruption 开头）
 * @param data 错误附带的上下文数据
 */
export function triggerFatalError(errorCode: ErrorCode) {
  fatalError.value = errorCode;
}
