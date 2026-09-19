/**
 * 通用文件选择器的纯函数工具。
 *
 * 提供扩展名匹配、路径拼接与根路径判断，供文件选择器组件与调用方复用。
 * 模块不依赖任何业务模块，可在非组件环境中独立测试。
 */

/**
 * 判断文件名是否匹配给定的扩展名列表。
 * @param fileName 文件名
 * @param extensions 扩展名列表（不带点；大小写不敏感）；为空视为全部匹配
 * @returns 文件名的最后一段扩展名命中列表时返回 true；无扩展名时返回 false
 */
export function matchesExtensions(
  fileName: string,
  extensions: readonly string[],
): boolean {
  if (extensions.length === 0) return true;
  const extension = lastExtensionOf(fileName);
  if (extension === "") return false;
  return extensions.some((item) => item.toLowerCase() === extension);
}

/**
 * 取文件名最后一段扩展名（不含点，统一为小写）。
 * @param fileName 文件名
 * @returns 小写扩展名；无扩展名（含以点号结尾）时返回空字符串
 */
function lastExtensionOf(fileName: string): string {
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex < 0 || dotIndex === fileName.length - 1) return "";
  return fileName.slice(dotIndex + 1).toLowerCase();
}

/**
 * 拼接目录路径与文件名。
 * @param directory 目录路径
 * @param fileName 文件名
 * @returns 目录以 "/" 或 "\" 结尾时直接拼接，否则以 "/" 连接
 */
export function joinPath(directory: string, fileName: string): string {
  return /[/\\]$/.test(directory)
    ? `${directory}${fileName}`
    : `${directory}/${fileName}`;
}

/** 保存文件名校验的错误类型 */
export type FileNameError = "invalid-character" | "reserved-name";

/** 文件名非法字符（Windows 禁用字符与控制字符） */
const INVALID_FILE_NAME_PATTERN = /[<>:"/\\|?*\u0000-\u001f]/;

/** 系统保留设备名（不区分大小写，可带扩展名） */
const RESERVED_FILE_NAME_PATTERN =
  /^(?:con|prn|aux|nul|com[0-9]|lpt[0-9])(?:\..*)?$/i;

/**
 * 校验用户输入的保存文件名。
 *
 * 按 Windows 规则跨平台统一校验（禁用字符、路径分隔符、点号/空格的非法位置、
 * 系统保留设备名），避免各平台行为差异，并防止名称被操作系统静默规范化后
 * 覆盖到与覆盖确认不一致的目标文件。
 * 调用方需先去除首尾空白并完成非空校验。
 * @param name 已去除首尾空白的文件名
 * @returns 含非法字符或格式时返回 "invalid-character"；为系统保留设备名时返回 "reserved-name"；合法时返回 null
 */
export function validateFileName(name: string): FileNameError | null {
  if (name === "." || name === "..") return "invalid-character";
  if (name.endsWith(".") || name.endsWith(" ")) return "invalid-character";
  if (INVALID_FILE_NAME_PATTERN.test(name)) return "invalid-character";
  if (RESERVED_FILE_NAME_PATTERN.test(name)) return "reserved-name";
  return null;
}
