// 调试自动化 HTTP 接口的封装：/health、/screenshot、/ui-tree、/input、/shutdown。

import fs from "node:fs";
import path from "node:path";

/** 调试自动化服务的固定基地址 */
export const BASE_URL = "http://127.0.0.1:17432";

/**
 * 向调试自动化服务发送 HTTP 请求。
 * @param {string} method HTTP 方法。
 * @param {string} endpoint 端点路径（以 / 开头）。
 * @param {object} [options] 可选参数。
 * @param {unknown} [options.body] JSON 请求体；缺省时不携带请求体。
 * @param {number} [options.timeoutMs=60000] 超时毫秒数。
 * @returns {Promise<Response>} fetch 响应对象。
 */
async function request(method, endpoint, { body, timeoutMs = 60000 } = {}) {
  return fetch(`${BASE_URL}${endpoint}`, {
    method,
    headers: body === undefined ? undefined : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(timeoutMs),
  });
}

/**
 * 读取服务健康信息。
 * @returns {Promise<{version: string, width: number, height: number}>} 版本号与整窗物理像素尺寸。
 */
export async function health() {
  const res = await request("GET", "/health", { timeoutMs: 3000 });
  if (!res.ok) {
    throw new Error(`GET /health -> ${res.status}`);
  }
  return res.json();
}

/**
 * 读取整窗截图。
 * @returns {Promise<Buffer>} PNG 图片数据。
 */
export async function screenshot() {
  const res = await request("GET", "/screenshot");
  if (!res.ok) {
    throw new Error(`GET /screenshot -> ${res.status} ${await res.text()}`);
  }
  return Buffer.from(await res.arrayBuffer());
}

/**
 * 保存整窗截图到指定文件。
 * @param {string} filePath 目标 PNG 文件路径。
 * @returns {Promise<string>} 保存的文件路径。
 */
export async function saveScreenshot(filePath) {
  const buf = await screenshot();
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, buf);
  return filePath;
}

/**
 * 读取整窗 UI 树。
 * @returns {Promise<object[]>} UI 树顶层节点数组。
 */
export async function uiTree() {
  const res = await request("GET", "/ui-tree");
  if (!res.ok) {
    throw new Error(`GET /ui-tree -> ${res.status}`);
  }
  return res.json();
}

/**
 * 执行输入命令队列；队列失败时抛出携带服务端错误详情的异常。
 * @param {object[]} commands 命令数组（非空）。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeoutMs=60000] 超时毫秒数。
 * @returns {Promise<number>} 请求耗时毫秒数。
 */
export async function postInput(commands, { timeoutMs = 60000 } = {}) {
  if (!Array.isArray(commands) || commands.length === 0) {
    throw new Error("postInput: commands must be a non-empty array");
  }
  const started = Date.now();
  const res = await request("POST", "/input", { body: { commands }, timeoutMs });
  const ms = Date.now() - started;
  if (res.status !== 204) {
    throw new Error(`POST /input -> ${res.status} ${await res.text()}`);
  }
  return ms;
}

/**
 * 执行输入命令队列并返回原始结果，不因业务错误抛异常（供故意触发错误路径的用例使用）。
 * @param {object[]} commands 命令数组（非空）。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeoutMs=60000] 超时毫秒数。
 * @returns {Promise<{status: number, body: object|null, ms: number}>} HTTP 状态、解析后的响应体与耗时。
 */
export async function sendInput(commands, { timeoutMs = 60000 } = {}) {
  if (!Array.isArray(commands) || commands.length === 0) {
    throw new Error("sendInput: commands must be a non-empty array");
  }
  const started = Date.now();
  const res = await request("POST", "/input", { body: { commands }, timeoutMs });
  const ms = Date.now() - started;
  const text = await res.text();
  let body = null;
  if (text) {
    try {
      body = JSON.parse(text);
    } catch {
      body = { raw: text };
    }
  }
  return { status: res.status, body, ms };
}

/**
 * 请求关闭应用：服务先返回 204，随后应用按正常退出流程自行退出。
 * @returns {Promise<void>} 无返回值。
 */
export async function shutdown() {
  const res = await request("POST", "/shutdown", { timeoutMs: 10000 });
  if (res.status !== 204) {
    throw new Error(`POST /shutdown -> ${res.status}`);
  }
}

// ---------- /input 命令构造器 ----------

/**
 * 构造鼠标移动命令。
 * @param {number} x 目标 x 坐标（整窗物理像素）。
 * @param {number} y 目标 y 坐标（整窗物理像素）。
 * @returns {object} 命令对象。
 */
export const mouseMove = (x, y) => ({ kind: "mouse_move", x, y });

/**
 * 构造鼠标按下命令。
 * @param {"left"|"right"} [button="left"] 鼠标按键。
 * @returns {object} 命令对象。
 */
export const mousePress = (button = "left") => ({ kind: "mouse_press", button });

/**
 * 构造鼠标松开命令。
 * @param {"left"|"right"} [button="left"] 鼠标按键。
 * @returns {object} 命令对象。
 */
export const mouseRelease = (button = "left") => ({ kind: "mouse_release", button });

/**
 * 构造鼠标点击命令。
 * @param {"left"|"right"} [button="left"] 鼠标按键。
 * @param {number} [count=1] 点击次数（1..=10）。
 * @returns {object} 命令对象。
 */
export const mouseClick = (button = "left", count = 1) => ({ kind: "mouse_click", button, count });

/**
 * 构造鼠标滚轮命令。
 * @param {"up"|"down"|"left"|"right"} direction 滚轮方向。
 * @param {number} amount 滚轮刻度数（1..=1000）。
 * @returns {object} 命令对象。
 */
export const mouseScroll = (direction, amount) => ({ kind: "mouse_scroll", direction, amount });

/**
 * 构造鼠标拖拽命令。
 * @param {{x: number, y: number}} from 起点（整窗物理像素）。
 * @param {{x: number, y: number}} to 终点（整窗物理像素）。
 * @param {"left"|"right"} [button="left"] 鼠标按键。
 * @returns {object} 命令对象。
 */
export const mouseDrag = (from, to, button = "left") => ({ kind: "mouse_drag", from, to, button });

/**
 * 构造按键按下命令。
 * @param {string[]} keys 按键名数组（1..=8 个）。
 * @returns {object} 命令对象。
 */
export const keyPress = (keys) => ({ kind: "key_press", keys });

/**
 * 构造按键松开命令。
 * @param {string[]} keys 按键名数组（1..=8 个）。
 * @returns {object} 命令对象。
 */
export const keyRelease = (keys) => ({ kind: "key_release", keys });

/**
 * 构造按键点击命令。
 * @param {string[]} keys 按键名数组（互不重复，1..=8 个）。
 * @param {number} [count=1] 序列重复轮数（1..=10）。
 * @returns {object} 命令对象。
 */
export const keyClick = (keys, count = 1) => ({ kind: "key_click", keys, count });

/**
 * 构造文本输入命令。
 * @param {string} text 要输入的 Unicode 文本（不得包含空字符）。
 * @returns {object} 命令对象。
 */
export const typeText = (text) => ({ kind: "type", text });

/**
 * 构造等待命令。
 * @param {number} duration 时长数值。
 * @param {"ms"|"s"} [unit="ms"] 时长单位。
 * @returns {object} 命令对象。
 */
export const wait = (duration, unit = "ms") => ({ kind: "wait", duration, unit });

/**
 * 构造配置命令（仅对本次请求内其后的命令生效）。
 * @param {object} fields 要覆盖的配置字段（mouseClickSpan、mouseMoveSpeed、keyClickInTurnInterval、keyClickCrossTurnInterval、keyClickSpan、commandInterval）。
 * @returns {object} 命令对象。
 */
export const config = (fields) => ({ kind: "config", ...fields });
