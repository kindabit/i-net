// e2e 测试执行器：串行运行 e2e\script 下的全部用例脚本并汇总执行情况。
//
// 用法：在项目根目录执行 `node e2e\script\execute.js`（或 `pnpm test:e2e`）。
// 前置：应用可执行文件需已由 `pnpm tauri:build:debug` 构建（可执行器加 `--build` 自动构建）。
//
// 行为：
// - 按目录名排序发现 script\case_XXX\case_XXX.js 形式的用例脚本；
// - 串行执行（注入输入会独占鼠标键盘，禁止并行）；
// - 每个用例以独立进程运行，完成后读取其 output\report.json 汇总断言结果；
// - 任一用例失败不中断后续用例；最终按失败情况设置进程退出码；
// - 汇总结果同时写入 e2e\script\output\summary.json。
//
// 注意：执行期间会真实操控本机鼠标键盘，请勿执行与本软件无关的操作。

import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import * as api from "./lib/api.js";
import { APP_EXE_PATH, PROJECT_ROOT, SCRIPT_DIR } from "./lib/paths.js";
import { sleep } from "./lib/util.js";

/** 用例目录名模式 */
const CASE_DIR_PATTERN = /^case_\d+/;
/** 汇总文件路径 */
const SUMMARY_FILE = path.join(SCRIPT_DIR, "output", "summary.json");
/** 用例之间的缓冲时间（毫秒），确保上一个应用完全退出 */
const CASE_GAP_MS = 2000;

/**
 * 发现全部用例脚本。
 * @returns {{name: string, dir: string, script: string}[]} 按目录名排序的用例列表。
 */
function discoverCases() {
  return fs
    .readdirSync(SCRIPT_DIR, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && CASE_DIR_PATTERN.test(entry.name))
    .map((entry) => entry.name)
    .sort()
    .map((name) => {
      const dir = path.join(SCRIPT_DIR, name);
      const script = path.join(dir, `${name}.js`);
      return fs.existsSync(script) ? { name, dir, script } : null;
    })
    .filter((entry) => entry !== null);
}

/**
 * 确认调试自动化端口空闲（没有遗留的应用实例在运行）。
 * @returns {Promise<void>} 端口空闲时返回；否则抛错。
 */
async function ensurePortFree() {
  try {
    await api.health();
  } catch {
    return;
  }
  throw new Error(
    "debug automation port 17432 is already in use; close the running app before starting the e2e runner",
  );
}

/**
 * 在项目根目录执行命令并透传输出（用于可选的应用构建）。
 * @param {string} command 命令名。
 * @param {string[]} args 命令参数。
 * @returns {Promise<number>} 退出码。
 */
function runCommand(command, args) {
  return new Promise((resolve) => {
    const child = spawn(command, args, {
      cwd: PROJECT_ROOT,
      stdio: "inherit",
      shell: true,
      windowsHide: true,
    });
    child.on("exit", (code) => resolve(code ?? -1));
  });
}

/**
 * 以独立进程运行单个用例脚本（子进程输出直接透传到当前控制台）。
 * @param {string} script 用例脚本路径。
 * @returns {Promise<{exitCode: number, durationMs: number}>} 退出码与耗时。
 */
function runCase(script) {
  return new Promise((resolve) => {
    const started = Date.now();
    const child = spawn(process.execPath, [script], { stdio: "inherit", windowsHide: true });
    child.on("exit", (code) => {
      resolve({ exitCode: code ?? -1, durationMs: Date.now() - started });
    });
  });
}

/**
 * 读取用例的 report.json。
 * @param {string} dir 用例目录。
 * @returns {object|null} 解析后的报告；不存在或解析失败时为 null。
 */
function readReport(dir) {
  const file = path.join(dir, "output", "report.json");
  if (!fs.existsSync(file)) {
    return null;
  }
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch {
    return null;
  }
}

/**
 * 打印并返回最终汇总。
 * @param {object[]} results 用例结果列表。
 * @returns {object} 汇总对象。
 */
function summarize(results) {
  console.log("\n===== SUMMARY =====");
  for (const result of results) {
    const checks = result.total === null ? "?" : `${result.total - result.failed}/${result.total}`;
    console.log(
      `${result.pass ? "PASS" : "FAIL"}  ${result.name}  checks=${checks}  ${Math.round(result.durationMs / 1000)}s` +
        (result.fatal ? `  fatal=${result.fatal.split("\n")[0]}` : ""),
    );
  }
  const passed = results.filter((result) => result.pass).length;
  const totalChecks = results.reduce((sum, result) => sum + (result.total ?? 0), 0);
  const failedChecks = results.reduce((sum, result) => sum + (result.failed ?? 0), 0);
  console.log(
    `\n${passed}/${results.length} cases passed, checks: ${totalChecks - failedChecks}/${totalChecks}`,
  );
  return {
    finishedAt: new Date().toISOString(),
    cases: results,
    casesPassed: passed,
    casesTotal: results.length,
    checksPassed: totalChecks - failedChecks,
    checksTotal: totalChecks,
  };
}

const cases = discoverCases();
if (cases.length === 0) {
  console.error("no e2e cases found under e2e\\script");
  process.exit(2);
}

if (process.argv.includes("--build")) {
  console.log("building debug app: pnpm tauri:build:debug ...");
  const buildExit = await runCommand("pnpm", ["tauri:build:debug"]);
  if (buildExit !== 0) {
    console.error(`build failed with exit code ${buildExit}`);
    process.exit(2);
  }
}

if (!fs.existsSync(APP_EXE_PATH)) {
  console.error(`app executable not found: ${APP_EXE_PATH}`);
  console.error('run "pnpm tauri:build:debug" first, or use "node e2e\\script\\execute.js --build"');
  process.exit(2);
}

console.log(`e2e runner: ${cases.length} cases discovered`);
console.log(`app executable: ${APP_EXE_PATH} (built ${fs.statSync(APP_EXE_PATH).mtime.toISOString()})`);
console.log("WARNING: the runner injects real mouse and keyboard input; do not use this computer while it runs");
await ensurePortFree();

const results = [];
for (const testCase of cases) {
  console.log(`\n===== RUN ${testCase.name} =====`);
  const outcome = await runCase(testCase.script);
  const report = readReport(testCase.dir);
  const result = {
    name: testCase.name,
    exitCode: outcome.exitCode,
    durationMs: outcome.durationMs,
    total: report?.total ?? null,
    failed: report?.failed ?? null,
    fatal: report?.fatal ?? null,
    pass: outcome.exitCode === 0 && (report === null ? false : report.failed === 0),
  };
  results.push(result);
  console.log(
    `----- ${testCase.name}: ${result.pass ? "PASS" : "FAIL"} (exit=${outcome.exitCode}, ` +
      `checks=${result.total ?? "?"}, failed=${result.failed ?? "?"}, ${Math.round(outcome.durationMs / 1000)}s)`,
  );
  await sleep(CASE_GAP_MS);
}

const summary = summarize(results);
fs.mkdirSync(path.dirname(SUMMARY_FILE), { recursive: true });
fs.writeFileSync(SUMMARY_FILE, JSON.stringify(summary, null, 2), "utf8");
console.log(`summary written: ${SUMMARY_FILE}`);
process.exitCode = summary.casesPassed === summary.casesTotal ? 0 : 1;
