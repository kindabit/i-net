// case_018 首页自动填最后打开的数据库。
//
// 流程（同一数据目录、三段应用会话；会话 1 以空数据目录启动，不复制 fixture）：
//   会话 1：S0 空数据启动基线 → S1/F2 空列表不自动填（清空剪贴板后聚焦输入框
//     全选复制，读回为空串）→ S2 创建 zz-e2e-018-a 进入画布宇宙 → S3 保存并退出回首页
//     → S4 自动填 A → S5/F1 停留在名称步骤 → S6 创建 zz-e2e-018-b 并保存退出
//     → S7 自动填更新为 B → S8/F4 手动输入 zz-e2e-018-manual 后开关归档管理对话框，
//     输入不被覆盖
//   会话 2：S9 重启后自动填 B（持久化）→ S10 点「确认」进入密码步（打开形态，
//     无「创建密码」「确认密码」）→ S11 输入密码解锁出现「根画布」
//     → S12 保存并退出后自动填仍为 B → S13 归档 B（snackbar + 行状态「已归档」）
//   会话 3：S14/F3 归档 B 后重启自动填 A（归档库不参与自动填）→ S15 /shutdown 收尾
//
// 脚本规范要点：
// - UI 树不返回输入框的值，名称读值一律用「聚焦 → Ctrl+A → Ctrl+C → Get-Clipboard」；
//   空输入框的全选复制不产生选区，故读值前必须先清空系统剪贴板（Set-Clipboard -Value $null），
//   否则无法区分「未写入」与上次残留值；
// - 自动填后面板状态的断言方式：UI 树中可编辑控件集合（存在「数据库名称」，不存在
//   「密码」「创建密码」「确认密码」）；空列表的「列表已加载」依据为候选下拉显示「无匹配项」；
// - 名称读值的等待以轮询（读回值等于期望）表达状态条件，不用固定 sleep 代替；
// - 多段会话用 launch/stop 手动管理；每段会话结束等待进程真正退出后再启动下一段；
// - 应用生命周期由 closeSession 兜底，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_018\case_018.js`

import { execSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as api from "../lib/api.js";
import { launch, stop, prepareCaseOutput } from "../lib/app.js";
import { Report, finalizeReport } from "../lib/report.js";
import * as ui from "../lib/ui.js";
import { sleep, waitForCondition } from "../lib/util.js";

const CASE_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.join(CASE_DIR, "output");
const DATA_DIR = path.join(OUTPUT_DIR, "data");
/** 三段会话共用的应用日志（追加写入） */
const LOG_FILE = path.join(OUTPUT_DIR, "app.log");

/** 会话 1 创建的两个数据库名称与共用密码 */
const DB_NAME_A = "zz-e2e-018-a";
const DB_NAME_B = "zz-e2e-018-b";
/** S8 手动输入的名称（用于验证归档对话框关闭不覆盖输入） */
const MANUAL_NAME = "zz-e2e-018-manual";
const DB_PASSWORD = "e2e-password";

const report = new Report("case_018 首页自动填最后打开的数据库");

let shotIndex = 0;
/** 当前会话的应用句柄（null 表示当前无会话） */
let app = null;

/**
 * 截取当前界面并保存到 output 目录（带自增编号）。
 * @param {string} label 截图标签。
 * @returns {Promise<string>} 截图文件路径。
 */
async function snap(label) {
  shotIndex += 1;
  const file = path.join(OUTPUT_DIR, `${String(shotIndex).padStart(2, "0")}-${label}.png`);
  await api.saveScreenshot(file);
  console.log(`  (screenshot) ${file}`);
  return file;
}

// ---------- 剪贴板读值 ----------

/**
 * 清空系统剪贴板（用于区分「未写入」与上次残留值）。
 * @returns {void} 无返回值。
 */
function clearClipboard() {
  execSync('powershell -NoProfile -Command "Set-Clipboard -Value $null"', {
    encoding: "utf8",
    timeout: 15000,
  });
}

/**
 * 读取系统剪贴板文本（仅用于验证脚本自身复制或写入的测试内容）。
 * @returns {string|null} 剪贴板文本；读取失败时为 null。
 */
function readClipboard() {
  try {
    return execSync(
      'powershell -NoProfile -Command "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-Clipboard -Raw"',
      { encoding: "utf8", timeout: 15000 },
    ).replace(/\r?\n$/, "");
  } catch (error) {
    console.log(`  (clipboard read failed: ${error.message})`);
    return null;
  }
}

/**
 * 聚焦「数据库名称」输入框并以剪贴板读回其值（先清空剪贴板）。
 * @returns {Promise<string|null>} 读回的文本；读取失败时为 null。
 */
async function readHomeNameOnce() {
  clearClipboard();
  await ui.clickEditable("数据库名称");
  await sleep(250);
  await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
  await sleep(350);
  return readClipboard();
}

/**
 * 轮询读回「数据库名称」输入框的值，直到等于期望值或超时。
 * @param {string} expected 期望值。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=15000] 超时毫秒数。
 * @returns {Promise<string|null>} 最后一次读到的值。
 */
async function waitForHomeNameValue(expected, { timeout = 15000 } = {}) {
  const deadline = Date.now() + timeout;
  let value = null;
  do {
    value = await readHomeNameOnce();
    if (value === expected) {
      return value;
    }
    await sleep(400);
  } while (Date.now() < deadline);
  return value;
}

// ---------- UI 树查询 ----------

/**
 * 深度展开 UI 树为节点数组。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 全部节点数组（先序）。
 */
function flattenNodes(tree) {
  return ui.flatten(tree).map(({ node }) => node);
}

/**
 * 断言首页停留在名称步骤：存在「数据库名称」可编辑控件，且不存在密码类可编辑控件。
 * @param {string} label 报告描述前缀。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {void} 无返回值。
 */
function checkNameStep(label, tree) {
  report.check(ui.findEditable(tree, "数据库名称") !== null, `${label} 存在「数据库名称」可编辑控件`);
  report.check(ui.findEditable(tree, "密码") === null, `${label} 不存在「密码」可编辑控件（未自动跳到密码步骤）`);
  report.check(ui.findEditable(tree, "创建密码") === null, `${label} 不存在「创建密码」可编辑控件`);
  report.check(ui.findEditable(tree, "确认密码") === null, `${label} 不存在「确认密码」可编辑控件`);
}

// ---------- 会话管理 ----------

/**
 * 等待应用进程退出（/health 请求失败或超时视为不可达）。
 * @param {number} [timeoutMs=15000] 超时毫秒数。
 * @returns {Promise<boolean>} 超时前不可达时为 true。
 */
async function waitForAppGone(timeoutMs = 15000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    let up = false;
    try {
      await api.health();
      up = true;
    } catch {
      up = false;
    }
    if (!up) {
      return true;
    }
    await sleep(400);
  }
  return false;
}

/**
 * 启动一段应用会话并记录断言。
 * @param {string} label 会话标签（用于报告与日志）。
 * @returns {Promise<void>} 无返回值。
 */
async function launchSession(label) {
  app = await launch({ dataDir: DATA_DIR, logFile: LOG_FILE });
  report.check(true, `启动应用（${label}）`, `data-dir=${DATA_DIR}`);
}

/**
 * 关闭当前会话应用（兜底；应用已退出时幂等）。
 * @returns {Promise<void>} 无返回值。
 */
async function closeSession() {
  if (!app) {
    return;
  }
  await stop(app).catch(() => {});
  app = null;
}

/**
 * 等待首页就绪并确保界面语言为中文：首页未出现中文名称输入框时，
 * 通过右上角「切换语言」菜单切回「简体中文」。
 * @returns {Promise<boolean>} 是否执行了语言切换。
 */
async function ensureChineseUi() {
  const initial = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return (ui.findEditable(tree, "数据库名称") ?? ui.findEditable(tree, "Database Name")) ? tree : null;
    },
    { timeout: 30000, label: "首页数据库名称输入框" },
  );
  if (ui.findEditable(initial, "数据库名称")) {
    return false;
  }
  console.log("  (ui language is not Chinese; switching back via the language menu)");
  const button = ui.findButton(initial, "切换语言") ?? ui.findButton(initial, "Switch Language");
  if (!button) {
    throw new Error("language switch button not found on home page");
  }
  await ui.clickNode(button);
  await ui.clickText("简体中文", { exact: true });
  await ui.waitForEditable("数据库名称", { timeout: 15000 });
  return true;
}

// ---------- 首页与数据库操作 ----------

/**
 * 创建数据库：名称步输入名称并确认，密码步输入两次密码并确认，等待进入画布宇宙。
 * @param {string} name 数据库名称。
 * @returns {Promise<void>} 无返回值。
 */
async function createDatabase(name) {
  await ui.typeIntoEditable("数据库名称", name);
  await ui.clickButton("确认");
  await ui.waitForEditable("创建密码", { timeout: 15000 });
  await ui.typeIntoEditable("创建密码", DB_PASSWORD);
  await ui.typeIntoEditable("确认密码", DB_PASSWORD);
  await ui.clickButton("确认");
  await ui.waitForTextStable("根画布", { timeout: 40000, settleMs: 800 });
  await sleep(600);
}

/**
 * 点击右下角「保存并退出」保存数据库并回到首页（应用不退出）。
 * @returns {Promise<void>} 无返回值。
 */
async function saveAndExit() {
  const saveExit = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return (
        flattenNodes(tree)
          .filter((n) => n.role === "button" && ui.subtreeText(n).includes("保存并退出"))
          .sort((a, b) => b.bounds.top - a.bounds.top)[0] ?? null
      );
    },
    { timeout: 10000, interval: 200, label: "「保存并退出」按钮" },
  );
  await ui.clickNode(saveExit);
  await ui.waitForEditable("数据库名称", { timeout: 30000 });
  await sleep(1200);
}

// ---------- 归档管理 ----------

/**
 * 打开右下角归档管理对话框。
 * @returns {Promise<void>} 无返回值。
 */
async function openArchiveDialog() {
  const button = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "aria-label", "数据库归档与删除"),
    { timeout: 10000, interval: 200, label: "「数据库归档与删除」按钮" },
  );
  await ui.clickNode(button);
  await ui.waitForDialog("归档管理", { timeout: 10000 });
  await sleep(900);
}

/**
 * 在归档管理列表中查找指定名称的数据库行。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} name 数据库名称。
 * @returns {object|null} 列表行节点；未命中时为 null。
 */
function findArchiveRow(tree, name) {
  const dialog = ui.findDialog(tree, "归档管理");
  if (!dialog) {
    return null;
  }
  return (
    flattenNodes([dialog]).find(
      (node) => node.role === "listitem" && ui.subtreeText(node).includes(name),
    ) ?? null
  );
}

/**
 * 读取归档管理列表行的按钮文本数组。
 * @param {object|null} row 列表行节点。
 * @returns {string[]} 按钮文本数组；row 为空时为空数组。
 */
function rowButtonTexts(row) {
  return row
    ? flattenNodes([row])
        .filter((node) => node.role === "button")
        .map((node) => ui.subtreeText(node))
    : [];
}

/**
 * 点击归档管理对话框内的「关闭」按钮并等待对话框消失。
 * @returns {Promise<void>} 无返回值。
 */
async function closeArchiveDialog() {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = ui.findDialog(tree, "归档管理");
      return dialog ? ui.findButton(tree, "关闭", { root: dialog, exact: true }) : null;
    },
    { timeout: 10000, interval: 250, label: "归档管理「关闭」按钮" },
  );
  await ui.clickNode(button);
  await ui.waitForDialogGone("归档管理", { timeout: 10000 });
  await sleep(700);
}

/**
 * 归档指定数据库：打开归档管理，点击该行的「归档」按钮，等待成功 snackbar 与行状态更新。
 * @param {string} name 数据库名称。
 * @returns {Promise<void>} 无返回值。
 */
async function archiveDatabase(name) {
  await openArchiveDialog();
  {
    const tree = await api.uiTree();
    const row = findArchiveRow(tree, name);
    const buttons = rowButtonTexts(row);
    report.check(
      row !== null && buttons.length === 1 && buttons[0] === "归档",
      `S13 归档前「${name}」行处于未归档状态（仅「归档」按钮）`,
      buttons.join(",") || "none",
    );
  }
  const archiveButton = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const row = findArchiveRow(tree, name);
      if (!row) {
        return null;
      }
      const hit = ui.findButton([row], "归档", { exact: true });
      return hit && hit.states?.disabled !== true ? hit : null;
    },
    { timeout: 10000, interval: 250, label: `「${name}」行「归档」按钮` },
  );
  await ui.clickNode(archiveButton);
  const snackbar = await ui
    .waitForTextStable(`数据库“${name}”已归档`, { timeout: 10000 })
    .catch(() => null);
  report.check(snackbar !== null, `S13 归档后 snackbar「数据库“${name}”已归档」`);
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const row = findArchiveRow(tree, name);
      return row && ui.subtreeText(row).includes("已归档") ? tree : null;
    },
    { timeout: 8000, interval: 250, label: `「${name}」行状态变为「已归档」` },
  );
  {
    const tree = await api.uiTree();
    const row = findArchiveRow(tree, name);
    const buttons = rowButtonTexts(row);
    report.check(
      buttons.includes("解除归档") && buttons.includes("删除"),
      `S13 「${name}」行出现「解除归档」「删除」按钮`,
      buttons.join(",") || "none",
    );
  }
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 会话 1 启动基线（步骤 1）");
  await launchSession("会话 1");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  await ensureChineseUi();
  report.check(ui.findEditable(await api.uiTree(), "数据库名称") !== null, "首页显示「数据库名称」输入框");
  report.check(ui.findButton(await api.uiTree(), "确认") !== null, "首页显示「确认」按钮");
  await snap("s0-home-empty");

  report.section("S1/F2 空列表不自动填（步骤 2）");
  clearClipboard();
  await ui.clickEditable("数据库名称");
  await ui.waitForTextStable("无匹配项", { timeout: 10000 });
  await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
  await sleep(400);
  const emptyValue = readClipboard();
  report.check(emptyValue === "", "F2 空列表时名称输入框读回为空串（不自动填）", `clip=[${emptyValue}]`);
  await snap("s1-empty-list-no-fill");

  report.section("S2 创建数据库 A（步骤 3-4）");
  await createDatabase(DB_NAME_A);
  report.check(ui.hasText(await api.uiTree(), "根画布"), "创建 A 成功并进入画布宇宙");
  await snap("s2-a-created");
  await saveAndExit();
  {
    const up = await api.health().then(() => true).catch(() => false);
    report.check(up, "「保存并退出」后应用未退出（/health 正常）");
  }
  await snap("s3-home-after-save-exit-a");

  report.section("S4 保存并退出后自动填 A（步骤 5-6）");
  const valueA = await waitForHomeNameValue(DB_NAME_A);
  report.check(valueA === DB_NAME_A, `自动填入「${DB_NAME_A}」（最后打开的未归档数据库）`, `clip=[${valueA}]`);
  {
    const tree = await api.uiTree();
    checkNameStep("F1 自动填 A 后", tree);
  }
  await snap("s4-autofill-a");

  report.section("S6 创建数据库 B（步骤 7）");
  await createDatabase(DB_NAME_B);
  report.check(ui.hasText(await api.uiTree(), "根画布"), "创建 B 成功并进入画布宇宙");
  await saveAndExit();
  await snap("s6-home-after-save-exit-b");

  report.section("S7 自动填更新为 B（步骤 8）");
  const valueB = await waitForHomeNameValue(DB_NAME_B);
  report.check(valueB === DB_NAME_B, `自动填更新为「${DB_NAME_B}」（最后打开者变化）`, `clip=[${valueB}]`);
  {
    const tree = await api.uiTree();
    checkNameStep("F1 自动填 B 后", tree);
  }
  await snap("s7-autofill-b");

  report.section("S8/F4 归档对话框关闭不覆盖输入（步骤 9）");
  await ui.typeIntoEditable("数据库名称", MANUAL_NAME);
  const manualBefore = await readHomeNameOnce();
  report.check(manualBefore === MANUAL_NAME, "手动输入后名称输入框值为 zz-e2e-018-manual", `clip=[${manualBefore}]`);
  await openArchiveDialog();
  {
    const tree = await api.uiTree();
    report.check(findArchiveRow(tree, DB_NAME_A) !== null, `归档管理列表包含「${DB_NAME_A}」`);
    report.check(findArchiveRow(tree, DB_NAME_B) !== null, `归档管理列表包含「${DB_NAME_B}」`);
  }
  await snap("s8-archive-dialog-open");
  await closeArchiveDialog();
  const manualAfter = await readHomeNameOnce();
  report.check(
    manualAfter === MANUAL_NAME,
    "F4 归档管理对话框关闭后输入未被自动填覆盖（仍为 zz-e2e-018-manual）",
    `clip=[${manualAfter}]`,
  );
  await snap("s8-manual-kept-after-dialog");

  await closeSession();
  report.check(await waitForAppGone(15000), "会话 1 应用进程已退出（/health 不可达）");

  report.section("S9 会话 2 重启后自动填持久化（步骤 10）");
  await launchSession("会话 2");
  await ensureChineseUi();
  {
    const value = await waitForHomeNameValue(DB_NAME_B);
    report.check(value === DB_NAME_B, `重启后自动填「${DB_NAME_B}」（跨会话持久化生效）`, `clip=[${value}]`);
  }
  {
    const tree = await api.uiTree();
    checkNameStep("F1 重启自动填后", tree);
  }
  await snap("s9-session2-autofill-b");

  report.section("S10 确认进入密码步（步骤 11）");
  await ui.clickButton("确认");
  await ui.waitForEditable("密码", { timeout: 15000 });
  await sleep(500);
  {
    const tree = await api.uiTree();
    report.check(ui.findEditable(tree, "密码") !== null, "出现「密码」输入框");
    report.check(ui.findEditable(tree, "创建密码") === null, "不出现「创建密码」输入框（按已注册数据库打开）");
    report.check(ui.findEditable(tree, "确认密码") === null, "不出现「确认密码」输入框（按已注册数据库打开）");
  }
  await snap("s10-password-step-open-form");

  report.section("S11 输入密码解锁（步骤 12）");
  await ui.typeIntoEditable("密码", DB_PASSWORD);
  await ui.clickButton("确认");
  await ui.waitForTextStable("根画布", { timeout: 40000, settleMs: 800 });
  report.check(true, "输入正确密码后解锁成功（出现「根画布」）");
  await snap("s11-unlocked-b");

  report.section("S12 保存并退出后自动填仍为 B（步骤 13）");
  await saveAndExit();
  {
    const value = await waitForHomeNameValue(DB_NAME_B);
    report.check(
      value === DB_NAME_B,
      `打开操作更新 last_open_time 后自动填仍为「${DB_NAME_B}」`,
      `clip=[${value}]`,
    );
  }
  await snap("s12-autofill-b-after-open");

  report.section("S13 归档数据库 B（步骤 14）");
  await archiveDatabase(DB_NAME_B);
  {
    const tree = await api.uiTree();
    const rowA = findArchiveRow(tree, DB_NAME_A);
    report.check(
      rowA !== null && ui.subtreeText(rowA).includes("未归档"),
      `归档后「${DB_NAME_A}」行仍为「未归档」（仅 B 被归档）`,
    );
  }
  await snap("s13-archived-b");
  await closeArchiveDialog();
  await closeSession();
  report.check(await waitForAppGone(15000), "会话 2 应用进程已退出（/health 不可达）");

  report.section("S14/F3 会话 3 归档库不参与自动填（步骤 15）");
  await launchSession("会话 3");
  await ensureChineseUi();
  {
    const value = await waitForHomeNameValue(DB_NAME_A);
    report.check(
      value === DB_NAME_A,
      `归档 B 后重启自动填「${DB_NAME_A}」（归档数据库不在未归档列表中）`,
      `clip=[${value}]`,
    );
  }
  {
    const tree = await api.uiTree();
    checkNameStep("F1 归档后自动填时", tree);
  }
  await snap("s14-session3-autofill-a");

  report.section("S15 /shutdown 收尾（步骤 16）");
  await api.shutdown();
  report.check(await waitForAppGone(15000), "POST /shutdown 后应用正常退出（/health 不可达）");
  await closeSession();
  report.check(true, "F5 按计划不构造：refreshMetadatas 失败时的自动填行为需要破坏元数据数据库（记录）");
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
  await main();
} catch (error) {
  fatal = error;
  await snap("fatal").catch(() => {});
  report.check(false, "用例执行中断", error.message);
} finally {
  await closeSession();
  finalizeReport(report, OUTPUT_DIR, fatal);
}
