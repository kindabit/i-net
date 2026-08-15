/**
 * 通用 preference 读写工具（技术基座，不包含任何业务含义）。
 *
 * 对后端 preference 数据库 thin-wrapper：将 KV 读写与错误提示封装为
 * 可直接调用的工具函数。业务模块自行定义 preference key，并通过本模块
 * 读写——本模块不感知任何业务键名、不做任何语义假设。
 */
import { preferenceGet, preferenceSave, preferenceSet } from "@/api";
import { snackbarErrorCode } from "@/composables/use-snackbar";

/**
 * 读取 preference 值（失败时通过 snackbar 提示用户并返回 null）。
 * @param name preference 键名
 * @returns 偏好值；不存在或读取失败时返回 null
 */
export async function loadPreference(name: string): Promise<string | null> {
  try {
    return await preferenceGet(name);
  } catch (error) {
    snackbarErrorCode(error);
    return null;
  }
}

/**
 * 写入 preference 值并落盘（失败时通过 snackbar 提示用户）。
 * @param name preference 键名
 * @param value 偏好值
 */
export async function storePreference(
  name: string,
  value: string,
): Promise<void> {
  try {
    await preferenceSet(name, value);
    await preferenceSave();
  } catch (error) {
    snackbarErrorCode(error);
  }
}
