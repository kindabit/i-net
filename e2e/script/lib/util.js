// e2e 脚本通用工具：等待与轮询。

/**
 * 等待指定毫秒数。
 * @param {number} ms 毫秒数。
 * @returns {Promise<void>} 无返回值。
 */
export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * 轮询执行条件函数，直到返回真值或达到超时。
 * @param {() => Promise<unknown> | unknown} fn 条件函数；返回真值视为满足。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @param {number} [options.interval=250] 轮询间隔毫秒数。
 * @param {string} [options.label="condition"] 超时错误消息中使用的条件名。
 * @returns {Promise<unknown>} 条件首次返回的真值。
 */
export async function waitForCondition(fn, { timeout = 10000, interval = 250, label = "condition" } = {}) {
  const deadline = Date.now() + timeout;
  let lastError = null;
  while (Date.now() < deadline) {
    try {
      const value = await fn();
      if (value) {
        return value;
      }
    } catch (error) {
      lastError = error;
    }
    await sleep(interval);
  }
  throw new Error(
    `waitForCondition(${label}) timed out after ${timeout}ms` +
      (lastError ? `; last error: ${lastError.message}` : ""),
  );
}
