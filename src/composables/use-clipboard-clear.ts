import { ref } from "vue";
import { clipboardClear } from "@/api";
import { loadPreference, storePreference } from "@/preferences";

/** 全局单例状态 */
const timeoutSeconds = ref(10);
const refreshKey = ref(0);

let clearTimer: number | null = null;

/** 初始化时读取配置的超时时间 */
async function init() {
  const value = await loadPreference("clipboard_clear_timeout");
  if (value !== null) {
    timeoutSeconds.value = parseInt(value, 10);
  }
}

/**
 * 启动剪贴板清空倒计时
 * @param copyValue 已复制到剪贴板的值
 */
function startCountdown(_copyValue: string) {
  // 清除已有的倒计时
  if (clearTimer) {
    clearTimeout(clearTimer);
    clearTimer = null;
  }

  refreshKey.value += 1;

  clearTimer = window.setTimeout(() => {
    stopCountdown();
    clipboardClear().catch((e) => {
      console.error("清空剪贴板失败:", e);
    });
  }, timeoutSeconds.value * 1000);
}

/** 停止倒计时 */
function stopCountdown() {
  if (clearTimer) {
    clearTimeout(clearTimer);
    clearTimer = null;
  }
  refreshKey.value = 0;
}

/** 保存超时时间配置到偏好设置 */
async function saveTimeoutConfig() {
  await storePreference("clipboard_clear_timeout", String(timeoutSeconds.value));
}

init();

export function useClipboardClear() {
  return {
    timeoutSeconds,
    refreshKey,
    startCountdown,
    stopCountdown,
    saveTimeoutConfig,
  };
}
