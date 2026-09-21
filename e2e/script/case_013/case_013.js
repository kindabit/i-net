// case_013 归档与删除数据库。
//
// 流程：清空 output → 复制 base fixture → 启动应用 → 停留在首页
//   → 步骤 1-2：打开归档管理（初始态：未归档 + 最后打开时间 + 仅「归档」按钮）
//     → 归档（snackbar + 副标题「已归档」+ 出现「解除归档」「删除」+ 删除按钮 error 色像素断言）
//   → 步骤 3-4：关闭对话框 → 首页名称输入 zz → 候选为空（「无匹配项」）
//   → 步骤 5：重新打开归档管理 → 解除归档（snackbar + 回到「未归档」+「删除」消失）
//   → 步骤 6：再次归档 → 打开删除对话框（正文 + 「确认删除」禁用 + 两个输入框）
//   → 步骤 7（F1）：输入错误名称 → 「确认删除」保持禁用、无内联错误文案
//   → 步骤 8（F2）：正确名称 + 错误密码 → snackbar「密码错误」；
//     删除对话框在提交后关闭、归档管理保持打开、数据库行与数据目录不变
//   → 步骤 8b（F4）：重开删除对话框（输入重置、「确认删除」禁用）→ 点击「取消」→ 数据保留
//   → 步骤 9：重开并输入正确密码 → snackbar「数据库“…”已删除」；列表「暂无数据库」；
//     user_database_set 下 uuid 目录被清除
//   → 步骤 10：关闭对话框 → 首页候选为空（输入 zz 显示「无匹配项」）；目录已清理
//   → 步骤 11：重新打开归档管理 → 列表为空
//   → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 断言优先使用 UI 树（文本、bounds、states），截图用于视觉留档；
// - 「删除」按钮的 error 色不在 UI 树属性中暴露（attrs 白名单无颜色），脚本用截图像素统计
//   断言（删除按钮区域红色像素数量 vs「解除归档」按钮区域为 0），并保留截图供目视核对；
// - snackbar 断言使用 waitForTextStable（等过渡静止后复查）；点击会产生同名消息的操作前
//   先等待旧消息消失，规避 Vuetify 消息过渡期间新旧文本并存；
// - 删除对话框在点击「确认删除」后立即关闭（成功与失败路径均如此，见计划末尾的修订记录）；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_013\case_013.js`

import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as api from "../lib/api.js";
import { withApp, prepareCaseOutput } from "../lib/app.js";
import { prepareDataDir } from "../lib/fixtures.js";
import { decodePng } from "../lib/image.js";
import { Report, finalizeReport } from "../lib/report.js";
import * as ui from "../lib/ui.js";
import { sleep, waitForCondition } from "../lib/util.js";

const CASE_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.join(CASE_DIR, "output");
const DATA_DIR = path.join(OUTPUT_DIR, "data");
/** 用户数据库集合目录（删除后比对该目录下的子目录是否被清除） */
const USER_DB_SET_DIR = path.join(DATA_DIR, "user_database_set");

/** fixture 数据库名称与密码（见 _fixtures\README.md） */
const DB_NAME = "zz-e2e-base";
const DB_PASSWORD = "e2e-password";
/** F1 使用的错误数据库名称 */
const WRONG_DB_NAME = "zz-wrong-name";
/** F2 使用的错误密码 */
const WRONG_PASSWORD = "wrong-password";

/** 对话框标题 */
const ARCHIVE_DIALOG_TITLE = "归档管理";
const DELETE_DIALOG_TITLE = "删除数据库";

const report = new Report("case_013 归档与删除数据库");

/** 启动前记录的 user_database_set 下的数据库目录名列表 */
let dbDirsBeforeLaunch = [];

let shotIndex = 0;

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

/**
 * 深度展开 UI 树为节点数组。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 全部节点数组（先序）。
 */
function flattenNodes(tree) {
  return ui.flatten(tree).map(({ node }) => node);
}

/**
 * 列出 user_database_set 下的数据库子目录名。
 * @returns {string[]} 数据库子目录名列表（目录不存在时为空数组）。
 */
function listUserDbDirs() {
  return fs.existsSync(USER_DB_SET_DIR) ? fs.readdirSync(USER_DB_SET_DIR).sort() : [];
}

/**
 * 在 UI 树中查找归档管理对话框。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 对话框节点；未命中时为 null。
 */
function findArchiveDialog(tree) {
  return ui.findDialog(tree, ARCHIVE_DIALOG_TITLE);
}

/**
 * 在 UI 树中查找删除数据库对话框。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 对话框节点；未命中时为 null。
 */
function findDeleteDialog(tree) {
  return ui.findDialog(tree, DELETE_DIALOG_TITLE);
}

/**
 * 在归档管理列表中查找 fixture 数据库行。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 列表项节点；未命中时为 null。
 */
function findRow(tree) {
  const dialog = findArchiveDialog(tree);
  if (!dialog) {
    return null;
  }
  return (
    flattenNodes([dialog]).find(
      (node) => node.role === "listitem" && ui.subtreeText(node).includes(DB_NAME),
    ) ?? null
  );
}

/**
 * 读取列表行内除数据库名称外的第一条文本（即副标题）。
 * @param {object} row 列表行节点。
 * @returns {string} 副标题文本；不存在时为空串。
 */
function rowSubtitle(row) {
  const texts = flattenNodes([row])
    .filter((node) => node.tag === "#text")
    .map((node) => (node.text ?? "").trim())
    .filter((text) => text !== "" && text !== DB_NAME);
  return texts[0] ?? "";
}

/**
 * 列出列表行内的按钮节点。
 * @param {object|null} row 列表行节点。
 * @returns {object[]} 按钮节点数组；row 为空时为空数组。
 */
function rowButtons(row) {
  return row ? flattenNodes([row]).filter((node) => node.role === "button") : [];
}

/**
 * 取按钮的子树文本数组。
 * @param {object[]} buttons 按钮节点数组。
 * @returns {string[]} 按钮文本数组。
 */
function buttonTexts(buttons) {
  return buttons.map((button) => ui.subtreeText(button));
}

/**
 * 统计截图上指定 bounds 区域内符合 error 红色特征的像素数量。
 * 判定条件：r > 120 且 r > g × 1.6 且 r > b × 1.6（Vuetify error 色 #F44336 的核心像素）。
 * @param {Buffer} buffer 截图的 PNG 数据。
 * @param {{left: number, top: number, width: number, height: number}} bounds 区域（窗口物理像素）。
 * @returns {number} 命中的像素数量。
 */
function countRedPixels(buffer, bounds) {
  const png = decodePng(buffer);
  const left = Math.max(0, Math.floor(bounds.left));
  const top = Math.max(0, Math.floor(bounds.top));
  const right = Math.min(png.width, Math.ceil(bounds.left + bounds.width));
  const bottom = Math.min(png.height, Math.ceil(bounds.top + bounds.height));
  let count = 0;
  for (let y = top; y < bottom; y += 1) {
    for (let x = left; x < right; x += 1) {
      const i = (y * png.width + x) * 4;
      const r = png.data[i];
      const g = png.data[i + 1];
      const b = png.data[i + 2];
      if (r > 120 && r > g * 1.6 && r > b * 1.6) {
        count += 1;
      }
    }
  }
  return count;
}

/**
 * 读取系统剪贴板文本（仅用于验证脚本自身写入或复制的测试内容）。
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
 * 聚焦可编辑控件并以 Ctrl+A/Ctrl+C 复制其内容后读取剪贴板（用于验证输入框的值）。
 * @param {object} editable 可编辑控件节点。
 * @returns {Promise<string|null>} 控件内容；读取失败时为 null。
 */
async function readEditableValueByClipboard(editable) {
  await ui.clickNode(editable);
  await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
  await sleep(400);
  return readClipboard();
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
      return ui.findEditable(tree, "数据库名称") ?? ui.findEditable(tree, "Database Name") ? tree : null;
    },
    { timeout: 30000, label: "home page name input" },
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
  await ui.clickText("简体中文");
  await ui.waitForEditable("数据库名称", { timeout: 15000 });
  return true;
}

/**
 * 点击右下角「数据库归档与删除」按钮并等待归档管理对话框打开。
 * @returns {Promise<object>} 对话框节点。
 */
async function openArchiveDialog() {
  const button = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "aria-label", "数据库归档与删除"),
    { timeout: 10000, interval: 200, label: "「数据库归档与删除」按钮" },
  );
  await ui.clickNode(button);
  await ui.waitForDialog(ARCHIVE_DIALOG_TITLE, { timeout: 10000 });
  await sleep(900);
  return findArchiveDialog(await api.uiTree());
}

/**
 * 点击归档管理列表行内的按钮（精确匹配文本，等待按钮可用）。
 * @param {string} text 按钮文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickRowButton(text) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const row = findRow(tree);
      if (!row) {
        return null;
      }
      const hit = ui.findButton([row], text, { exact: true });
      return hit && hit.states?.disabled !== true ? hit : null;
    },
    { timeout: 10000, interval: 250, label: `归档管理行内按钮「${text}」` },
  );
  await ui.clickNode(button);
  await sleep(500);
}

/**
 * 点击指定对话框中文本匹配的按钮。
 * @param {(tree: object[]) => object|null} dialogFinder 对话框查找函数。
 * @param {string} text 按钮文本（精确匹配）。
 * @param {string} label 等待条件名（用于错误消息）。
 * @returns {Promise<object>} 被点击的按钮节点。
 */
async function clickDialogButton(dialogFinder, text, label) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = dialogFinder(tree);
      return dialog ? ui.findButton(tree, text, { root: dialog, exact: true }) : null;
    },
    { timeout: 10000, interval: 250, label },
  );
  await ui.clickNode(button);
  await sleep(400);
  return button;
}

/**
 * 在指定对话框内查找可编辑控件并覆盖输入文本。
 * @param {(tree: object[]) => object|null} dialogFinder 对话框查找函数。
 * @param {string} nameFragment 名称片段。
 * @param {string} text 要输入的文本。
 * @returns {Promise<void>} 无返回值。
 */
async function typeIntoDialog(dialogFinder, nameFragment, text) {
  await sleep(250);
  const editable = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = dialogFinder(tree);
      return dialog ? ui.findEditable([dialog], nameFragment) : null;
    },
    { timeout: 8000, interval: 200, label: `对话框输入框「${nameFragment}」` },
  );
  await ui.clickNode(editable);
  await api.postInput([api.keyClick(["Control", "a"]), api.typeText(text)]);
  await sleep(500);
}

/**
 * 聚焦首页名称输入框并覆盖输入文本。
 * @param {string} text 要输入的文本。
 * @returns {Promise<void>} 无返回值。
 */
async function typeIntoHomeName(text) {
  await ui.clickEditable("数据库名称");
  await sleep(300);
  await api.postInput([api.keyClick(["Control", "a"]), api.typeText(text)]);
  await sleep(600);
}

/**
 * 等待首页候选下拉显示「无匹配项」且没有任何候选列表项（元数据刷新完成后）。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @returns {Promise<object[]>} 满足条件时的 UI 树。
 */
async function waitForNoMatchDropdown({ timeout = 10000 } = {}) {
  return waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const candidates = ui.findByRole(tree, "listitem");
      return ui.hasText(tree, "无匹配项") && candidates.length === 0 ? tree : null;
    },
    { timeout, interval: 300, label: "候选下拉显示「无匹配项」且无候选项" },
  );
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 前置与首页");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  report.check(
    dbDirsBeforeLaunch.length === 1,
    "前置：记录启动前 user_database_set 下的数据库目录（uuid）",
    dbDirsBeforeLaunch.join(",") || "empty",
  );
  await ensureChineseUi();
  await sleep(800);
  {
    const tree = await api.uiTree();
    report.check(ui.findEditable(tree, "数据库名称") !== null, "首页显示「数据库名称」输入框");
    report.check(
      ui.findAttr(tree, "aria-label", "数据库归档与删除") !== null,
      "首页存在 aria-label=「数据库归档与删除」按钮",
    );
  }
  await snap("s0-home");

  report.section("S1 归档管理初始态（步骤 1）");
  await openArchiveDialog();
  {
    const tree = await api.uiTree();
    report.check(findArchiveDialog(tree) !== null, "步骤 1 打开对话框「归档管理」");
    const row = findRow(tree);
    report.check(row !== null, `步骤 1 列表包含数据库「${DB_NAME}」`);
    const subtitle = row ? rowSubtitle(row) : "";
    report.check(subtitle.includes("未归档"), "步骤 1 行副标题含「未归档」", subtitle || "none");
    report.check(
      /\d{1,4}\/\d{1,2}\/\d{1,4}.*\d{1,2}:\d{2}/.test(subtitle),
      "步骤 1 行副标题含最后打开时间（本地化 short 日期时间）",
      subtitle || "none",
    );
    const texts = buttonTexts(rowButtons(row));
    report.check(
      texts.length === 1 && texts[0] === "归档",
      "步骤 1 未归档行只有「归档」按钮",
      texts.join(",") || "none",
    );
  }
  await snap("s1-archive-dialog-initial");

  report.section("S2 归档与即时状态更新（步骤 2）");
  await ui.waitForGone(`数据库“${DB_NAME}”已归档`, { timeout: 2000 }).catch(() => null);
  await clickRowButton("归档");
  const archiveSnackbar = await ui
    .waitForTextStable(`数据库“${DB_NAME}”已归档`, { timeout: 10000 })
    .catch(() => null);
  report.check(archiveSnackbar !== null, `步骤 2 snackbar「数据库“${DB_NAME}”已归档」`);
  {
    const tree = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        const r = findRow(t);
        return r && rowSubtitle(r).includes("已归档") ? t : null;
      },
      { timeout: 8000, interval: 250, label: "行副标题变为「已归档」" },
    ).catch(() => null);
    report.check(tree !== null, "步骤 2 行副标题即时更新为「已归档」");
    const row = tree ? findRow(tree) : null;
    const buttons = rowButtons(row);
    const texts = buttonTexts(buttons);
    report.check(texts.includes("解除归档"), "步骤 2 出现「解除归档」按钮", texts.join(",") || "none");
    report.check(texts.includes("删除"), "步骤 2 出现「删除」按钮", texts.join(",") || "none");
    const deleteButton = buttons.find((b) => ui.subtreeText(b) === "删除");
    const unarchiveButton = buttons.find((b) => ui.subtreeText(b) === "解除归档");
    const buffer = await api.screenshot();
    const redDelete = deleteButton ? countRedPixels(buffer, deleteButton.bounds) : -1;
    const redUnarchive = unarchiveButton ? countRedPixels(buffer, unarchiveButton.bounds) : -1;
    report.check(
      redDelete >= 20,
      "步骤 2 「删除」按钮为 error 色（截图红色像素统计 ≥ 20）",
      `red=${redDelete}`,
    );
    report.check(
      redUnarchive <= 2,
      "步骤 2 「解除归档」按钮非 error 色（红色像素 ≤ 2 作为对照）",
      `red=${redUnarchive}`,
    );
  }
  await snap("s2-archived");

  report.section("S3 关闭对话框（步骤 3）");
  await clickDialogButton(findArchiveDialog, "关闭", "归档管理「关闭」按钮");
  const archiveClosed = await ui
    .waitForDialogGone(ARCHIVE_DIALOG_TITLE, { timeout: 10000 })
    .then(() => true)
    .catch(() => false);
  report.check(archiveClosed, "步骤 3 对话框关闭");
  {
    const tree = await api.uiTree();
    report.check(
      ui.findEditable(tree, "数据库名称") !== null && findArchiveDialog(tree) === null,
      "步骤 3 回到首页",
    );
  }
  await snap("s3-home-after-close");

  report.section("S4 首页候选过滤（步骤 4）");
  await typeIntoHomeName("zz");
  const noMatchTree = await waitForNoMatchDropdown({ timeout: 10000 }).catch(() => null);
  report.check(noMatchTree !== null, "步骤 4 候选列表显示「无匹配项」且无候选列表项");
  report.check(
    noMatchTree !== null && !ui.hasText(noMatchTree, DB_NAME),
    `步骤 4 归档数据库「${DB_NAME}」不再作为候选出现`,
  );
  await snap("s4-home-filter-no-match");

  report.section("S5 解除归档（步骤 5）");
  await ui.waitForGone(`数据库“${DB_NAME}”已归档`, { timeout: 2000 }).catch(() => null);
  await openArchiveDialog();
  await clickRowButton("解除归档");
  const unarchiveSnackbar = await ui
    .waitForTextStable(`数据库“${DB_NAME}”已解除归档`, { timeout: 10000 })
    .catch(() => null);
  report.check(unarchiveSnackbar !== null, `步骤 5 snackbar「数据库“${DB_NAME}”已解除归档」`);
  {
    const tree = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        const r = findRow(t);
        return r && rowSubtitle(r).includes("未归档") ? t : null;
      },
      { timeout: 8000, interval: 250, label: "行状态回到「未归档」" },
    ).catch(() => null);
    report.check(tree !== null, "步骤 5 行状态回到「未归档」");
    const row = tree ? findRow(tree) : null;
    const texts = buttonTexts(rowButtons(row));
    report.check(
      texts.length === 1 && texts[0] === "归档",
      "步骤 5 「删除」按钮消失（仅剩「归档」）",
      texts.join(",") || "none",
    );
  }
  await snap("s5-unarchived");

  report.section("S6 再次归档并打开删除对话框（步骤 6）");
  await ui.waitForGone(`数据库“${DB_NAME}”已解除归档`, { timeout: 8000 }).catch(() => null);
  await clickRowButton("归档");
  await ui.waitForTextStable(`数据库“${DB_NAME}”已归档`, { timeout: 10000 }).catch(() => null);
  await clickRowButton("删除");
  const deleteOpened = await ui
    .waitForDialog(DELETE_DIALOG_TITLE, { timeout: 10000 })
    .then(() => true)
    .catch(() => false);
  report.check(deleteOpened, "步骤 6 打开对话框「删除数据库」");
  await sleep(900);
  {
    const tree = await api.uiTree();
    const dialog = findDeleteDialog(tree);
    report.check(dialog !== null, "步骤 6 对话框「删除数据库」可见");
    report.check(
      dialog !== null &&
        ui.hasText(
          [dialog],
          `此操作将永久删除数据库“${DB_NAME}”及其全部数据，且无法恢复。`,
        ),
      "步骤 6 显示正文「此操作将永久删除数据库…且无法恢复。」",
    );
    const confirm = dialog ? ui.findButton(tree, "确认删除", { root: dialog, exact: true }) : null;
    report.check(confirm?.states?.disabled === true, "步骤 6 「确认删除」按钮禁用");
    report.check(
      dialog !== null && ui.findEditable([dialog], "请输入数据库名称以确认") !== null,
      "步骤 6 存在名称确认输入框「请输入数据库名称以确认」",
    );
    const password = dialog ? ui.findEditable([dialog], "数据库密码") : null;
    report.check(
      password !== null && password.attrs?.type === "password",
      "步骤 6 存在密码输入框「数据库密码」（type=password）",
      `type=${password?.attrs?.type ?? "none"}`,
    );
  }
  await snap("s6-delete-dialog");

  report.section("S7 F1 名称不匹配（步骤 7）");
  await typeIntoDialog(findDeleteDialog, "请输入数据库名称以确认", WRONG_DB_NAME);
  await sleep(600);
  {
    const tree = await api.uiTree();
    const dialog = findDeleteDialog(tree);
    const nameEditable = dialog ? ui.findEditable([dialog], "请输入数据库名称以确认") : null;
    const value = nameEditable ? await readEditableValueByClipboard(nameEditable) : null;
    report.check(value === WRONG_DB_NAME, "步骤 7 名称输入框值为 zz-wrong-name（剪贴板验证）", `clip=${value}`);
    const confirm = dialog ? ui.findButton(tree, "确认删除", { root: dialog, exact: true }) : null;
    report.check(confirm?.states?.disabled === true, "F1 名称不匹配时「确认删除」保持禁用");
    const inlineErrors = dialog
      ? flattenNodes([dialog]).filter(
          (node) => node.tag === "#text" && /不符|不匹配|不一致|错误|必须/.test(node.text ?? ""),
        )
      : [];
    report.check(
      inlineErrors.length === 0,
      "F1 名称不匹配时无内联错误文案",
      inlineErrors.map((node) => node.text.trim()).join(",") || "none",
    );
  }
  await snap("s7-f1-wrong-name");

  report.section("S8 F2 密码错误（步骤 8）");
  await typeIntoDialog(findDeleteDialog, "请输入数据库名称以确认", DB_NAME);
  await typeIntoDialog(findDeleteDialog, "数据库密码", WRONG_PASSWORD);
  {
    const tree = await api.uiTree();
    const dialog = findDeleteDialog(tree);
    const confirm = dialog ? ui.findButton(tree, "确认删除", { root: dialog, exact: true }) : null;
    report.check(
      confirm?.states?.disabled === false,
      "步骤 8 名称匹配且密码非空时「确认删除」可用",
    );
  }
  await snap("s8-before-confirm-wrong-password");
  await clickDialogButton(findDeleteDialog, "确认删除", "删除对话框「确认删除」（密码错误路径）");
  const wrongPasswordSnackbar = await ui
    .waitForTextStable("密码错误", { timeout: 12000 })
    .catch(() => null);
  report.check(wrongPasswordSnackbar !== null, "F2 密码错误 snackbar「密码错误」");
  const deleteClosedOnFailure = await ui
    .waitForDialogGone(DELETE_DIALOG_TITLE, { timeout: 10000 })
    .then(() => true)
    .catch(() => false);
  report.check(deleteClosedOnFailure, "F2 删除对话框在提交后关闭（提交即关闭）");
  {
    const tree = await api.uiTree();
    report.check(findArchiveDialog(tree) !== null, "F2 归档管理对话框保持打开");
    const row = findRow(tree);
    report.check(
      row !== null && rowSubtitle(row).includes("已归档"),
      "F2 数据库行仍在归档列表中（状态「已归档」）",
      row ? rowSubtitle(row) : "none",
    );
    const dirs = listUserDbDirs();
    report.check(
      dirs.length === dbDirsBeforeLaunch.length && dirs[0] === dbDirsBeforeLaunch[0],
      "F2 数据目录中数据库目录未变化",
      dirs.join(",") || "empty",
    );
  }
  await snap("s8-f2-wrong-password");

  report.section("S8b F4 取消删除（步骤 8b）");
  await clickRowButton("删除");
  await ui.waitForDialog(DELETE_DIALOG_TITLE, { timeout: 10000 });
  await sleep(900);
  {
    const tree = await api.uiTree();
    const dialog = findDeleteDialog(tree);
    const confirm = dialog ? ui.findButton(tree, "确认删除", { root: dialog, exact: true }) : null;
    report.check(
      confirm?.states?.disabled === true,
      "F4 重开删除对话框后「确认删除」禁用（输入已重置）",
    );
  }
  await snap("s8b-reopen-reset");
  await clickDialogButton(findDeleteDialog, "取消", "删除对话框「取消」");
  const cancelClosed = await ui
    .waitForDialogGone(DELETE_DIALOG_TITLE, { timeout: 10000 })
    .then(() => true)
    .catch(() => false);
  report.check(cancelClosed, "F4 「取消」后删除对话框关闭");
  {
    const tree = await api.uiTree();
    const row = findRow(tree);
    report.check(
      row !== null && rowSubtitle(row).includes("已归档"),
      "F4 数据库保留在归档列表中",
      row ? rowSubtitle(row) : "none",
    );
    const dirs = listUserDbDirs();
    report.check(
      dirs.length === dbDirsBeforeLaunch.length && dirs[0] === dbDirsBeforeLaunch[0],
      "F4 数据目录中数据库目录保留",
      dirs.join(",") || "empty",
    );
  }
  await snap("s8b-f4-cancel");

  report.section("S9 成功删除（步骤 9）");
  await clickRowButton("删除");
  await ui.waitForDialog(DELETE_DIALOG_TITLE, { timeout: 10000 });
  await sleep(900);
  await typeIntoDialog(findDeleteDialog, "请输入数据库名称以确认", DB_NAME);
  await typeIntoDialog(findDeleteDialog, "数据库密码", DB_PASSWORD);
  {
    const tree = await api.uiTree();
    const dialog = findDeleteDialog(tree);
    const confirm = dialog ? ui.findButton(tree, "确认删除", { root: dialog, exact: true }) : null;
    report.check(
      confirm?.states?.disabled === false,
      "步骤 9 正确名称与密码时「确认删除」可用",
    );
  }
  await snap("s9-before-delete");
  await clickDialogButton(findDeleteDialog, "确认删除", "删除对话框「确认删除」（成功路径）");
  const deletedSnackbar = await ui
    .waitForTextStable(`数据库“${DB_NAME}”已删除`, { timeout: 15000 })
    .catch(() => null);
  report.check(deletedSnackbar !== null, `步骤 9 snackbar「数据库“${DB_NAME}”已删除」`);
  const deleteClosedOnSuccess = await ui
    .waitForDialogGone(DELETE_DIALOG_TITLE, { timeout: 10000 })
    .then(() => true)
    .catch(() => false);
  report.check(deleteClosedOnSuccess, "步骤 9 删除对话框关闭");
  const emptyListTree = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return ui.hasText(tree, "暂无数据库") ? tree : null;
    },
    { timeout: 10000, interval: 300, label: "归档管理列表显示「暂无数据库」" },
  ).catch(() => null);
  {
    const tree = emptyListTree ?? (await api.uiTree());
    const dialog = findArchiveDialog(tree);
    report.check(dialog !== null && ui.hasText([dialog], "暂无数据库"), "步骤 9 列表该行消失（显示「暂无数据库」）");
    report.check(
      dialog !== null && ui.findByRole([dialog], "listitem").length === 0,
      "步骤 9 归档管理列表无任何列表项",
    );
    const dirs = listUserDbDirs();
    report.check(
      !dirs.includes(dbDirsBeforeLaunch[0]),
      "步骤 9 数据目录中 uuid 目录已被清除",
      dirs.join(",") || "empty",
    );
  }
  await snap("s9-deleted");

  report.section("S10 关闭与目录核对（步骤 10）");
  await clickDialogButton(findArchiveDialog, "关闭", "归档管理「关闭」按钮");
  await ui
    .waitForDialogGone(ARCHIVE_DIALOG_TITLE, { timeout: 10000 })
    .catch(() => null);
  await sleep(800);
  await typeIntoHomeName("zz");
  const noMatchAfterDelete = await waitForNoMatchDropdown({ timeout: 10000 }).catch(() => null);
  report.check(
    noMatchAfterDelete !== null,
    "步骤 10 首页候选为空（输入 zz 显示「无匹配项」且无候选列表项）",
  );
  {
    const dirs = listUserDbDirs();
    report.check(
      !dirs.includes(dbDirsBeforeLaunch[0]),
      "步骤 10 启动前记录的 uuid 目录已不存在",
      dirs.join(",") || "empty",
    );
  }
  await snap("s10-home-empty");

  report.section("S11 删除后归档管理为空（步骤 11）");
  await openArchiveDialog();
  {
    const tree = await api.uiTree();
    const dialog = findArchiveDialog(tree);
    report.check(dialog !== null, "步骤 11 重新打开归档管理对话框");
    report.check(
      dialog !== null && ui.hasText([dialog], "暂无数据库"),
      "步骤 11 已归档与未归档列表均为空（「暂无数据库」）",
    );
    report.check(
      dialog !== null && ui.findByRole([dialog], "listitem").length === 0,
      "步骤 11 列表无任何列表项",
    );
  }
  await snap("s11-empty-dialog");
  console.log(
    "  (note) F3 记录：未归档行不渲染「删除」按钮（步骤 1/5 已断言），后端 UserDatabaseMustBeArchivedBeforeDelete 无 UI 触达路径",
  );

  report.section("S12 收尾");
  {
    const tree = await api.uiTree();
    report.check(findArchiveDialog(tree) !== null, "用例结束时应用仍可交互（归档管理对话框响应正常）");
  }
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
  prepareDataDir(DATA_DIR, "base");
  dbDirsBeforeLaunch = listUserDbDirs();
  await withApp(
    { dataDir: DATA_DIR, logFile: path.join(OUTPUT_DIR, "app.log") },
    async () => {
      try {
        await main();
      } catch (error) {
        await snap("fatal").catch(() => {});
        throw error;
      }
    },
  );
} catch (error) {
  fatal = error;
  report.check(false, "用例执行中断", error.message);
} finally {
  finalizeReport(report, OUTPUT_DIR, fatal);
}
