// case_008 附件管理。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）
// → hover「账号 A」点「管理附件」打开附件对话框（空态、无孤儿警告）
// → 创建文本附件 note（自动补全 .txt）；F1 非法扩展名 bad.exe 拦截（snackbar + 标红）；
//   改 bad.md 后取消创建（错误态解除、未创建）
// → 经 FilePickerDialog 导入资产 sample.txt；再创建 a.txt、b.txt
// → 重命名 note.txt：F2 空名拦截 → 输入 note-renamed.md 提交成功
// → 拖拽排序（第一项与第三项交换）→ 关闭重开验证顺序已落库
// → 预览 note-renamed.md：语言名 Markdown / 保存禁用 → 编辑（未保存标记 + 保存可用）
//   → 保存（snackbar）→ 再编辑后关闭触发 F4 未保存确认（取消留在预览 / 确认放弃）
// → 导出 sample.txt（save 模式默认文件名）→ 断言导出文件内容等于资产
//   → 再次导出触发 F3 覆盖确认（取消不覆盖 → 确认导出成功）
// → 删除 a.txt：F5 取消保留 → 确认进入回收站（分区出现）→ 恢复回正常列表
//   → 再删除 → F5 永久删除取消保留 → 确认永久删除（分区消失）
// → 关闭对话框后伪造孤儿文件 → 重开触发孤儿警告与说明 → 删除孤儿文件（确认框 → snackbar）
//   → 断言磁盘文件消失 → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 附件对话框、预览对话框、文件选择器与确认框均为嵌套 VDialog（role=dialog 为全屏 overlay）：
//   定位时在对话框子树内查找，或用特征元素（含「路径」输入框者为文件选择器）区分；
// - 附件行的行内图标 title 相同（重命名/预览/导出/删除），按行文本的垂直中心与图标的
//   垂直中心接近（<40px）筛选后取最左者；正常列表与回收站分区按「回收站（n）」标题的
//   top 分界；
// - 重命名输入框与创建输入框为可编辑元素，错误态（标红）在 UI 树中不可见，用输入框
//   bounds 内的红色像素断言；输入框当前值不在 UI 树（value 被排除），导出默认文件名
//   用 Ctrl+A/Ctrl+C 复制后读取系统剪贴板验证；
// - 拖拽排序用 mouse_drag（内建抖动 + 停顿）从「拖拽调整顺序」手柄拖到目标行垂直中心；
// - CodeMirror 编辑区为 contenteditable，点击聚焦后 Ctrl+A 再 type 注入文本可行（已实测）；
// - 伪造孤儿文件在附件对话框关闭后写入数据目录 attachment/，重开对话框触发全局检测；
// - 全部等待使用 lib 等待函数；应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_008\case_008.js`

import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as api from "../lib/api.js";
import { withApp, prepareCaseOutput } from "../lib/app.js";
import { prepareDataDir } from "../lib/fixtures.js";
import * as image from "../lib/image.js";
import { Report, finalizeReport } from "../lib/report.js";
import * as ui from "../lib/ui.js";
import { sleep, waitForCondition } from "../lib/util.js";

const CASE_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.join(CASE_DIR, "output");
const DATA_DIR = path.join(OUTPUT_DIR, "data");

/** fixture 数据库名称与密码（见 _fixtures\README.md） */
const DB_NAME = "zz-e2e-base";
const DB_PASSWORD = "e2e-password";

/** 脚本资产：导入附件与导出内容比对使用的文本文件 */
const SAMPLE_FILE = path.join(CASE_DIR, "sample.txt");
/** 导出目标文件名 */
const EXPORT_NAME = "sample-export.txt";
/** 伪造孤儿文件的 uuid 文件名（写入数据目录 attachment/） */
const ORPHAN_ID = "11111111-2222-3333-4444-555555555555";

/** 输入框标红断言下限：输入框 bounds 内红色像素数 */
const RED_TINT_MIN = 20;

const report = new Report("case_008 附件管理");

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
 * 返回 UI 树中文本精确匹配的 #text 节点，按 top、left 升序排序。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 目标文本。
 * @returns {object[]} 命中的文本节点数组。
 */
function textNodes(tree, text) {
  return flattenNodes(tree)
    .filter((node) => node.tag === "#text" && (node.text ?? "").trim() === text)
    .sort((a, b) => a.bounds.top - b.bounds.top || a.bounds.left - b.bounds.left);
}

/**
 * 返回 UI 树中 title 属性匹配的元素节点。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} title 目标 title 值。
 * @returns {object[]} 命中的元素数组。
 */
function titledNodes(tree, title) {
  return flattenNodes(tree).filter((node) => node.attrs?.title === title && node.bounds.width > 0);
}

/**
 * 在 UI 树中查找包含指定标题文本的对话框（按「子树含精确文本」判定）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} title 对话框标题文本。
 * @returns {object|null} 命中的对话框；未命中时为 null。
 */
function dialogWithExactText(tree, title) {
  return ui.findByRole(tree, "dialog").find((d) => ui.findByText([d], title, { exact: true }) !== null) ?? null;
}

/**
 * 在 UI 树中查找附件预览对话框（特征：包含高度超过 300px 的可编辑编辑器区域）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 命中的预览对话框；未命中时为 null。
 */
function previewDialog(tree) {
  return (
    ui.findByRole(tree, "dialog").find((d) =>
      flattenNodes([d]).some((n) => n.editable && n.bounds.height > 300),
    ) ?? null
  );
}

/**
 * 在 UI 树中查找文件选择器对话框（特征：包含名称为「路径」的可编辑控件）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 命中的文件选择器对话框；未命中时为 null。
 */
function dialogWithPathInput(tree) {
  return ui.findByRole(tree, "dialog").find((d) => ui.findEditable([d], "路径") !== null) ?? null;
}

/**
 * 统计 PNG 指定区域内红色系像素数量（用于断言错误标红）。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {{left: number, top: number, width: number, height: number}} region 区域。
 * @returns {number} 红色系像素数量。
 */
function redPixelsIn(png, region) {
  let count = 0;
  const left = Math.max(0, Math.floor(region.left));
  const top = Math.max(0, Math.floor(region.top));
  const right = Math.min(png.width, Math.ceil(region.left + region.width));
  const bottom = Math.min(png.height, Math.ceil(region.top + region.height));
  for (let y = top; y < bottom; y++) {
    for (let x = left; x < right; x++) {
      const i = (png.width * y + x) << 2;
      const r = png.data[i];
      const g = png.data[i + 1];
      const b = png.data[i + 2];
      if (r > 150 && r > g + 50 && r > b + 50) count += 1;
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
 * 返回数据目录中数据库的附件文件夹路径（动态发现 user_database_set 下的唯一子目录）。
 * @returns {string} attachment 目录绝对路径。
 */
function attachmentDir() {
  const setRoot = path.join(DATA_DIR, "user_database_set");
  const dbDir = fs
    .readdirSync(setRoot, { withFileTypes: true })
    .find((entry) => entry.isDirectory());
  if (!dbDir) throw new Error(`no database directory under ${setRoot}`);
  return path.join(setRoot, dbDir.name, "attachment");
}

/**
 * 解析附件对话框的列表分区：正常列表行、回收站行、回收站标题与孤儿文件 id。
 * 行按 top 升序；正常行在「回收站（n）」标题之上，回收站行在其下。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {{normal: object[], deleted: object[], recycleTitle: object|null, orphanIds: object[]}} 分区结果。
 */
function attachmentList(tree) {
  const dialog = ui.findDialog(tree, "的附件");
  if (!dialog) return { normal: [], deleted: [], recycleTitle: null, orphanIds: [] };
  const nodes = flattenNodes([dialog]);
  const rows = nodes
    .filter(
      (node) =>
        node.tag === "#text" &&
        /\.(txt|md)$/.test((node.text ?? "").trim()) &&
        node.bounds.width > 0 &&
        node.bounds.left < 700,
    )
    .sort((a, b) => a.bounds.top - b.bounds.top);
  const recycleTitle =
    nodes.find((node) => node.tag === "#text" && /^回收站（\d+）$/.test((node.text ?? "").trim())) ?? null;
  const cut = recycleTitle ? recycleTitle.bounds.top : Number.POSITIVE_INFINITY;
  return {
    normal: rows.filter((row) => row.bounds.top < cut),
    deleted: rows.filter((row) => row.bounds.top > cut),
    recycleTitle,
    orphanIds: nodes.filter((node) => node.tag === "#text" && /^[0-9a-f]{8}-[0-9a-f-]{27}$/.test((node.text ?? "").trim())),
  };
}

/**
 * 返回附件对话框中的行文件名数组（按 top 升序）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} 正常列表文件名数组。
 */
function normalNames(tree) {
  return attachmentList(tree).normal.map((row) => (row.text ?? "").trim());
}

/**
 * 取指定附件行内指定 title 的图标按钮（按垂直中心接近筛选后取最左者）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} rowText 行文件名文本节点。
 * @param {string} title 图标 title。
 * @returns {object|null} 命中的图标元素；未命中时为 null。
 */
function iconOfRow(tree, rowText, title) {
  const cy = rowText.bounds.top + rowText.bounds.height / 2;
  const hits = titledNodes(tree, title)
    .filter((node) => Math.abs(node.bounds.top + node.bounds.height / 2 - cy) < 40)
    .sort((a, b) => a.bounds.left - b.bounds.left);
  return hits[0] ?? null;
}

/**
 * 等待正常列表出现指定文件名并返回其行文本节点。
 * @param {string} name 目标文件名。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @returns {Promise<object>} 行文件名文本节点。
 */
async function waitForRow(name, { timeout = 10000 } = {}) {
  return waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return attachmentList(tree).normal.find((row) => (row.text ?? "").trim() === name) ?? null;
    },
    { timeout, interval: 250, label: `attachment row "${name}"` },
  );
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
    { timeout: 20000, label: "home page name input" },
  );
  if (ui.findEditable(initial, "数据库名称")) return false;
  console.log("  (ui language is not Chinese; switching back via the language menu)");
  const button = ui.findButton(initial, "切换语言") ?? ui.findButton(initial, "Switch Language");
  if (!button) throw new Error("language switch button not found on home page");
  await ui.clickNode(button);
  await ui.clickText("简体中文");
  await ui.waitForEditable("数据库名称", { timeout: 10000 });
  return true;
}

/**
 * 解锁 fixture 数据库：名称步选择候选并确认，密码步输入密码确认，等待进入根画布。
 * @returns {Promise<void>} 无返回值。
 */
async function unlock() {
  await ui.clickEditable("数据库名称");
  await api.postInput([api.typeText(DB_NAME)]);
  await ui.waitForTextStable(DB_NAME, { timeout: 8000 });
  await api.postInput([api.keyClick(["Enter"])]);
  await ui.clickButton("确认");
  await ui.waitForEditable("密码", { timeout: 20000 });
  await ui.typeIntoEditable("密码", DB_PASSWORD);
  await ui.clickButton("确认");
  await ui.waitForTextStable("账号 A", { timeout: 40000, settleMs: 800 });
}

/**
 * 在画布中 hover「账号 A」并点击悬浮工具条的「管理附件」按钮打开附件对话框。
 * @returns {Promise<void>} 无返回值。
 */
async function openAttachmentDialog() {
  await api.postInput([api.mouseMove(1420, 300)]);
  await sleep(400);
  const title = await waitForCondition(
    async () => textNodes(await api.uiTree(), "账号 A")[0] ?? null,
    { timeout: 6000, interval: 200, label: "「账号 A」文本" },
  );
  const center = ui.boundsCenter(title);
  await api.postInput([api.mouseMove(center.x, center.y)]);
  await sleep(800);
  const button = await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "管理附件"), {
    timeout: 6000,
    interval: 200,
    label: "「管理附件」按钮",
  });
  await ui.clickNode(button);
  await waitForCondition(async () => ui.findDialog(await api.uiTree(), "的附件") !== null, {
    timeout: 8000,
    interval: 250,
    label: "附件对话框",
  });
  await sleep(800);
}

/**
 * 点击附件对话框内的按钮（在对话框子树内查找）。
 * @param {string} buttonText 按钮文本。
 * @param {object} [options] 可选参数。
 * @param {boolean} [options.exact=false] 是否精确匹配按钮子树文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickAttachmentButton(buttonText, { exact = false } = {}) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = ui.findDialog(tree, "的附件");
      return dialog ? ui.findButton(tree, buttonText, { root: dialog, exact }) : null;
    },
    { timeout: 8000, interval: 250, label: `附件对话框按钮「${buttonText}」` },
  );
  await ui.clickNode(button);
}

/**
 * 关闭附件对话框（点「关闭」）并等待关闭动画结束。
 * @returns {Promise<void>} 无返回值。
 */
async function closeAttachmentDialog() {
  await clickAttachmentButton("关闭");
  await ui.waitForDialogGone("的附件", { timeout: 8000 });
  await sleep(700);
}

/**
 * 点击「创建文本附件」进入创建态并返回文件名输入框。
 * @returns {Promise<object>} 文件名输入框节点。
 */
async function startCreate() {
  await clickAttachmentButton("创建文本附件");
  return waitForCondition(async () => ui.findEditable(await api.uiTree(), "文件名（如 note.txt）"), {
    timeout: 6000,
    interval: 200,
    label: "创建文本附件输入框",
  });
}

/**
 * 点击「确认创建」提交创建态。
 * @returns {Promise<void>} 无返回值。
 */
async function submitCreate() {
  const button = await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "确认创建"), {
    timeout: 6000,
    interval: 200,
    label: "「确认创建」按钮",
  });
  await ui.clickNode(button);
}

/**
 * 创建文本附件：进入创建态输入文件名并确认，等待 snackbar 与目标文件名出现在正常列表。
 * @param {string} typedName 输入的文件名。
 * @param {string} expectedName 期望出现在列表中的文件名。
 * @returns {Promise<object|null>} 观察到的「附件已创建」snackbar 节点；未观察到时为 null。
 */
async function createTextAttachment(typedName, expectedName) {
  await ui.waitForGone("附件已创建", { timeout: 6000 }).catch(() => null);
  const input = await startCreate();
  await ui.clickNode(input);
  await api.postInput([api.typeText(typedName)]);
  await sleep(400);
  await submitCreate();
  const snackbar = await ui.waitForTextStable("附件已创建", { timeout: 8000 }).catch(() => null);
  await waitForRow(expectedName, { timeout: 10000 });
  await sleep(700);
  return snackbar;
}

/**
 * 在文件选择器中输入目录路径、可选文件名，并点击「确认」。
 * @param {string} dir 目录绝对路径（路径框内容）。
 * @param {string|null} fileName 目标文件名；null 表示 open 模式不填写文件名。
 * @returns {Promise<void>} 无返回值。
 */
async function fillPickerAndConfirm(dir, fileName) {
  const picker = await waitForCondition(async () => dialogWithPathInput(await api.uiTree()), {
    timeout: 8000,
    interval: 250,
    label: "文件选择器",
  });
  await sleep(600);
  await ui.clickNode(ui.findEditable([picker], "路径"));
  await api.postInput([api.keyClick(["Control", "a"]), api.typeText(dir), api.keyClick(["Enter"])]);
  await sleep(1200);
  if (fileName !== null) {
    const tree = await api.uiTree();
    const fresh = dialogWithPathInput(tree);
    await ui.clickNode(ui.findEditable([fresh], "文件名"));
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText(fileName)]);
    await sleep(400);
  }
  const tree = await api.uiTree();
  const fresh = dialogWithPathInput(tree) ?? dialogWithPathInput(await api.uiTree());
  await ui.clickNode(ui.findButton(tree, "确认", { root: fresh, exact: true }));
  await sleep(600);
}

/**
 * 点击确认框（ConfirmDialog）内的按钮。
 * @param {string} title 确认框标题。
 * @param {string} buttonText 按钮文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickConfirmDialogButton(title, buttonText) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = dialogWithExactText(tree, title);
      return dialog ? ui.findButton(tree, buttonText, { root: dialog, exact: true }) : null;
    },
    { timeout: 8000, interval: 250, label: `确认框「${title}」按钮「${buttonText}」` },
  );
  await ui.clickNode(button);
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 前置与解锁（前置条件）");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  report.check(fs.existsSync(SAMPLE_FILE), "脚本资产 sample.txt 存在", SAMPLE_FILE);
  await ensureChineseUi();
  await unlock();
  const tree0 = await api.uiTree();
  report.check(ui.hasText(tree0, "账号 A"), "解锁后进入根画布并出现节点「账号 A」");
  await sleep(600);
  await snap("s0-unlocked-root");

  report.section("S1 打开附件对话框与空态（步骤 1）");
  await openAttachmentDialog();
  let tree = await api.uiTree();
  {
    const dialogText = ui.findDialog(tree, "的附件") ? ui.subtreeText(ui.findDialog(tree, "的附件")) : "";
    report.check(tree !== null && dialogText.includes('节点"账号 A"的附件'), "步骤 1 打开附件对话框，标题为「节点\"账号 A\"的附件」", dialogText.slice(0, 60));
    report.check(ui.hasText(tree, "暂无附件，点击下方按钮导入"), "步骤 1 空态提示「暂无附件，点击下方按钮导入」");
    report.check(!ui.hasText(tree, "孤儿文件"), "步骤 1 初始无孤儿文件警告");
    report.check(
      ui.findButton(tree, "导入附件", { root: ui.findDialog(tree, "的附件") }) !== null &&
        ui.findButton(tree, "创建文本附件", { root: ui.findDialog(tree, "的附件") }) !== null,
      "步骤 1 对话框含「导入附件」「创建文本附件」按钮",
    );
  }
  await snap("s1-attachment-dialog-empty");

  report.section("S2 创建文本附件（步骤 2）");
  {
    const created = await createTextAttachment("note", "note.txt");
    report.check(created !== null, "步骤 2 创建成功提示「附件已创建」");
    tree = await api.uiTree();
    report.check(
      normalNames(tree).join(",") === "note.txt",
      "步骤 2 输入 note 后列表出现 note.txt（无扩展名自动补全 .txt）",
      normalNames(tree).join(","),
    );
    await snap("s2-note-created");
  }

  report.section("S3 F1 非法扩展名拦截（步骤 3）");
  {
    const input = await startCreate();
    await ui.clickNode(input);
    await api.postInput([api.typeText("bad.exe")]);
    await sleep(400);
    await submitCreate();
    const f1 = await ui
      .waitForTextStable("文件名必须使用文本类型的扩展名（如 .txt、.md）", { timeout: 8000 })
      .catch(() => null);
    report.check(f1 !== null, "F1 非法扩展名提交提示「文件名必须使用文本类型的扩展名（如 .txt、.md）」");
    await sleep(500);
    tree = await api.uiTree();
    report.check(normalNames(tree).join(",") === "note.txt", "F1 未创建附件（列表仍只有 note.txt）", normalNames(tree).join(","));
    {
      const input2 = ui.findEditable(tree, "文件名（如 note.txt）");
      report.check(input2 !== null, "F1 创建输入框保持输入态");
      const png = image.decodePng(await api.screenshot());
      const red = input2 ? redPixelsIn(png, input2.bounds) : 0;
      report.check(red >= RED_TINT_MIN, "F1 创建输入框标红（像素断言）", `redPixels=${red}`);
      await snap("s3-f1-bad-exe");
    }
  }

  report.section("S4 取消创建（步骤 4）");
  {
    tree = await api.uiTree();
    const input = ui.findEditable(tree, "文件名（如 note.txt）");
    await ui.clickNode(input);
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText("bad.md")]);
    await sleep(400);
    const cancel = await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "取消创建"), {
      timeout: 6000,
      interval: 200,
      label: "「取消创建」按钮",
    });
    await ui.clickNode(cancel);
    await waitForCondition(
      async () => ui.findEditable(await api.uiTree(), "文件名（如 note.txt）") === null,
      { timeout: 6000, interval: 200, label: "创建输入框消失" },
    );
    await sleep(600);
    tree = await api.uiTree();
    report.check(ui.findAttr(tree, "title", "取消创建") === null, "步骤 4 取消后创建输入态消失");
    report.check(
      ui.findButton(tree, "创建文本附件", { root: ui.findDialog(tree, "的附件") }) !== null,
      "步骤 4 恢复「创建文本附件」按钮态（错误态解除）",
    );
    report.check(normalNames(tree).join(",") === "note.txt", "步骤 4 输入 bad.md 取消后未创建附件", normalNames(tree).join(","));
    await snap("s4-after-cancel");
  }

  report.section("S5 导入附件（步骤 5-6）");
  {
    await clickAttachmentButton("导入附件");
    await waitForCondition(async () => dialogWithPathInput(await api.uiTree()) !== null, {
      timeout: 8000,
      interval: 250,
      label: "导入附件文件选择器",
    });
    await sleep(900);
    tree = await api.uiTree();
    {
      const picker = dialogWithPathInput(tree);
      report.check(picker !== null, "步骤 5 打开文件选择器");
      report.check(ui.hasText(tree, "导入附件"), "步骤 5 文件选择器标题「导入附件」");
      report.check(ui.findEditable([picker], "路径") !== null, "步骤 5 路径框（label「路径」）可输入");
      await snap("s5-file-picker-import");
    }
    await ui.clickNode(ui.findEditable([dialogWithPathInput(tree)], "路径"));
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText(CASE_DIR), api.keyClick(["Enter"])]);
    await sleep(1200);
    const sampleText = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        const picker = dialogWithPathInput(t);
        return picker ? ui.findByText([picker], "sample.txt", { exact: true }) : null;
      },
      { timeout: 10000, interval: 250, label: "文件选择器列出 sample.txt" },
    );
    await snap("s5b-file-picker-listed");
    report.check(sampleText !== null, "步骤 6 输入资产目录路径后列出 sample.txt");
    await ui.clickNode(sampleText, { count: 2 });
    const imported = await ui.waitForTextStable("附件已导入", { timeout: 10000 }).catch(() => null);
    report.check(imported !== null, "步骤 6 双击 sample.txt 后提示「附件已导入」");
    await waitForRow("sample.txt", { timeout: 10000 });
    await sleep(800);
    report.check(normalNames(await api.uiTree()).join(",") === "note.txt,sample.txt", "步骤 6 列表出现 sample.txt");
    await snap("s6-sampled-imported");
  }

  report.section("S6 创建 a.txt 与 b.txt（步骤 7）");
  {
    const createdA = await createTextAttachment("a.txt", "a.txt");
    const createdB = await createTextAttachment("b.txt", "b.txt");
    report.check(createdA !== null && createdB !== null, "步骤 7 两次创建均提示「附件已创建」");
  }
  tree = await api.uiTree();
  report.check(
    normalNames(tree).join(",") === "note.txt,sample.txt,a.txt,b.txt",
    "步骤 7 列表共 4 个附件（note.txt / sample.txt / a.txt / b.txt）",
    normalNames(tree).join(","),
  );
  await snap("s7-four-attachments");

  report.section("S7 重命名与 F2 空名拦截（步骤 8-10）");
  {
    const noteRow = await waitForRow("note.txt");
    await ui.clickNode(iconOfRow(await api.uiTree(), noteRow, "重命名附件"));
    await waitForCondition(
      async () => flattenNodes(await api.uiTree()).some((n) => n.role === "textbox" && n.tag === "INPUT" && n.states?.focused) ?? false,
      { timeout: 6000, interval: 200, label: "重命名输入框聚焦" },
    );
    await sleep(400);
    tree = await api.uiTree();
    const renameInput = flattenNodes(tree).find((n) => n.role === "textbox" && n.tag === "INPUT" && n.states?.focused);
    report.check(renameInput !== null, "步骤 8 点击重命名图标后文件名变为内联输入框");
    await snap("s8-rename-editing");
    // F2：清空后 Enter
    await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Delete"]), api.keyClick(["Enter"])]);
    const f2 = await ui.waitForTextStable("文件名不能为空", { timeout: 8000 }).catch(() => null);
    report.check(f2 !== null, "F2 空文件名提交提示「文件名不能为空」");
    await sleep(500);
    {
      const png = image.decodePng(await api.screenshot());
      const red = redPixelsIn(png, renameInput.bounds);
      report.check(red >= RED_TINT_MIN, "F2 重命名输入框标红（像素断言）", `redPixels=${red}`);
      await snap("s9-f2-rename-empty");
    }
    // 步骤 10：输入新名提交
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText("note-renamed.md"), api.keyClick(["Enter"])]);
    const renamed = await ui.waitForTextStable("附件已重命名", { timeout: 8000 }).catch(() => null);
    report.check(renamed !== null, "步骤 10 重命名提交提示「附件已重命名」");
    await waitForRow("note-renamed.md", { timeout: 10000 });
    await sleep(800);
    tree = await api.uiTree();
    report.check(
      normalNames(tree).join(",") === "note-renamed.md,sample.txt,a.txt,b.txt",
      "步骤 10 列表显示 note-renamed.md 且无 note.txt",
      normalNames(tree).join(","),
    );
    await snap("s10-renamed");
  }

  report.section("S8 拖拽排序（步骤 11）");
  {
    tree = await api.uiTree();
    const before = normalNames(tree);
    const handles = titledNodes(tree, "拖拽调整顺序");
    const thirdRow = attachmentList(tree).normal[2];
    report.check(before.length === 4 && handles.length === 4, "步骤 11 前存在 4 行与 4 个拖拽手柄", `rows=${before.length} handles=${handles.length}`);
    await api.postInput([api.mouseDrag(ui.boundsCenter(handles[0]), ui.boundsCenter(thirdRow))]);
    await sleep(1500);
    tree = await api.uiTree();
    const after = normalNames(tree);
    report.check(
      after.join(",") === "a.txt,sample.txt,note-renamed.md,b.txt",
      "步骤 11 第一项与第三项交换位置（note-renamed.md 移到原第三项位置）",
      after.join(","),
    );
    await snap("s11-after-sort");
    // 关闭重开验证落库
    await closeAttachmentDialog();
    await sleep(500);
    await openAttachmentDialog();
    tree = await api.uiTree();
    report.check(
      normalNames(tree).join(",") === "a.txt,sample.txt,note-renamed.md,b.txt",
      "步骤 11 刷新（关闭重开）后顺序保持（排序已落库）",
      normalNames(tree).join(","),
    );
    await snap("s11b-sort-persisted");
  }

  report.section("S9 预览与文本编辑保存（步骤 12-14）");
  {
    const row = await waitForRow("note-renamed.md");
    await ui.clickNode(iconOfRow(await api.uiTree(), row, "预览附件"));
    await waitForCondition(async () => previewDialog(await api.uiTree()) !== null, {
      timeout: 10000,
      interval: 250,
      label: "预览对话框",
    });
    await sleep(1200);
    tree = await api.uiTree();
    {
      const preview = previewDialog(tree);
      report.check(preview !== null, "步骤 12 点击预览打开预览对话框（标题 note-renamed.md）");
      report.check(ui.hasText(tree, "note-renamed.md"), "步骤 12 预览对话框标题为 note-renamed.md");
      report.check(ui.hasText(tree, "Markdown"), "步骤 12 文本查看器工具栏显示语言名 Markdown");
      const saveBtn = ui.findButton(tree, "保存", { root: preview, exact: true });
      report.check(saveBtn?.states?.disabled === true, "步骤 12 保存按钮初始禁用");
      const cm = flattenNodes(tree).find((n) => n.editable && n.bounds.height > 300);
      report.check(cm !== null, "步骤 12 存在可编辑的 CodeMirror 编辑区");
      await snap("s12-preview-open");
      // 步骤 13：编辑
      await ui.clickNode(cm);
      await sleep(500);
      await api.postInput([api.keyClick(["Control", "a"])]);
      await sleep(300);
      await api.postInput([api.typeText("e2e attachment edited")]);
      const unsaved = await ui.waitForTextStable("未保存", { timeout: 8000 }).catch(() => null);
      report.check(unsaved !== null, "步骤 13 编辑后工具栏出现「未保存」标记");
      tree = await api.uiTree();
      {
        const preview2 = previewDialog(tree);
        const saveBtn2 = ui.findButton(tree, "保存", { root: preview2, exact: true });
        report.check(saveBtn2?.states?.disabled === false, "步骤 13 「保存」按钮变为可用");
      }
      await snap("s13-preview-edited");
    }
    // 步骤 14：保存
    {
      tree = await api.uiTree();
      const preview = previewDialog(tree);
      await ui.clickNode(ui.findButton(tree, "保存", { root: preview, exact: true }));
    }
    const saved = await ui.waitForTextStable("附件已保存", { timeout: 10000 }).catch(() => null);
    report.check(saved !== null, "步骤 14 点击保存提示「附件已保存」");
    await sleep(800);
    tree = await api.uiTree();
    report.check(!ui.hasText(tree, "未保存"), "步骤 14 「未保存」标记消失");
    await snap("s14-preview-saved");

    report.section("S10 未保存关闭确认 F4（步骤 15-16）");
    {
      const cm = flattenNodes(tree).find((n) => n.editable && n.bounds.height > 300);
      await ui.clickNode(cm);
      await sleep(400);
      await api.postInput([api.typeText(" pending")]);
      await ui.waitForTextStable("未保存", { timeout: 8000 }).catch(() => null);
      await sleep(400);
      tree = await api.uiTree();
      const preview = previewDialog(tree);
      await ui.clickNode(ui.findButton(tree, "关闭", { root: preview, exact: true }));
    }
    const unsavedDialog = await waitForCondition(
      async () => dialogWithExactText(await api.uiTree(), "未保存的修改"),
      { timeout: 8000, interval: 250, label: "「未保存的修改」确认框" },
    ).catch(() => null);
    report.check(unsavedDialog !== null, "步骤 15 关闭时弹出确认框「未保存的修改」");
    await sleep(600);
    tree = await api.uiTree();
    report.check(
      ui.hasText(tree, "当前附件有未保存的修改，关闭后将丢失，确定要关闭吗？"),
      "步骤 15 确认框正文与预期一致",
    );
    await snap("s15-unsaved-confirm");
    await clickConfirmDialogButton("未保存的修改", "取消");
    await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "未保存的修改") === null, {
      timeout: 8000,
      interval: 250,
      label: "确认框关闭",
    });
    await sleep(800);
    tree = await api.uiTree();
    report.check(previewDialog(tree) !== null, "步骤 16 点「取消」后留在预览对话框");
    await snap("s16-f4-cancel-stay");
    // 再次关闭并确认
    {
      const preview = previewDialog(tree);
      await ui.clickNode(ui.findButton(tree, "关闭", { root: preview, exact: true }));
    }
    await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "未保存的修改") !== null, {
      timeout: 8000,
      interval: 250,
      label: "「未保存的修改」确认框（二次）",
    });
    await sleep(500);
    await clickConfirmDialogButton("未保存的修改", "确认");
    await waitForCondition(async () => previewDialog(await api.uiTree()) === null, {
      timeout: 8000,
      interval: 250,
      label: "预览对话框关闭",
    });
    await sleep(800);
    tree = await api.uiTree();
    report.check(ui.findDialog(tree, "的附件") !== null, "步骤 16 点「确认」后预览对话框关闭并放弃修改（返回附件对话框）");
    await snap("s16b-preview-closed");
  }

  report.section("S11 导出与覆盖确认 F3（步骤 17-19）");
  {
    const row = await waitForRow("sample.txt");
    await ui.clickNode(iconOfRow(await api.uiTree(), row, "导出附件"));
    const picker = await waitForCondition(async () => dialogWithPathInput(await api.uiTree()), {
      timeout: 8000,
      interval: 250,
      label: "导出附件文件选择器",
    });
    await sleep(900);
    tree = await api.uiTree();
    report.check(picker !== null, "步骤 17 打开文件选择器（save 模式）");
    report.check(ui.hasText(tree, "导出附件"), "步骤 17 文件选择器标题「导出附件」");
    {
      const nameInput = ui.findEditable([picker], "文件名");
      report.check(nameInput !== null, "步骤 17 存在文件名输入框");
      await snap("s17-export-picker");
      await ui.clickNode(nameInput);
      await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
      await sleep(400);
      const clip = readClipboard();
      report.check(clip === "sample.txt", "步骤 17 默认文件名为 sample.txt（剪贴板验证）", `clip=${clip}`);
    }
    // 路径 + 文件名 + 确认
    await ui.clickNode(ui.findEditable([dialogWithPathInput(tree)], "路径"));
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText(OUTPUT_DIR), api.keyClick(["Enter"])]);
    await sleep(1200);
    {
      tree = await api.uiTree();
      const fresh = dialogWithPathInput(tree);
      await ui.clickNode(ui.findEditable([fresh], "文件名"));
      await api.postInput([api.keyClick(["Control", "a"]), api.typeText(EXPORT_NAME)]);
      await sleep(400);
      await snap("s18-export-filled");
      const treeFilled = await api.uiTree();
      const freshFilled = dialogWithPathInput(treeFilled);
      await ui.clickNode(ui.findButton(treeFilled, "确认", { root: freshFilled, exact: true }));
    }
    const exported = await ui.waitForTextStable("附件已导出", { timeout: 10000 }).catch(() => null);
    report.check(exported !== null, "步骤 18 导出成功提示「附件已导出」");
    await sleep(800);
    {
      const exportPath = path.join(OUTPUT_DIR, EXPORT_NAME);
      const exists = fs.existsSync(exportPath);
      const same = exists && fs.readFileSync(exportPath).equals(fs.readFileSync(SAMPLE_FILE));
      report.check(exists, "步骤 18 导出文件存在", exportPath);
      report.check(same, "步骤 18 导出文件内容等于资产 sample.txt");
    }
    await snap("s18b-exported");

    // 步骤 19 + F3：再次导出 → 覆盖确认 → 取消 → 再确认
    const row2 = await waitForRow("sample.txt");
    await ui.clickNode(iconOfRow(await api.uiTree(), row2, "导出附件"));
    await fillPickerAndConfirm(OUTPUT_DIR, EXPORT_NAME);
    const overwrite = await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "文件已存在"), {
      timeout: 8000,
      interval: 250,
      label: "覆盖确认框",
    });
    report.check(overwrite !== null, "步骤 19 再次导出弹出覆盖确认「文件已存在」");
    await sleep(600);
    tree = await api.uiTree();
    report.check(ui.hasText(tree, '"sample-export.txt" 已存在，是否覆盖？'), "步骤 19 覆盖确认正文与预期一致");
    await snap("s19-overwrite-confirm");
    await clickConfirmDialogButton("文件已存在", "取消");
    await sleep(900);
    tree = await api.uiTree();
    report.check(dialogWithPathInput(tree) !== null, "F3 点「取消」后文件选择器保持打开");
    {
      const exportPath = path.join(OUTPUT_DIR, EXPORT_NAME);
      report.check(
        fs.readFileSync(exportPath).equals(fs.readFileSync(SAMPLE_FILE)),
        "F3 点「取消」后原文件未被覆盖（内容不变）",
      );
    }
    await snap("s19b-f3-cancel");
    // 再点确认 → 覆盖框 → 确认
    {
      const fresh = dialogWithPathInput(tree);
      await ui.clickNode(ui.findButton(tree, "确认", { root: fresh, exact: true }));
    }
    await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "文件已存在") !== null, {
      timeout: 8000,
      interval: 250,
      label: "覆盖确认框（二次）",
    });
    await sleep(500);
    await clickConfirmDialogButton("文件已存在", "确认");
    const exported2 = await ui.waitForTextStable("附件已导出", { timeout: 10000 }).catch(() => null);
    report.check(exported2 !== null, "步骤 19 覆盖确认点「确认」后导出成功");
    await sleep(800);
    {
      const exportPath = path.join(OUTPUT_DIR, EXPORT_NAME);
      report.check(
        fs.readFileSync(exportPath).equals(fs.readFileSync(SAMPLE_FILE)),
        "步骤 19 覆盖后导出文件内容仍等于资产 sample.txt",
      );
    }
    await snap("s19c-overwrite-exported");
  }

  report.section("S12 删除·恢复·永久删除 F5（步骤 20-23）");
  {
    const row = await waitForRow("a.txt");
    await ui.clickNode(iconOfRow(await api.uiTree(), row, "删除附件"));
    const confirm = await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "删除附件"), {
      timeout: 8000,
      interval: 250,
      label: "删除附件确认框",
    });
    report.check(confirm !== null, "步骤 20 弹出确认框「删除附件」");
    await sleep(600);
    tree = await api.uiTree();
    report.check(ui.hasText(tree, '确定要删除附件"a.txt"吗？它将移入回收站。'), "步骤 20 删除确认正文与预期一致");
    await snap("s20-remove-confirm");
    // F5：取消
    await clickConfirmDialogButton("删除附件", "取消");
    await sleep(900);
    tree = await api.uiTree();
    report.check(normalNames(tree).includes("a.txt"), "F5 点「取消」后 a.txt 保留在正常列表");
    report.check(attachmentList(tree).recycleTitle === null, "F5 点「取消」后无回收站分区");
    await snap("s20b-f5-cancel-kept");
    // 再次删除并确认
    {
      const fresh = await waitForRow("a.txt");
      await ui.clickNode(iconOfRow(await api.uiTree(), fresh, "删除附件"));
    }
    await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "删除附件") !== null, {
      timeout: 8000,
      interval: 250,
      label: "删除附件确认框（二次）",
    });
    await sleep(500);
    await clickConfirmDialogButton("删除附件", "确认");
    const removed = await ui.waitForTextStable("附件已删除", { timeout: 10000 }).catch(() => null);
    report.check(removed !== null, "步骤 21 确认删除提示「附件已删除」");
    await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return !normalNames(t).includes("a.txt") && attachmentList(t).deleted.some((r) => (r.text ?? "").trim() === "a.txt");
      },
      { timeout: 8000, interval: 250, label: "a.txt 移入回收站" },
    );
    await sleep(800);
    tree = await api.uiTree();
    report.check(!normalNames(tree).includes("a.txt"), "步骤 21 正常列表无 a.txt");
    report.check(ui.hasText(tree, "回收站（1）"), "步骤 21 出现分区「回收站（1）」");
    await snap("s21-a-in-recycle-bin");

    // 步骤 22：恢复
    {
      const restoreIcon = await waitForCondition(async () => titledNodes(await api.uiTree(), "恢复附件")[0] ?? null, {
        timeout: 6000,
        interval: 200,
        label: "「恢复附件」图标",
      });
      await ui.clickNode(restoreIcon);
    }
    const restored = await ui.waitForTextStable("附件已恢复", { timeout: 10000 }).catch(() => null);
    report.check(restored !== null, "步骤 22 恢复提示「附件已恢复」");
    await waitForRow("a.txt", { timeout: 10000 });
    await sleep(800);
    tree = await api.uiTree();
    report.check(normalNames(tree).includes("a.txt"), "步骤 22 a.txt 回到正常列表");
    report.check(attachmentList(tree).recycleTitle === null, "步骤 22 回收站分区消失");
    await snap("s22-a-restored");

    // 步骤 23：再次删除 → 永久删除（F5 取消 + 确认）
    {
      const fresh = await waitForRow("a.txt");
      await ui.clickNode(iconOfRow(await api.uiTree(), fresh, "删除附件"));
    }
    await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "删除附件") !== null, {
      timeout: 8000,
      interval: 250,
      label: "删除附件确认框（三次）",
    });
    await sleep(500);
    await clickConfirmDialogButton("删除附件", "确认");
    await ui.waitForTextStable("附件已删除", { timeout: 10000 }).catch(() => null);
    await sleep(900);
    {
      const physicalIcon = await waitForCondition(async () => titledNodes(await api.uiTree(), "永久删除附件")[0] ?? null, {
        timeout: 6000,
        interval: 200,
        label: "「永久删除附件」图标",
      });
      await ui.clickNode(physicalIcon);
    }
    const physical = await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "永久删除附件"), {
      timeout: 8000,
      interval: 250,
      label: "永久删除确认框",
    });
    report.check(physical !== null, "步骤 23 弹出确认框「永久删除附件」");
    await sleep(600);
    tree = await api.uiTree();
    report.check(
      ui.hasText(tree, '确定要永久删除附件"a.txt"吗？附件文件将一并删除，此操作不可恢复。'),
      "步骤 23 永久删除确认正文与预期一致",
    );
    await snap("s23-physical-confirm");
    await clickConfirmDialogButton("永久删除附件", "取消");
    await sleep(900);
    tree = await api.uiTree();
    report.check(ui.hasText(tree, "回收站（1）"), "F5 永久删除点「取消」后附件保留在回收站");
    await snap("s23b-f5-physical-cancel");
    {
      const physicalIcon = await waitForCondition(async () => titledNodes(await api.uiTree(), "永久删除附件")[0] ?? null, {
        timeout: 6000,
        interval: 200,
        label: "「永久删除附件」图标（二次）",
      });
      await ui.clickNode(physicalIcon);
    }
    await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "永久删除附件") !== null, {
      timeout: 8000,
      interval: 250,
      label: "永久删除确认框（二次）",
    });
    await sleep(500);
    await clickConfirmDialogButton("永久删除附件", "确认");
    const physicallyRemoved = await ui.waitForTextStable("附件已删除", { timeout: 10000 }).catch(() => null);
    report.check(physicallyRemoved !== null, "步骤 23 永久删除后提示（文案「附件已删除」）");
    await waitForCondition(
      async () => {
        const t = await api.uiTree();
        const list = attachmentList(t);
        return list.recycleTitle === null && list.deleted.length === 0;
      },
      { timeout: 8000, interval: 250, label: "回收站分区消失" },
    );
    await sleep(800);
    tree = await api.uiTree();
    report.check(attachmentList(tree).recycleTitle === null, "步骤 23 回收站分区消失");
    report.check(!ui.hasText(tree, "a.txt"), "步骤 23 永久删除后列表不含 a.txt");
    await snap("s23c-physical-deleted");
  }

  report.section("S13 孤儿文件检测与删除（步骤 24-26）");
  {
    await closeAttachmentDialog();
    const orphanPath = path.join(attachmentDir(), `${ORPHAN_ID}.bin`);
    fs.writeFileSync(orphanPath, Buffer.from("orphan payload"));
    report.check(fs.existsSync(orphanPath), "步骤 24 已向数据目录附件文件夹写入伪造孤儿文件", orphanPath);
    await sleep(500);
    await openAttachmentDialog();
    tree = await api.uiTree();
    report.check(ui.hasText(tree, "发现 1 个孤儿文件"), "步骤 24 出现警告「发现 1 个孤儿文件」");
    report.check(
      ui.hasText(tree, "以下附件文件没有对应的元数据记录，可能是异常中断留下的残留，确认无用后可删除。"),
      "步骤 24 孤儿警告说明文案与预期一致",
    );
    report.check(
      attachmentList(tree).orphanIds.some((n) => (n.text ?? "").trim() === ORPHAN_ID),
      "步骤 24 下方列出孤儿文件 id",
      ORPHAN_ID,
    );
    await snap("s24-orphan-alert");
    {
      const removeIcon = await waitForCondition(async () => titledNodes(await api.uiTree(), "删除孤儿文件")[0] ?? null, {
        timeout: 6000,
        interval: 200,
        label: "「删除孤儿文件」图标",
      });
      await ui.clickNode(removeIcon);
    }
    const orphanConfirm = await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "删除孤儿文件"), {
      timeout: 8000,
      interval: 250,
      label: "删除孤儿文件确认框",
    });
    report.check(orphanConfirm !== null, "步骤 25 弹出确认框「删除孤儿文件」");
    await sleep(600);
    tree = await api.uiTree();
    report.check(
      ui.hasText(tree, `确定要删除孤儿文件"${ORPHAN_ID}"吗？该文件将被永久删除，不可恢复。`),
      "步骤 25 孤儿删除确认正文与预期一致",
    );
    await snap("s25-orphan-confirm");
    await clickConfirmDialogButton("删除孤儿文件", "确认");
    const orphanRemoved = await ui.waitForTextStable("孤儿文件已删除", { timeout: 10000 }).catch(() => null);
    report.check(orphanRemoved !== null, "步骤 26 删除成功提示「孤儿文件已删除」");
    await ui.waitForGone("发现 1 个孤儿文件", { timeout: 8000 }).catch(() => null);
    await sleep(600);
    tree = await api.uiTree();
    report.check(!ui.hasText(tree, "发现 1 个孤儿文件"), "步骤 26 警告区消失");
    report.check(!fs.existsSync(orphanPath), "步骤 26 磁盘上孤儿文件已不存在");
    await snap("s26-orphan-removed");
  }
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
  prepareDataDir(DATA_DIR, "base");
  await withApp({ dataDir: DATA_DIR, logFile: path.join(OUTPUT_DIR, "app.log") }, async () => {
    try {
      await main();
    } catch (error) {
      await snap("fatal").catch(() => {});
      throw error;
    }
  });
} catch (error) {
  fatal = error;
  report.check(false, "用例执行中断", error.message);
} finally {
  finalizeReport(report, OUTPUT_DIR, fatal);
}
