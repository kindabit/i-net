// case_003 画布宇宙画布管理。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）→ 面包屑进入画布宇宙
// → 双击根画布进入 → 模板面板拖拽创建画布数据节点（子画布）→ 返回宇宙 → 重命名（F1 空名称
// 禁用、F2 重名 root、成功改名「子画布一」）→ 自定义颜色（蓝色预设 + 节点区域像素比对）
// → 节点拖拽移动 → 自动布局（禁用态与坐标变化）→ 滚轮缩放 → 进入画布再返回（节点尺寸保持）
// → 逻辑删除与回收站面板（徽标、恢复、F4 永久删除取消、永久删除）→ 根画布按钮组（F3 无删除
// 入口）→ 重建子画布后清空回收站 → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 断言优先使用 UI 树（文本、bounds、states、role/name），截图用于视觉留档；配色效果辅以
//   截图像素比对；配色对话框的色块以 role=button + aria-label 入树、hover 后的 tooltip
//   以 role=tooltip 节点入树（2026-09-21 修复后均基于 UI 树断言）；
// - 错误提示断言使用 waitForTextStable（等过渡静止后复查），并追加提示消失的 waitForGone
//   断言，规避 Vuetify 消息过渡期间新旧文本瞬时并存的问题；
// - 回收站面板有 v-click-outside：点击画布节点按钮或确认框按钮都会关闭面板，确认框打开期间
//   面板被遮罩从 UI 树过滤，因此面板内操作前统一调用 ensureRecycleBinPanel 重新打开；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_003\case_003.js`

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

/** 根画布在后端的真实名称常量（前端显示为本地化的「根画布」，见 src-tauri 的 initialize.rs） */
const ROOT_CANVAS_DB_NAME = "root";
/** 新建画布（画布数据节点）的默认名称 */
const NEW_CANVAS_NAME = "新画布";
/** 本用例重命名后的子画布名称 */
const CHILD_CANVAS_NAME = "子画布一";
/** 蓝色预设的亮色/暗色背景字面量（见 src\node-colors\color-presets.ts） */
const PRESET_BLUE_LIGHT = "#e3f2fdff";
const PRESET_BLUE_DARK = "#1e3a5fff";
/** 浅色主题下蓝色预设背景 #e3f2fd 的 RGB 分量 */
const PRESET_BLUE_LIGHT_RGB = { r: 227, g: 242, b: 253 };
/** 暗色主题下蓝色预设背景 #1e3a5f 的 RGB 分量 */
const PRESET_BLUE_DARK_RGB = { r: 30, g: 58, b: 95 };
/** 画布节点配色对话框预设组合的色块可访问名称（即色块 tooltip 文案，顺序与预设列表一致） */
const PRESET_SWATCH_NAMES = ["蓝", "绿", "紫", "橙", "青", "粉", "灰"];
/** 配色对话框字段名（亮色/暗色两栏相同，顺序与字段定义表一致） */
const FIELD_NAMES = ["背景", "边框", "选中边框", "标题", "图标", "工具按钮"];
/** 亮色主题字段色块的可访问名称（「亮色主题 + 字段名」） */
const LIGHT_SWATCH_NAMES = FIELD_NAMES.map((name) => `亮色主题 ${name}`);
/** 暗色主题字段色块的可访问名称（「暗色主题 + 字段名」） */
const DARK_SWATCH_NAMES = FIELD_NAMES.map((name) => `暗色主题 ${name}`);

const report = new Report("case_003 画布宇宙画布管理");

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
 * 等待首页就绪并确保界面语言为中文：首页未出现中文名称输入框时，
 * 通过右上角「切换语言」菜单切回「简体中文」。
 * @returns {Promise<boolean>} 是否执行了语言切换。
 */
async function ensureChineseUi() {
  const initial = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return ui.findEditable(tree, "数据库名称") ?? ui.findEditable(tree, "Database Name")
        ? tree
        : null;
    },
    { timeout: 20000, label: "home page name input" },
  );
  if (ui.findEditable(initial, "数据库名称")) {
    return false;
  }
  console.log("  (ui language is not Chinese; switching back via the language menu)");
  const button =
    ui.findButton(initial, "切换语言") ?? ui.findButton(initial, "Switch Language");
  if (!button) {
    throw new Error("language switch button not found on home page");
  }
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
 * 点击面包屑「画布宇宙」进入画布宇宙，并等待「根画布」画布节点出现。
 * @returns {Promise<void>} 无返回值。
 */
async function gotoUniverse() {
  await ui.clickText("画布宇宙");
  await ui.waitForText("根画布", { timeout: 10000 });
  await sleep(700);
}

/**
 * 双击「根画布」画布节点进入根画布，并等待画布内数据节点出现。
 * @returns {Promise<void>} 无返回值。
 */
async function enterRootCanvas() {
  await ui.clickText("根画布", { count: 2 });
  await ui.waitForText("账号 A", { timeout: 15000 });
  await sleep(700);
}

/**
 * 将鼠标移到指定画布节点文本中心并等待 hover 按钮组淡入。
 * @param {string} nodeText 画布节点显示文本。
 * @returns {Promise<void>} 无返回值。
 */
async function hoverCanvasNode(nodeText) {
  const tree = await api.uiTree();
  const node = ui.findByText(tree, nodeText);
  if (!node) {
    throw new Error(`画布节点「${nodeText}」不存在`);
  }
  const center = ui.boundsCenter(node);
  await api.postInput([api.mouseMove(center.x, center.y)]);
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "自定义颜色"),
    { timeout: 6000, label: `节点「${nodeText}」按钮组淡入` },
  );
  await sleep(300);
}

/**
 * hover 指定画布节点并点击其操作按钮。
 * @param {string} nodeText 画布节点显示文本。
 * @param {string} actionTitle 操作按钮的 title（如「重命名画布」）。
 * @returns {Promise<void>} 无返回值。
 */
async function clickCanvasNodeAction(nodeText, actionTitle) {
  await hoverCanvasNode(nodeText);
  const button = ui.findAttr(await api.uiTree(), "title", actionTitle);
  if (!button) {
    throw new Error(`按钮「${actionTitle}」不存在`);
  }
  await ui.clickNode(button);
}

/**
 * 在画布内通过模板面板拖拽创建一个画布数据节点（子画布）。
 * @returns {Promise<void>} 无返回值。
 */
async function dragCreateChildCanvas() {
  const tree = await api.uiTree();
  const newButton = ui.findAttr(tree, "title", "新建节点");
  if (!newButton) {
    throw new Error("「新建节点」按钮不存在");
  }
  await ui.clickNode(newButton);
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建画布数据节点"),
    { timeout: 8000, label: "模板面板展开" },
  );
  await sleep(500);
  const handle = ui.findAttr(await api.uiTree(), "title", "拖拽创建画布数据节点");
  const from = ui.boundsCenter(handle);
  const info = await api.health();
  const to = { x: Math.round(info.width * 0.6), y: Math.round(info.height * 0.55) };
  await api.postInput([api.mouseDrag(from, to)]);
  await waitForCondition(async () => ui.hasText(await api.uiTree(), NEW_CANVAS_NAME), {
    timeout: 10000,
    label: "画布数据节点创建",
  });
  await sleep(700);
}

/**
 * 确保画布回收站面板处于打开状态（已打开时不重复点击，避免 toggle 关闭）。
 * @returns {Promise<void>} 无返回值。
 */
async function ensureRecycleBinPanel() {
  const tree = await api.uiTree();
  if (!ui.findButton(tree, "清空回收站")) {
    // 对话框关闭动画期间遮罩仍会遮挡悬浮菜单，等待「回收站」按钮重新可见后再点击
    const button = await waitForCondition(
      async () => ui.findAttr(await api.uiTree(), "title", "回收站"),
      { timeout: 8000, interval: 200, label: "「回收站」按钮可见" },
    );
    await ui.clickNode(button);
    await waitForCondition(
      async () => ui.findButton(await api.uiTree(), "清空回收站"),
      { timeout: 6000, interval: 200, label: "回收站面板打开" },
    );
    await sleep(400);
  }
}

/**
 * 读取回收站徽标（role=status 的 VBadge 徽章）中的数字文本。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} 数字文本列表；无徽标时为空数组。
 */
function badgeTexts(tree) {
  return ui
    .findByRole(tree, "status")
    .map((node) =>
      ui
        .flatten([node])
        .filter((entry) => entry.node.tag === "#text")
        .map((entry) => entry.node.text ?? "")
        .join(""),
    )
    .filter((text) => /^\d+$/.test(text.trim()));
}

/**
 * 在 UI 树中查找可访问名称精确匹配的按钮（含 role=button 的配色色块）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} name 目标可访问名称（aria-label）。
 * @returns {object|null} 命中节点；未命中时为 null。
 */
function findButtonByName(tree, name) {
  return (
    ui
      .findByRole(tree, "button")
      .find((node) => node.name === name && node.bounds && node.bounds.width > 0) ?? null
  );
}

/**
 * 读取 UI 树中全部 tooltip 节点（role=tooltip）的显示文本。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} tooltip 文本列表（已去除首尾空白，过滤空串）。
 */
function tooltipTexts(tree) {
  return ui
    .findByRole(tree, "tooltip")
    .map((node) => ui.subtreeText(node).trim())
    .filter((text) => text !== "");
}

/**
 * 统计截图中指定区域内出现次数最多的颜色（RGB 众数）。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {{left: number, top: number, width: number, height: number}} region 统计区域。
 * @returns {{r: number, g: number, b: number, count: number}|null} 众数颜色；区域无效时为 null。
 */
function dominantColor(png, region) {
  const cropped = image.cropPng(png, region);
  const counts = new Map();
  for (let i = 0; i < cropped.data.length; i += 4) {
    const key = (cropped.data[i] << 16) | (cropped.data[i + 1] << 8) | cropped.data[i + 2];
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  let best = -1;
  let bestCount = -1;
  for (const [key, count] of counts) {
    if (count > bestCount) {
      best = key;
      bestCount = count;
    }
  }
  if (best < 0) {
    return null;
  }
  return { r: (best >> 16) & 255, g: (best >> 8) & 255, b: best & 255, count: bestCount };
}

/**
 * 判断两个 bounds 的位置是否发生变化（任一轴差值超过 2 像素）。
 * @param {object|undefined} a 变化前 bounds。
 * @param {object|undefined} b 变化后 bounds。
 * @returns {boolean} 位置发生变化时为 true。
 */
function boundsMoved(a, b) {
  if (!a || !b) {
    return false;
  }
  return Math.abs(a.left - b.left) > 2 || Math.abs(a.top - b.top) > 2;
}

/**
 * 判断两个 bounds 的尺寸是否一致（宽高差值均不超过 2 像素）。
 * @param {object|undefined} a 基准 bounds。
 * @param {object|undefined} b 待比较 bounds。
 * @returns {boolean} 尺寸一致时为 true。
 */
function boundsSizeKept(a, b) {
  if (!a || !b) {
    return false;
  }
  return Math.abs(a.width - b.width) <= 2 && Math.abs(a.height - b.height) <= 2;
}

/**
 * 判断颜色是否接近给定 RGB（各分量差值不超过 8）。
 * @param {{r: number, g: number, b: number}|null} color 实测颜色。
 * @param {{r: number, g: number, b: number}} expected 期望颜色。
 * @returns {boolean} 接近时为 true。
 */
function colorNear(color, expected) {
  if (!color) {
    return false;
  }
  return (
    Math.abs(color.r - expected.r) <= 8 &&
    Math.abs(color.g - expected.g) <= 8 &&
    Math.abs(color.b - expected.b) <= 8
  );
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 前置与解锁（前置条件）");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  await ensureChineseUi();
  await ui.waitForEditable("数据库名称", { timeout: 20000 });
  await unlock();
  const unlockedTree = await api.uiTree();
  report.check(ui.hasText(unlockedTree, "账号 A"), "解锁后进入根画布并出现节点「账号 A」");
  report.check(ui.hasText(unlockedTree, "备注 B"), "解锁后出现节点「备注 B」");
  report.check(ui.hasText(unlockedTree, "根画布"), "按 lastScene 恢复场景（面包屑显示「根画布」）");
  await snap("unlocked-root-canvas");

  report.section("S1 进入画布宇宙（步骤 1）");
  await gotoUniverse();
  const universeTree = await api.uiTree();
  report.check(ui.hasText(universeTree, "根画布"), "步骤 1 宇宙中出现画布节点「根画布」");
  report.check(!ui.hasText(universeTree, "账号 A"), "步骤 1 宇宙中不显示根画布内的数据节点");
  await snap("step1-universe");

  report.section("S2 进入根画布（步骤 2）");
  await enterRootCanvas();
  const canvasTree = await api.uiTree();
  report.check(ui.hasText(canvasTree, "账号 A"), "步骤 2 根画布中出现节点「账号 A」");
  report.check(ui.hasText(canvasTree, "备注 B"), "步骤 2 根画布中出现节点「备注 B」");
  await snap("step2-root-canvas");

  report.section("S3 打开模板面板（步骤 3）");
  const newNodeButton = ui.findAttr(await api.uiTree(), "title", "新建节点");
  report.check(newNodeButton !== null, "步骤 3 悬浮菜单存在按钮「新建节点」");
  await ui.clickNode(newNodeButton);
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "拖拽创建画布数据节点"),
    { timeout: 8000, label: "模板面板展开" },
  );
  await sleep(400);
  const panelTree = await api.uiTree();
  report.check(
    ui.findAttr(panelTree, "title", "拖拽创建数据节点") !== null,
    "步骤 3 模板面板出现手柄「拖拽创建数据节点」",
  );
  report.check(
    ui.findAttr(panelTree, "title", "拖拽创建画布数据节点") !== null,
    "步骤 3 模板面板出现手柄「拖拽创建画布数据节点」",
  );
  await snap("step3-template-panel");

  report.section("S4 拖拽创建子画布（步骤 4）");
  const handle = ui.findAttr(await api.uiTree(), "title", "拖拽创建画布数据节点");
  const dragFrom = ui.boundsCenter(handle);
  const dragTo = { x: Math.round(info.width * 0.6), y: Math.round(info.height * 0.55) };
  await api.postInput([api.mouseDrag(dragFrom, dragTo)]);
  await ui.waitForText(NEW_CANVAS_NAME, { timeout: 10000 });
  const panelClosed = await ui
    .waitForGone("拖拽创建画布数据节点", { timeout: 5000 })
    .catch(() => false);
  report.check(panelClosed, "步骤 4 创建成功后模板面板自动关闭");
  await snap("step4-child-canvas-created");

  report.section("S5 返回宇宙（步骤 5）");
  await gotoUniverse();
  const twoCanvasTree = await api.uiTree();
  const rootBoundsStep5 = ui.findByText(twoCanvasTree, "根画布")?.bounds;
  const childBoundsStep5 = ui.findByText(twoCanvasTree, NEW_CANVAS_NAME)?.bounds;
  report.check(
    rootBoundsStep5 !== undefined && childBoundsStep5 !== undefined,
    "步骤 5 宇宙中出现「根画布」与「新画布」两个画布节点",
    JSON.stringify({ root: rootBoundsStep5, child: childBoundsStep5 }),
  );
  await snap("step5-universe-two-canvases");

  report.section("S6 重命名对话框与空名称失败路径（步骤 6、F1）");
  await clickCanvasNodeAction(NEW_CANVAS_NAME, "重命名画布");
  await ui.waitForDialog("重命名画布", { timeout: 8000 });
  await sleep(500);
  const renameTree = await api.uiTree();
  report.check(ui.findDialog(renameTree, "重命名画布") !== null, "步骤 6 打开「重命名画布」对话框");
  report.check(ui.findEditable(renameTree, "画布名称") !== null, "步骤 6 对话框包含「画布名称」输入框");
  await ui.clickEditable("画布名称");
  await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Delete"])]);
  await sleep(400);
  const confirmDisabled = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const button = ui.findButton(tree, "确认", { root: ui.findDialog(tree, "重命名画布") });
      return button?.states?.disabled === true;
    },
    { timeout: 4000, interval: 150, label: "空名称时确认按钮禁用" },
  )
    .then(() => true)
    .catch(() => false);
  report.check(confirmDisabled, "F1 空名称时「确认」按钮保持禁用，无法提交");
  await snap("f1-rename-empty-disabled");

  report.section("S7 重名失败路径（F2）");
  await ui.clickEditable("画布名称");
  await api.postInput([api.typeText(ROOT_CANVAS_DB_NAME)]);
  await sleep(300);
  const renameDialog2 = ui.findDialog(await api.uiTree(), "重命名画布");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: renameDialog2 }));
  const dupTitle = await ui
    .waitForTextStable("画布名称已存在", { timeout: 8000 })
    .catch(() => null);
  report.check(dupTitle !== null, "F2 重名提交后 snackbar 标题「画布名称已存在」");
  const dupText = await ui
    .waitForTextStable(`名为“${ROOT_CANVAS_DB_NAME}”的画布已存在，请更换名称`, { timeout: 6000 })
    .catch(() => null);
  report.check(
    dupText !== null,
    `F2 snackbar 正文「名为“${ROOT_CANVAS_DB_NAME}”的画布已存在，请更换名称」`,
  );
  await snap("f2-rename-exists");
  const afterDupTree = await api.uiTree();
  report.check(
    ui.findDialog(afterDupTree, "重命名画布") === null,
    "F2 重名提交后对话框已关闭",
  );
  report.check(ui.hasText(afterDupTree, NEW_CANVAS_NAME), "F2 失败后画布名保持不变（仍为「新画布」）");
  const dupGone = await ui.waitForGone("画布名称已存在", { timeout: 9000 }).catch(() => false);
  report.check(dupGone, "F2 重名提示在 snackbar 超时后消失");

  report.section("S8 重命名成功（步骤 7）");
  await clickCanvasNodeAction(NEW_CANVAS_NAME, "重命名画布");
  await ui.waitForDialog("重命名画布", { timeout: 8000 });
  await sleep(500);
  await ui.typeIntoEditable("画布名称", CHILD_CANVAS_NAME);
  const renameDialog3 = ui.findDialog(await api.uiTree(), "重命名画布");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: renameDialog3 }));
  await ui.waitForTextStable(CHILD_CANVAS_NAME, { timeout: 10000, settleMs: 500 });
  const renameGone = await ui.waitForDialogGone("重命名画布", { timeout: 6000 }).catch(() => false);
  report.check(renameGone, "步骤 7 重命名成功后对话框关闭");
  const renamedTree = await api.uiTree();
  report.check(ui.hasText(renamedTree, CHILD_CANVAS_NAME), "步骤 7 宇宙中节点文本变为「子画布一」");
  report.check(!ui.hasText(renamedTree, NEW_CANVAS_NAME), "步骤 7 旧名称「新画布」已消失");
  await snap("step7-renamed");

  report.section("S9 自定义颜色与预设（步骤 8-9）");
  await clickCanvasNodeAction(CHILD_CANVAS_NAME, "自定义颜色");
  await ui.waitForDialog("自定义画布节点颜色", { timeout: 8000 });
  await sleep(600);
  const colorTree = await api.uiTree();
  const colorDialog = ui.findDialog(colorTree, "自定义画布节点颜色");
  report.check(colorDialog !== null, "步骤 8 打开「自定义画布节点颜色」对话框");
  report.check(ui.hasText(colorTree, "预设组合"), "步骤 8 对话框包含「预设组合」区块");
  report.check(
    !ui.hasText(colorTree, "历史组合"),
    "步骤 8 fixture 无历史配色，不渲染「历史组合」区块",
  );
  report.check(
    ui.findButton(colorTree, "保存", { root: colorDialog }) !== null,
    "步骤 8 对话框包含「保存」按钮",
  );
  report.check(
    ui.findButton(colorTree, "取消", { root: colorDialog }) !== null,
    "步骤 8 对话框包含「取消」按钮",
  );
  // 色块以 role=button + name（aria-label）入树：预设组合（tooltip 文案）与亮/暗字段色块
  const presetHit = PRESET_SWATCH_NAMES.filter((name) => findButtonByName(colorTree, name) !== null);
  report.check(
    presetHit.length === PRESET_SWATCH_NAMES.length,
    "步骤 8 预设组合 7 个色块以 role=button + name 入树",
    presetHit.join("/"),
  );
  const lightHit = LIGHT_SWATCH_NAMES.filter((name) => findButtonByName(colorTree, name) !== null);
  report.check(
    lightHit.length === LIGHT_SWATCH_NAMES.length,
    "步骤 8 亮色主题 6 个字段色块以 role=button + name 入树",
    lightHit.join("/"),
  );
  const darkHit = DARK_SWATCH_NAMES.filter((name) => findButtonByName(colorTree, name) !== null);
  report.check(
    darkHit.length === DARK_SWATCH_NAMES.length,
    "步骤 8 暗色主题 6 个字段色块以 role=button + name 入树",
    darkHit.join("/"),
  );
  await snap("step8-color-dialog");

  // hover 预设「蓝」色块：UI 树出现 role=tooltip 节点且文本为「蓝」
  const blueSwatch = findButtonByName(colorTree, "蓝");
  if (!blueSwatch) {
    throw new Error("预设「蓝」色块未在 UI 树中");
  }
  const blueCenter = ui.boundsCenter(blueSwatch);
  await api.postInput([api.mouseMove(blueCenter.x, blueCenter.y)]);
  const tooltipShown = await waitForCondition(
    async () => (tooltipTexts(await api.uiTree()).includes("蓝") ? true : null),
    { timeout: 6000, interval: 200, label: "预设「蓝」tooltip 出现" },
  )
    .then(() => true)
    .catch(() => false);
  report.check(tooltipShown, "步骤 8 hover「蓝」色块后 UI 树出现 role=tooltip 节点且文本为「蓝」");
  await snap("step9-swatch-hover");

  // 点击「蓝」色块应用预设
  await api.postInput([api.mouseClick("left", 1)]);
  await sleep(500);
  const presetApplied = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return ui.hasText(tree, PRESET_BLUE_LIGHT) && ui.hasText(tree, PRESET_BLUE_DARK);
    },
    { timeout: 5000, interval: 150, label: "蓝色预设色值回填" },
  )
    .then(() => true)
    .catch(() => false);
  report.check(
    presetApplied,
    "步骤 9 点击第 1 个色块后对话框背景字段值变为蓝色预设",
    `${PRESET_BLUE_LIGHT} / ${PRESET_BLUE_DARK}`,
  );

  // 鼠标移开后 tooltip 节点从 UI 树消失
  await api.postInput([api.mouseMove(300, 300)]);
  const tooltipGone = await waitForCondition(
    async () => (tooltipTexts(await api.uiTree()).includes("蓝") ? null : true),
    { timeout: 6000, interval: 200, label: "「蓝」tooltip 消失" },
  )
    .then(() => true)
    .catch(() => false);
  report.check(tooltipGone, "步骤 8 mouse 移开后「蓝」tooltip 节点从 UI 树消失");

  const colorDialog2 = ui.findDialog(await api.uiTree(), "自定义画布节点颜色");
  await ui.clickNode(ui.findButton(await api.uiTree(), "保存", { root: colorDialog2 }));
  await ui.waitForDialogGone("自定义画布节点颜色", { timeout: 8000 });
  await sleep(700);
  await api.postInput([api.mouseMove(300, 300)]);
  await sleep(500);
  const coloredTree = await api.uiTree();
  const coloredChildText = ui.findByText(coloredTree, CHILD_CANVAS_NAME);
  const colorRegion = {
    left: Math.round(coloredChildText.bounds.left - 50),
    top: Math.round(coloredChildText.bounds.top - 20),
    width: Math.round(coloredChildText.bounds.width + 100),
    height: Math.round(coloredChildText.bounds.height + 40),
  };
  const domColor = dominantColor(image.decodePng(await api.screenshot()), colorRegion);
  const colorOk =
    colorNear(domColor, PRESET_BLUE_LIGHT_RGB) || colorNear(domColor, PRESET_BLUE_DARK_RGB);
  report.check(
    colorOk,
    "步骤 9 保存后「子画布一」节点背景呈现蓝色预设（区域主色比对）",
    domColor ? `rgb(${domColor.r},${domColor.g},${domColor.b})` : "no-color",
  );
  await snap("step9-colored");

  report.section("S10 拖拽移动画布节点（步骤 10）");
  const moveFrom = ui.boundsCenter(ui.findByText(await api.uiTree(), CHILD_CANVAS_NAME));
  const moveBefore = ui.findByText(await api.uiTree(), CHILD_CANVAS_NAME)?.bounds;
  const moveTo = { x: moveFrom.x - 320, y: moveFrom.y - 170 };
  await api.postInput([api.mouseDrag(moveFrom, moveTo)]);
  await sleep(900);
  const moveAfter = ui.findByText(await api.uiTree(), CHILD_CANVAS_NAME)?.bounds;
  const movedEnough =
    moveBefore && moveAfter && Math.abs(moveAfter.left - moveBefore.left) >= 50;
  report.check(
    movedEnough,
    "步骤 10 拖拽后「子画布一」节点位置改变",
    `${JSON.stringify(moveBefore)} -> ${JSON.stringify(moveAfter)}`,
  );

  report.section("S11 自动布局（步骤 11）");
  const layoutBeforeTree = await api.uiTree();
  const rootBefore = ui.findByText(layoutBeforeTree, "根画布")?.bounds;
  const childBefore = ui.findByText(layoutBeforeTree, CHILD_CANVAS_NAME)?.bounds;
  await ui.clickNode(ui.findAttr(layoutBeforeTree, "title", "自动布局"));
  const sawDisabled = await waitForCondition(
    async () => {
      const button = ui.findAttr(await api.uiTree(), "title", "自动布局");
      return button?.states?.disabled === true;
    },
    { timeout: 2500, interval: 60, label: "自动布局按钮进入禁用态" },
  )
    .then(() => true)
    .catch(() => false);
  report.check(sawDisabled, "步骤 11 自动布局期间按钮进入禁用态");
  await sleep(1600);
  const layoutAfterTree = await api.uiTree();
  const layoutButton = ui.findAttr(layoutAfterTree, "title", "自动布局");
  report.check(layoutButton?.states?.disabled === false, "步骤 11 等待 1 秒后按钮恢复可用");
  const rootAfter = ui.findByText(layoutAfterTree, "根画布")?.bounds;
  const childAfter = ui.findByText(layoutAfterTree, CHILD_CANVAS_NAME)?.bounds;
  report.check(
    boundsMoved(rootBefore, rootAfter) || boundsMoved(childBefore, childAfter),
    "步骤 11 自动布局后至少一个画布节点坐标发生变化",
    JSON.stringify({ rootBefore, rootAfter, childBefore, childAfter }),
  );
  await snap("step11-auto-layout");

  report.section("S12 宇宙视口缩放（步骤 12）");
  const zoomBefore = ui.findByText(await api.uiTree(), "根画布")?.bounds;
  await api.postInput([api.mouseMove(600, 300), api.mouseScroll("down", 3)]);
  await sleep(1400);
  const zoomAfter = ui.findByText(await api.uiTree(), "根画布")?.bounds;
  const zoomed =
    zoomBefore &&
    zoomAfter &&
    (zoomAfter.width < zoomBefore.width - 2 || zoomAfter.height < zoomBefore.height - 2);
  report.check(
    zoomed,
    "步骤 12 滚轮缩小后「根画布」节点 bounds 尺寸变化",
    `${JSON.stringify(zoomBefore)} -> ${JSON.stringify(zoomAfter)}`,
  );
  await snap("step12-zoomed-out");

  report.section("S13 视口持久化（步骤 13）");
  await enterRootCanvas();
  report.check(ui.hasText(await api.uiTree(), "账号 A"), "步骤 13 双击「根画布」进入根画布");
  await ui.clickText("画布宇宙");
  await ui.waitForText("根画布", { timeout: 10000 });
  await sleep(1200);
  const persistedTree = await api.uiTree();
  const zoomBack = ui.findByText(persistedTree, "根画布")?.bounds;
  report.check(
    boundsSizeKept(zoomAfter, zoomBack),
    "步骤 13 进入画布再返回后「根画布」节点尺寸与缩放后一致（±2px）",
    `${JSON.stringify(zoomAfter)} -> ${JSON.stringify(zoomBack)}`,
  );
  await snap("step13-viewport-persisted");

  report.section("S14 逻辑删除与徽标（步骤 14）");
  await clickCanvasNodeAction(CHILD_CANVAS_NAME, "删除画布");
  await ui.waitForGone(CHILD_CANVAS_NAME, { timeout: 10000 });
  await sleep(900);
  const deletedTree = await api.uiTree();
  report.check(!ui.hasText(deletedTree, CHILD_CANVAS_NAME), "步骤 14 「子画布一」立即从宇宙消失（逻辑删除无确认框）");
  const badgeAfterDelete = badgeTexts(deletedTree);
  report.check(
    badgeAfterDelete.includes("1"),
    "步骤 14 回收站按钮出现红色徽标「1」",
    JSON.stringify(badgeAfterDelete),
  );
  await snap("step14-deleted-badge");

  report.section("S15 回收站面板（步骤 15）");
  await ui.clickNode(ui.findAttr(await api.uiTree(), "title", "回收站"));
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "恢复画布"),
    { timeout: 6000, label: "回收站面板出现恢复按钮" },
  );
  await sleep(400);
  const recyclePanelTree = await api.uiTree();
  report.check(ui.hasText(recyclePanelTree, CHILD_CANVAS_NAME), "步骤 15 面板中出现「子画布一」条目");
  report.check(
    ui.findAttr(recyclePanelTree, "title", "恢复画布") !== null,
    "步骤 15 面板条目包含「恢复画布」按钮",
  );
  await snap("step15-recycle-bin-panel");

  report.section("S16 恢复画布（步骤 16）");
  await ui.clickNode(ui.findAttr(await api.uiTree(), "title", "恢复画布"));
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return ui.hasText(tree, CHILD_CANVAS_NAME) && ui.findAttr(tree, "title", "恢复画布") === null;
    },
    { timeout: 10000, label: "节点恢复且面板条目消失" },
  );
  await sleep(800);
  const restoredTree = await api.uiTree();
  report.check(ui.hasText(restoredTree, CHILD_CANVAS_NAME), "步骤 16 宇宙重新出现「子画布一」节点");
  report.check(
    ui.findAttr(restoredTree, "title", "恢复画布") === null,
    "步骤 16 面板中该条目消失",
  );
  await snap("step16-restored");

  report.section("S17 永久删除确认框（步骤 17）");
  await clickCanvasNodeAction(CHILD_CANVAS_NAME, "删除画布");
  await ui.waitForGone(CHILD_CANVAS_NAME, { timeout: 10000 });
  await sleep(700);
  report.check(!ui.hasText(await api.uiTree(), CHILD_CANVAS_NAME), "步骤 17 再次逻辑删除「子画布一」");
  await ensureRecycleBinPanel();
  await ui.clickNode(ui.findAttr(await api.uiTree(), "title", "永久删除"));
  await ui.waitForDialog("永久删除画布", { timeout: 8000 });
  const physicalDeleteText = `确定要永久删除画布"${CHILD_CANVAS_NAME}"吗？其所有子画布及画布内的全部内容将被连带永久删除。此操作不可撤销。`;
  const physicalDeleteShown = await ui
    .waitForTextStable(physicalDeleteText, { timeout: 6000 })
    .catch(() => null);
  report.check(
    physicalDeleteShown !== null,
    "步骤 17 确认框标题「永久删除画布」与正文与预期一致",
    physicalDeleteText,
  );
  await snap("step17-physical-delete-confirm");

  report.section("S18 F4 取消永久删除");
  await sleep(350);
  const physicalDialog = ui.findDialog(await api.uiTree(), "永久删除画布");
  await ui.clickNode(ui.findButton(await api.uiTree(), "取消", { root: physicalDialog }));
  const physicalGone = await ui
    .waitForDialogGone("永久删除画布", { timeout: 6000 })
    .catch(() => false);
  report.check(physicalGone, "F4 点击「取消」后确认框关闭");
  await ensureRecycleBinPanel();
  const f4Tree = await api.uiTree();
  report.check(
    ui.findAttr(f4Tree, "title", "永久删除") !== null,
    "F4 取消后「子画布一」仍保留在回收站面板中",
  );
  await snap("f4-cancel-kept");

  report.section("S19 永久删除（步骤 18）");
  await ui.clickNode(ui.findAttr(await api.uiTree(), "title", "永久删除"));
  await ui.waitForDialog("永久删除画布", { timeout: 8000 });
  await sleep(350);
  const physicalDialog2 = ui.findDialog(await api.uiTree(), "永久删除画布");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: physicalDialog2 }));
  await sleep(1200);
  const permanentlyDeletedTree = await api.uiTree();
  report.check(
    !ui.hasText(permanentlyDeletedTree, CHILD_CANVAS_NAME),
    "步骤 18 永久删除后宇宙中无「子画布一」",
  );
  report.check(
    badgeTexts(permanentlyDeletedTree).length === 0,
    "步骤 18 回收站徽标消失",
  );
  await ensureRecycleBinPanel();
  const emptyBinTree = await api.uiTree();
  report.check(ui.hasText(emptyBinTree, "回收站为空"), "步骤 18 重新打开回收站面板显示「回收站为空」");
  report.check(
    ui.findAttr(emptyBinTree, "title", "永久删除") === null,
    "步骤 18 回收站面板中条目消失",
  );
  await snap("step18-recycle-bin-empty");

  report.section("S20 根画布按钮组（步骤 19、F3）");
  await api.postInput([api.mouseMove(700, 900), api.mouseClick("left", 1)]);
  await sleep(700);
  await hoverCanvasNode("根画布");
  const rootHoverTree = await api.uiTree();
  report.check(
    ui.findAttr(rootHoverTree, "title", "自定义颜色") !== null,
    "步骤 19 根画布 hover 按钮组出现「自定义颜色」按钮",
  );
  report.check(
    ui.findAttr(rootHoverTree, "title", "重命名画布") === null,
    "步骤 19 根画布不提供「重命名画布」按钮",
  );
  report.check(
    ui.findAttr(rootHoverTree, "title", "删除画布") === null,
    "F3 根画布不提供「删除画布」按钮，无法经 UI 触达 RootCanvasCannotBeDeleted",
  );
  await snap("step19-root-hover-actions");

  report.section("S21 清空回收站（步骤 20 修订）");
  await api.postInput([api.mouseMove(700, 900), api.mouseClick("left", 1)]);
  await sleep(700);
  await enterRootCanvas();
  await dragCreateChildCanvas();
  report.check(ui.hasText(await api.uiTree(), NEW_CANVAS_NAME), "步骤 20 重新创建画布数据节点（默认名「新画布」）");
  await gotoUniverse();
  report.check(ui.hasText(await api.uiTree(), NEW_CANVAS_NAME), "步骤 20 宇宙中出现「新画布」画布节点");
  await clickCanvasNodeAction(NEW_CANVAS_NAME, "删除画布");
  await ui.waitForGone(NEW_CANVAS_NAME, { timeout: 10000 });
  await sleep(800);
  await ensureRecycleBinPanel();
  const emptyButtonTree = await api.uiTree();
  report.check(
    ui.findButton(emptyButtonTree, "清空回收站")?.states?.disabled === false,
    "步骤 20 面板底部「清空回收站」按钮可用",
  );
  await ui.clickNode(ui.findButton(emptyButtonTree, "清空回收站"));
  await ui.waitForDialog("清空回收站", { timeout: 8000 });
  const emptyBinText =
    "确定要清空回收站吗？共 1 个画布将被永久删除，其子画布及画布内的全部内容将被连带永久删除。此操作不可撤销。";
  const emptyBinShown = await ui
    .waitForTextStable(emptyBinText, { timeout: 6000 })
    .catch(() => null);
  report.check(emptyBinShown !== null, "步骤 20 清空回收站确认框标题与正文与预期一致", emptyBinText);
  await snap("step20-empty-confirm");
  await sleep(350);
  const emptyDialog = ui.findDialog(await api.uiTree(), "清空回收站");
  await ui.clickNode(ui.findButton(await api.uiTree(), "确认", { root: emptyDialog }));
  await sleep(1500);
  await ensureRecycleBinPanel();
  const emptiedTree = await api.uiTree();
  report.check(ui.hasText(emptiedTree, "回收站为空"), "步骤 20 清空后面板显示「回收站为空」");
  report.check(!ui.hasText(emptiedTree, NEW_CANVAS_NAME), "步骤 20 清空后宇宙中无「新画布」");
  report.check(badgeTexts(emptiedTree).length === 0, "步骤 20 清空后回收站徽标消失");
  await snap("step20-emptied");
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
  prepareDataDir(DATA_DIR, "base");
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
