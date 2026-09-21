// 应用进程的启动、就绪等待与关闭（Windows 平台）。

import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import * as api from "./api.js";
import { APP_EXE_PATH } from "./paths.js";
import { sleep } from "./util.js";

/** 应用启动的默认就绪等待上限（毫秒） */
export const DEFAULT_BOOT_TIMEOUT_MS = 120000;

/**
 * 探测调试自动化服务是否已就绪。
 * @returns {Promise<boolean>} 服务可访问时为 true。
 */
async function isServiceUp() {
  try {
    await api.health();
    return true;
  } catch {
    return false;
  }
}

/**
 * 等待子进程退出。
 * @param {import("node:child_process").ChildProcess} child 子进程。
 * @param {number} timeoutMs 超时毫秒数。
 * @returns {Promise<boolean>} 超时前退出时为 true。
 */
function waitForExit(child, timeoutMs) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve(true);
  }
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(false), timeoutMs);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve(true);
    });
  });
}

/**
 * 强制结束应用进程树（shutdown 接口失效时的兜底手段）。
 * @param {import("node:child_process").ChildProcess} child 子进程。
 * @returns {Promise<void>} 无返回值。
 */
async function forceKill(child) {
  if (child.exitCode !== null) {
    return;
  }
  spawn("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
    windowsHide: true,
    stdio: "ignore",
  });
  await waitForExit(child, 10000);
}

/**
 * 启动应用（预构建的调试可执行文件 + --data-dir）并等待调试自动化服务就绪。
 * 应用可执行文件由 `pnpm tauri:build:debug` 产出；本函数不触发任何构建，
 * 以保证每个用例的启动开销最小且构建版本由调用方明确控制。
 * @param {object} options 启动参数。
 * @param {string} options.dataDir 用户数据目录（--data-dir 参数值）。
 * @param {string} options.logFile 应用输出日志文件路径（追加写入）。
 * @param {number} [options.bootTimeoutMs=DEFAULT_BOOT_TIMEOUT_MS] 就绪等待上限毫秒数。
 * @returns {Promise<{child: import("node:child_process").ChildProcess, logFile: string}>} 应用进程句柄与日志路径。
 */
export async function launch({ dataDir, logFile, bootTimeoutMs = DEFAULT_BOOT_TIMEOUT_MS }) {
  if (await isServiceUp()) {
    throw new Error("debug automation port 17432 is already in use; close the running app first");
  }
  if (!fs.existsSync(APP_EXE_PATH)) {
    throw new Error(
      `debug app executable not found: ${APP_EXE_PATH}; run "pnpm tauri:build:debug" first`,
    );
  }
  fs.mkdirSync(path.dirname(logFile), { recursive: true });
  const log = fs.createWriteStream(logFile, { flags: "a" });
  const child = spawn(APP_EXE_PATH, ["--data-dir", dataDir], {
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  child.stdout.pipe(log);
  child.stderr.pipe(log);
  const deadline = Date.now() + bootTimeoutMs;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(`app process exited during boot (code ${child.exitCode}); see ${logFile}`);
    }
    if (await isServiceUp()) {
      return { child, logFile };
    }
    await sleep(250);
  }
  await forceKill(child);
  throw new Error(`app did not become ready within ${bootTimeoutMs}ms; see ${logFile}`);
}

/**
 * 关闭应用进程：先请求 /shutdown 正常退出，超时则强制结束进程树。
 * @param {{child: import("node:child_process").ChildProcess}} app launch 返回的句柄。
 * @returns {Promise<void>} 无返回值。
 */
export async function stop(app) {
  try {
    await api.shutdown();
  } catch {
    // 服务可能已经退出，忽略失败并继续等待进程结束。
  }
  const exited = await waitForExit(app.child, 30000);
  if (!exited) {
    await forceKill(app.child);
  }
  await sleep(1500);
}

/**
 * 应用生命周期封装：启动应用、执行主流程、无论成败都关闭应用。
 * @param {object} options 启动参数（见 launch）。
 * @param {(app: {child: import("node:child_process").ChildProcess, logFile: string}) => Promise<void>} fn 应用就绪后执行的主流程。
 * @returns {Promise<void>} 无返回值。
 */
export async function withApp(options, fn) {
  const app = await launch(options);
  try {
    await fn(app);
  } finally {
    await stop(app);
  }
}

/**
 * 清空并重建用例输出目录（output\data 等运行产物随目录一起重置）。
 * @param {string} outputDir 输出目录路径。
 * @returns {void} 无返回值。
 */
export function prepareCaseOutput(outputDir) {
  fs.rmSync(outputDir, { recursive: true, force: true });
  fs.mkdirSync(outputDir, { recursive: true });
}
