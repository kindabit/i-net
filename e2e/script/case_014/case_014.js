// case_014 偏好与外观。
//
// 流程：清空 output → 复制 base fixture → 启动应用 → 解锁进入根画布
//   → 步骤 1-3：语言菜单（当前项「简体中文」带高亮标记）→ 切 English（搜索按钮 title="Search"、
//     语言按钮 aria-label="Switch Language"）→ 切回「简体中文」（界面恢复中文）
//   → 步骤 4-6：主题菜单（「内置主题」分组 + 亮色/暗色/海洋/森林 + 「管理主题」；当前主题「亮色」带高亮）
//     → 点「暗色」（菜单保持打开，背景亮度骤降）→ 点主题按钮收起菜单 → 重开点「亮色」（恢复亮色）
//   → 步骤 7：主题管理对话框（四内置主题 + 「内置」标签 + 每行仅「导出」；新建/导入/关闭）
//   → 步骤 8（F1/F2）：新建主题编辑器（标识自动生成 custom-…、显示名默认「我的主题」、
//     8 项核心颜色色块以 role=button + name 入 UI 树、色块同行带同名颜色输入框）
//     → 清空标识保存 →「主题标识不能为空」→ 清空显示名保存 →「显示名称不能为空」
//   → 步骤 9：填「测试主题」保存 → snackbar + 列表新增自定义行
//   → F3/F4：内置名冲突（dark）与已存在标识两条内联错误
//   → 步骤 10：编辑（标识禁用）改显示名「测试主题 2」→ snackbar + 列表更新
//   → 步骤 11：导出（JSON 进剪贴板；脚本解析剪贴板 JSON 断言主题标识与显示名）
//   → 步骤 12（F6）：删除确认框（标题/正文/取消/删除）→ 取消保留 → 步骤 13 确认删除
//   → 步骤 14-15（F5）：导入对话框（textarea「主题 JSON」+ placeholder；空输入时导入按钮禁用）
//     → 非法 JSON 报 snackbar 且对话框保持打开 → 粘贴导出 JSON 导入成功、列表恢复
//   → 主题菜单自定义分组出现「测试主题 2」（补充覆盖）
//   → 步骤 16-20：设置对话框（滑块 aria-valuenow=10，刻度 1/10/30/60 秒）→ ArrowLeft×5 到 5 秒
//     （thumb 标签「5秒」由截图像素断言）→ 保存 → 重开仍为 5（持久化）→ ArrowRight×5 恢复 10 并保存
//   → 步骤 21-22：关于对话框（版本/作者/GitHub 仓库/Rust/Tauri/Vue/Vuetify + 简介；不点击仓库链接）→ 关闭
//   → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 断言优先使用 UI 树（文本、attrs、bounds、states），截图用于视觉留档；
// - 菜单当前项标记（active）不在 UI 树中暴露：以截图像素统计断言（亮色主题下激活行有整行灰色
//   高亮背景，非激活行仅文字像素），截图同步留档；
// - 主题切换以画布空白区域平均亮度断言（实测亮色 764 / 暗色 55，阈值 600 / 200）；
// - 主题菜单 `close-on-content-click=false`：点击主题项后菜单保持打开，需再点主题按钮收起
//   （与计划文本的差异记录在计划末尾的修订记录）；
// - 设置滑块 thumb 标签（「5秒」）不进入 UI 树：以 aria-valuenow + 截图 + thumb 上方深灰标签
//   像素统计断言；
// - 主题编辑器的 8 项核心颜色色块是原生 button（role=button，可访问名称即 `t('themes.colors.*')`
//   的「背景/表面/主色/次要色/成功色/警告色/错误色/信息色」），本脚本以 role+name 定位断言；
//   该对话框色块无 tooltip（源码未使用 VTooltip），故不做 tooltip 断言——修复记录中补语义的
//   ColorPairSwatch / ColorFieldEditor 用于节点颜色对话框，不在本用例范围；
// - 主题导出写系统剪贴板（navigator.clipboard.writeText），脚本读回解析 JSON 并在导入步骤回写；
// - 「关于」对话框的「GitHub 仓库」为 span role=button，点击会打开系统浏览器：本脚本只断言文本，
//   绝不点击；
// - snackbar 断言使用 waitForTextStable；点击会产生同名消息的操作前先等待旧消息消失；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_014\case_014.js`

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
/** 主题导出 JSON 的落盘备份（供导入步骤回写剪贴板） */
const THEME_JSON_FILE = path.join(OUTPUT_DIR, "theme-export.json");

/** fixture 数据库名称与密码（见 _fixtures\README.md） */
const DB_NAME = "zz-e2e-base";
const DB_PASSWORD = "e2e-password";

/** 自定义主题标识与显示名称（用例内固定，避免测试之间互相影响） */
const CUSTOM_THEME_ID = "custom-e2e-test";
const CUSTOM_THEME_NAME = "测试主题";
const CUSTOM_THEME_NAME_2 = "测试主题 2";

/** 各对话框的文本特征（用于 findDialog 定位） */
const DLG_MANAGER = "主题管理";
const DLG_EDITOR = "主题标识";
const DLG_IMPORT = "主题 JSON";
const DLG_SETTINGS = "自动清空剪贴板";
const DLG_ABOUT = "关于 I-Net";
const DLG_DELETE_CONFIRM = "永久删除自定义主题";

/** 主题编辑器中的 8 个颜色行标签 */
const COLOR_LABELS = ["背景", "表面", "主色", "次要色", "成功色", "警告色", "错误色", "信息色"];

/** 画布空白区域（用于主题明暗的平均亮度统计） */
const CANVAS_REGION = { left: 400, top: 600, width: 300, height: 300 };

/** 高亮背景像素阈值：当前菜单项（active）远高于非当前项（实测 >90% vs <2%） */
const HIGHLIGHT_ACTIVE_MIN = 4000;
const HIGHLIGHT_INACTIVE_MAX = 800;
/** 画布平均亮度阈值（实测亮色 764 / 暗色 55） */
const BRIGHTNESS_LIGHT_MIN = 600;
const BRIGHTNESS_DARK_MAX = 200;
/** 滑块 thumb 标签深灰像素阈值（实测未聚焦 273 / 聚焦 1057） */
const THUMB_LABEL_MIN = 600;

const report = new Report("case_014 偏好与外观");

let shotIndex = 0;
/** 窗口高度（用于底部 snackbar 区域判定） */
let windowHeight = 1208;
/** 设置对话框「保存」前记录的底部文本集合 */
let bottomTextsBeforeSave = [];

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
 * 返回 UI 树中文本精确匹配且有尺寸的 #text 节点数组。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 目标文本。
 * @returns {object[]} 命中的文本节点数组。
 */
function textNodes(tree, text) {
  return flattenNodes(tree).filter(
    (node) => node.tag === "#text" && (node.text ?? "").trim() === text && node.bounds.width > 0,
  );
}

/**
 * 返回 UI 树底部区域（snackbar 所在位置）的全部文本。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {string[]} 底部文本数组（已去重）。
 */
function bottomTexts(tree) {
  const threshold = Math.floor(windowHeight * 0.86);
  return [
    ...new Set(
      flattenNodes(tree)
        .filter((node) => node.tag === "#text" && (node.text ?? "").trim() !== "" && node.bounds.top > threshold)
        .map((node) => node.text.trim()),
    ),
  ];
}

/**
 * 在 UI 树中查找子树文本精确等于指定文本的列表项（菜单项、主题列表行等）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} exactText 列表项子树文本。
 * @returns {object|null} 命中的列表项；未命中时为 null。
 */
function findListRow(tree, exactText) {
  return (
    flattenNodes(tree).find(
      (node) => node.role === "listitem" && ui.subtreeText(node) === exactText,
    ) ?? null
  );
}

/**
 * 在主题管理列表中查找显示名称匹配的主题行。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} displayName 主题显示名称。
 * @returns {object|null} 命中的列表项；未命中时为 null。
 */
function findThemeRow(tree, displayName) {
  return (
    flattenNodes(tree).find(
      (node) =>
        node.role === "listitem" &&
        ui.subtreeText(node).includes(displayName) &&
        (ui.subtreeText(node).includes("内置") || ui.subtreeText(node).includes("自定义 ·")),
    ) ?? null
  );
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
 * 按 aria-label 在按钮数组中查找按钮。
 * @param {object[]} buttons 按钮节点数组。
 * @param {string} label aria-label 值。
 * @returns {object|null} 命中的按钮；未命中时为 null。
 */
function buttonByAria(buttons, label) {
  return buttons.find((button) => button.attrs?.["aria-label"] === label) ?? null;
}

/**
 * 统计截图上指定 bounds 区域内命中判定函数的像素数量。
 * @param {Buffer} buffer 截图的 PNG 数据。
 * @param {{left: number, top: number, width: number, height: number}} bounds 区域（窗口物理像素）。
 * @param {(r: number, g: number, b: number) => boolean} predicate 像素判定函数。
 * @returns {{count: number, total: number}} 命中像素数与总像素数。
 */
function countPixels(buffer, bounds, predicate) {
  const png = decodePng(buffer);
  const left = Math.max(0, Math.floor(bounds.left));
  const top = Math.max(0, Math.floor(bounds.top));
  const right = Math.min(png.width, Math.ceil(bounds.left + bounds.width));
  const bottom = Math.min(png.height, Math.ceil(bounds.top + bounds.height));
  let count = 0;
  let total = 0;
  for (let y = top; y < bottom; y += 1) {
    for (let x = left; x < right; x += 1) {
      const i = (y * png.width + x) * 4;
      total += 1;
      if (predicate(png.data[i], png.data[i + 1], png.data[i + 2])) {
        count += 1;
      }
    }
  }
  return { count, total };
}

/**
 * 判断像素是否为亮色主题下的菜单高亮背景（灰白系）。
 * @param {number} r 红色分量。
 * @param {number} g 绿色分量。
 * @param {number} b 蓝色分量。
 * @returns {boolean} 命中时为 true。
 */
function isGrayHighlight(r, g, b) {
  return r >= 195 && r <= 250 && Math.abs(r - g) <= 8 && Math.abs(g - b) <= 8;
}

/**
 * 判断像素是否为滑块 thumb 标签的深灰底色。
 * @param {number} r 红色分量。
 * @param {number} g 绿色分量。
 * @param {number} b 蓝色分量。
 * @returns {boolean} 命中时为 true。
 */
function isDarkGray(r, g, b) {
  return r >= 60 && r <= 140 && Math.abs(r - g) <= 12 && Math.abs(g - b) <= 12;
}

/**
 * 读取指定区域的画布平均亮度（三通道和）。
 * @param {{left: number, top: number, width: number, height: number}} region 区域（窗口物理像素）。
 * @returns {Promise<number>} 平均亮度（0..765）。
 */
async function canvasBrightness(region = CANVAS_REGION) {
  const png = decodePng(await api.screenshot());
  const left = Math.max(0, Math.floor(region.left));
  const top = Math.max(0, Math.floor(region.top));
  const right = Math.min(png.width, Math.ceil(region.left + region.width));
  const bottom = Math.min(png.height, Math.ceil(region.top + region.height));
  let sum = 0;
  let n = 0;
  for (let y = top; y < bottom; y += 1) {
    for (let x = left; x < right; x += 1) {
      const i = (y * png.width + x) * 4;
      sum += png.data[i] + png.data[i + 1] + png.data[i + 2];
      n += 1;
    }
  }
  return sum / n;
}

/**
 * 计算滑块 thumb 上方标签区域（用于深灰标签像素统计）。
 * @param {object} slider 滑块 thumb 节点。
 * @returns {{left: number, top: number, width: number, height: number}} 区域（窗口物理像素）。
 */
function thumbLabelRegion(slider) {
  return {
    left: slider.bounds.left + slider.bounds.width / 2 - 55,
    top: slider.bounds.top - 65,
    width: 110,
    height: 62,
  };
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
 * 将指定文件（UTF-8）的内容写入系统剪贴板。
 * @param {string} file 文件路径。
 * @returns {void} 无返回值。
 */
function writeClipboardFromFile(file) {
  execSync(
    `powershell -NoProfile -Command "Set-Clipboard -Value ([System.IO.File]::ReadAllText('${file}', [System.Text.Encoding]::UTF8))"`,
    { encoding: "utf8", timeout: 15000 },
  );
}

/**
 * 聚焦可编辑控件并以 Ctrl+A/Ctrl+C 复制其内容后读取剪贴板（用于验证输入框的值）。
 * @param {object} editable 可编辑控件节点。
 * @returns {Promise<string|null>} 控件内容；读取失败时为 null。
 */
async function readEditableValueByClipboard(editable) {
  await ui.clickNode(editable);
  await sleep(250);
  await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
  await sleep(400);
  return readClipboard();
}

/**
 * 点击 aria-label 匹配的按钮。
 * @param {string} label aria-label 值。
 * @returns {Promise<object>} 被点击的节点。
 */
async function clickAria(label) {
  const node = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "aria-label", label),
    { timeout: 10000, interval: 200, label: `aria-label=「${label}」按钮` },
  );
  await ui.clickNode(node);
  await sleep(500);
  return node;
}

/**
 * 等待 aria-label 匹配的按钮出现。
 * @param {string} label aria-label 值。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=10000] 超时毫秒数。
 * @returns {Promise<object>} 命中的节点。
 */
async function waitForAria(label, { timeout = 10000 } = {}) {
  return waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "aria-label", label),
    { timeout, interval: 250, label: `aria-label=「${label}」按钮` },
  );
}

/**
 * 在指定对话框内查找按钮并点击。
 * @param {string} dialogFragment 对话框文本特征。
 * @param {string} buttonText 按钮文本（精确匹配）。
 * @returns {Promise<object>} 被点击的按钮节点。
 */
async function clickDialogButton(dialogFragment, buttonText) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = ui.findDialog(tree, dialogFragment);
      return dialog ? ui.findButton(tree, buttonText, { root: dialog, exact: true }) : null;
    },
    { timeout: 10000, interval: 250, label: `对话框「${dialogFragment}」按钮「${buttonText}」` },
  );
  await ui.clickNode(button);
  await sleep(500);
  return button;
}

/**
 * 在指定对话框内聚焦可编辑控件并覆盖输入文本（或清空）。
 * @param {string} dialogFragment 对话框文本特征。
 * @param {string} nameFragment 可编辑控件名称片段。
 * @param {string} text 要输入的文本（clear 模式下忽略）。
 * @param {object} [options] 可选参数。
 * @param {boolean} [options.clear=false] 是否仅清空（不输入文本）。
 * @returns {Promise<object>} 被点击的可编辑控件。
 */
async function typeIntoDialog(dialogFragment, nameFragment, text, { clear = false } = {}) {
  const editable = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = ui.findDialog(tree, dialogFragment);
      return dialog ? ui.findEditable([dialog], nameFragment) : null;
    },
    { timeout: 8000, interval: 200, label: `对话框「${dialogFragment}」输入框「${nameFragment}」` },
  );
  await ui.clickNode(editable);
  await sleep(250);
  await api.postInput([api.keyClick(["Control", "a"])]);
  if (clear) {
    await api.postInput([api.keyClick(["Delete"])]);
  } else {
    await api.postInput([api.typeText(text)]);
  }
  await sleep(450);
  return editable;
}

/**
 * 打开语言菜单并等待其稳定（菜单项「English」出现，光标移出菜单区域）。
 * @param {string} [buttonLabel="切换语言"] 语言按钮的 aria-label（随当前界面语言变化）。
 * @returns {Promise<void>} 无返回值。
 */
async function openLanguageMenu(buttonLabel = "切换语言") {
  await clickAria(buttonLabel);
  await ui.waitForText("English", { exact: true, timeout: 6000 });
  await api.postInput([api.mouseMove(300, 400)]);
  await sleep(450);
}

/**
 * 打开主题菜单并等待其稳定（分组标题「内置主题」出现，光标移出菜单区域）。
 * @returns {Promise<void>} 无返回值。
 */
async function openThemeMenu() {
  await clickAria("切换主题");
  await ui.waitForText("内置主题", { exact: true, timeout: 6000 });
  await api.postInput([api.mouseMove(300, 400)]);
  await sleep(450);
}

/**
 * 通过主题按钮收起主题菜单（close-on-content-click=false，只能由 activator 切换）。
 * @returns {Promise<void>} 无返回值。
 */
async function closeThemeMenu() {
  const button = await waitForAria("切换主题");
  await ui.clickNode(button);
  await ui.waitForGone("内置主题", { timeout: 6000 });
  await sleep(400);
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
 * 打开主题管理对话框（经由主题菜单「管理主题」）。
 * @returns {Promise<void>} 无返回值。
 */
async function openThemeManager() {
  await openThemeMenu();
  await ui.clickText("管理主题", { exact: true });
  await ui.waitForDialog(DLG_MANAGER, { timeout: 10000 });
  await sleep(900);
}

/**
 * 打开设置对话框。
 * @returns {Promise<void>} 无返回值。
 */
async function openSettings() {
  await clickAria("设置");
  await ui.waitForDialog(DLG_SETTINGS, { timeout: 10000 });
  await sleep(900);
}

/**
 * 在设置对话框内点击滑块 thumb 使其聚焦。
 * @returns {Promise<object>} 滑块 thumb 节点。
 */
async function focusSlider() {
  const slider = await waitForCondition(
    async () => ui.findByRole(await api.uiTree(), "slider")[0] ?? null,
    { timeout: 8000, interval: 200, label: "设置滑块" },
  );
  await ui.clickNode(slider);
  await sleep(450);
  return slider;
}

/**
 * 读取设置滑块当前的 aria-valuenow。
 * @returns {Promise<string|null>} aria-valuenow 值；滑块不存在时为 null。
 */
async function sliderValueNow() {
  const tree = await api.uiTree();
  const slider = ui.findByRole(tree, "slider")[0];
  return slider?.attrs?.["aria-valuenow"] ?? null;
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  report.section("S0 前置与解锁");
  const info = await api.health();
  windowHeight = info.height;
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  const switched = await ensureChineseUi();
  report.check(true, "前置：界面语言为中文（必要时经语言菜单切回）", switched ? "已切回中文" : "初始即中文");
  await unlock();
  await sleep(800);
  {
    const tree = await api.uiTree();
    report.check(ui.hasText(tree, "账号 A"), "解锁后进入根画布并出现节点「账号 A」");
    report.check(
      (await waitForAria("切换语言")) !== null &&
        (await waitForAria("设置")) !== null &&
        (await waitForAria("切换主题")) !== null &&
        (await waitForAria("关于")) !== null,
      "右上角浮动操作区四个入口（切换语言/设置/切换主题/关于）均可见",
    );
  }
  await snap("s0-root-canvas");

  report.section("S1 语言切换（步骤 1-3）");
  await openLanguageMenu();
  {
    const tree = await api.uiTree();
    const zhRow = findListRow(tree, "简体中文");
    const enRow = findListRow(tree, "English");
    report.check(zhRow !== null && enRow !== null, "步骤 1 语言菜单列出「简体中文」与「English」");
    const buffer = await api.screenshot();
    const zh = zhRow ? countPixels(buffer, zhRow.bounds, isGrayHighlight) : { count: -1, total: 0 };
    const en = enRow ? countPixels(buffer, enRow.bounds, isGrayHighlight) : { count: -1, total: 0 };
    report.check(
      zh.count >= HIGHLIGHT_ACTIVE_MIN,
      "步骤 1 当前语言「简体中文」带高亮标记（高亮像素 ≥ 阈值）",
      `zh=${zh.count}/${zh.total}`,
    );
    report.check(
      en.count <= HIGHLIGHT_INACTIVE_MAX,
      "步骤 1 「English」无高亮标记（对照）",
      `en=${en.count}/${en.total}`,
    );
  }
  await snap("s1-language-menu-zh");

  await ui.clickText("English", { exact: true });
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "Search"),
    { timeout: 8000, interval: 200, label: "英文界面搜索按钮 title=Search" },
  );
  const langMenuClosed = await ui
    .waitForGone("English", { timeout: 6000 })
    .then(() => true)
    .catch(() => false);
  report.check(langMenuClosed, "步骤 2 切换语言后语言菜单自动关闭");
  await sleep(600);
  {
    const tree = await api.uiTree();
    report.check(ui.findAttr(tree, "title", "Search") !== null, "步骤 2 切换到 English 后搜索按钮 title=Search");
    report.check(
      ui.findAttr(tree, "aria-label", "Switch Language") !== null &&
        ui.findAttr(tree, "aria-label", "Switch Theme") !== null &&
        ui.findAttr(tree, "aria-label", "Settings") !== null &&
        ui.findAttr(tree, "aria-label", "About") !== null,
      "步骤 2 右上角按钮 aria-label 切换为英文（Switch Language 等）",
    );
    report.check(!ui.hasText(tree, "数据库"), "步骤 2 界面文案切换为英文（不残留中文界面）");
  }
  await snap("s2-english-ui");

  await openLanguageMenu("Switch Language");
  await ui.clickText("简体中文", { exact: true });
  await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "搜索"),
    { timeout: 8000, interval: 200, label: "中文界面搜索按钮 title=搜索" },
  );
  const langMenuClosed2 = await ui
    .waitForGone("简体中文", { timeout: 6000 })
    .then(() => true)
    .catch(() => false);
  report.check(langMenuClosed2, "步骤 3 切回中文后语言菜单自动关闭");
  await sleep(600);
  {
    const tree = await api.uiTree();
    report.check(ui.findAttr(tree, "title", "搜索") !== null, "步骤 3 切回「简体中文」后搜索按钮 title=搜索");
    report.check(
      ui.findAttr(tree, "aria-label", "切换语言") !== null,
      "步骤 3 语言按钮 aria-label 恢复为「切换语言」",
    );
  }
  await snap("s3-language-back-zh");

  report.section("S2 主题切换（步骤 4-6）");
  await openThemeMenu();
  {
    const tree = await api.uiTree();
    report.check(ui.hasText(tree, "内置主题", { exact: true }), "步骤 4 主题菜单显示「内置主题」分组");
    report.check(
      ["亮色", "暗色", "海洋", "森林"].every((name) => findListRow(tree, name) !== null),
      "步骤 4 主题菜单列出四个内置主题",
    );
    report.check(findListRow(tree, "管理主题") !== null, "步骤 4 主题菜单包含「管理主题」入口");
    report.check(
      !ui.hasText(tree, "自定义主题", { exact: true }),
      "步骤 4 初始无自定义主题时不显示「自定义主题」分组（源码按需渲染）",
    );
    const lightRow = findListRow(tree, "亮色");
    const darkRow = findListRow(tree, "暗色");
    const buffer = await api.screenshot();
    const light = lightRow ? countPixels(buffer, lightRow.bounds, isGrayHighlight) : { count: -1, total: 0 };
    const dark = darkRow ? countPixels(buffer, darkRow.bounds, isGrayHighlight) : { count: -1, total: 0 };
    report.check(
      light.count >= HIGHLIGHT_ACTIVE_MIN,
      "步骤 4 当前主题「亮色」带高亮标记（高亮像素 ≥ 阈值）",
      `light=${light.count}/${light.total}`,
    );
    report.check(
      dark.count <= HIGHLIGHT_INACTIVE_MAX,
      "步骤 4 「暗色」无高亮标记（对照）",
      `dark=${dark.count}/${dark.total}`,
    );
  }
  await snap("s4-theme-menu-light");

  await ui.clickText("暗色", { exact: true });
  await sleep(900);
  {
    const tree = await api.uiTree();
    report.check(
      ui.hasText(tree, "内置主题", { exact: true }),
      "步骤 5 点击「暗色」后主题菜单保持打开（close-on-content-click=false）",
    );
    const brightness = await canvasBrightness();
    report.check(
      brightness <= BRIGHTNESS_DARK_MAX,
      "步骤 5 界面切换为暗色（画布平均亮度 ≤ 阈值）",
      `brightness=${brightness.toFixed(1)}`,
    );
  }
  await snap("s5-theme-dark");

  await closeThemeMenu();
  await openThemeMenu();
  await ui.clickText("亮色", { exact: true });
  await sleep(900);
  {
    const tree = await api.uiTree();
    report.check(
      ui.hasText(tree, "内置主题", { exact: true }),
      "步骤 6 点击「亮色」后主题菜单保持打开",
    );
    const brightness = await canvasBrightness();
    report.check(
      brightness >= BRIGHTNESS_LIGHT_MIN,
      "步骤 6 界面恢复亮色（画布平均亮度 ≥ 阈值）",
      `brightness=${brightness.toFixed(1)}`,
    );
  }
  await snap("s6-theme-light-restored");
  await closeThemeMenu();

  report.section("S3 主题管理（步骤 7-13）");
  await openThemeManager();
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_MANAGER);
    report.check(dialog !== null, "步骤 7 打开对话框「主题管理」");
    const builtinNames = ["亮色", "暗色", "海洋", "森林"];
    const rows = builtinNames.map((name) => findThemeRow(tree, name));
    report.check(rows.every((row) => row !== null), "步骤 7 列表含四个内置主题");
    report.check(
      rows.every((row) => row !== null && ui.subtreeText(row).includes("内置")),
      "步骤 7 内置主题行带「内置」标签",
    );
    report.check(
      rows.every((row) => rowButtons(row).length === 1 && buttonByAria(rowButtons(row), "导出") !== null),
      "步骤 7 内置项仅有「导出」操作（无编辑/删除按钮）",
    );
    report.check(
      dialog !== null &&
        ui.findButton(tree, "新建主题", { root: dialog, exact: true }) !== null &&
        ui.findButton(tree, "导入主题", { root: dialog, exact: true }) !== null &&
        ui.findButton(tree, "关闭", { root: dialog, exact: true }) !== null,
      "步骤 7 对话框含「新建主题」「导入主题」「关闭」按钮",
    );
  }
  await snap("s7-theme-manager");

  await clickDialogButton(DLG_MANAGER, "新建主题");
  await ui.waitForDialog(DLG_EDITOR, { timeout: 10000 });
  await sleep(900);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_EDITOR);
    report.check(dialog !== null, "步骤 8 打开对话框「新建主题」");
    report.check(
      dialog !== null && ui.hasText([dialog], "新建主题", { exact: true }),
      "步骤 8 编辑器标题为「新建主题」",
    );
    const idEditable = dialog ? ui.findEditable([dialog], "主题标识") : null;
    const nameEditable = dialog ? ui.findEditable([dialog], "显示名称") : null;
    const idValue = idEditable ? await readEditableValueByClipboard(idEditable) : null;
    const nameValue = nameEditable ? await readEditableValueByClipboard(nameEditable) : null;
    report.check(
      typeof idValue === "string" && /^custom-[0-9a-z]+$/.test(idValue),
      "步骤 8 「主题标识」已自动填充（custom-…）",
      `id=${idValue}`,
    );
    report.check(nameValue === "我的主题", "步骤 8 「显示名称」默认为「我的主题」", `name=${nameValue}`);
    report.check(
      dialog !== null && ui.findByRole([dialog], "checkbox").some((node) => node.attrs?.["aria-label"] === "暗色基调"),
      "步骤 8 存在「暗色基调」开关",
    );
    const swatches = COLOR_LABELS.map((label) =>
      dialog === null
        ? null
        : flattenNodes([dialog]).find(
            (node) => node.role === "button" && node.name === label && node.bounds.width > 0,
          ) ?? null,
    );
    report.check(
      swatches.every((node) => node !== null),
      "步骤 8 8 项核心颜色色块以 role=button + name 入 UI 树",
      COLOR_LABELS.join(","),
    );
    report.check(
      COLOR_LABELS.every((label) => dialog !== null && ui.findEditable([dialog], label) !== null),
      "步骤 8 8 项颜色行均带同名颜色输入框",
      COLOR_LABELS.join(","),
    );
  }
  await snap("s8-theme-editor-create");

  await typeIntoDialog(DLG_EDITOR, "主题标识", "", { clear: true });
  await clickDialogButton(DLG_EDITOR, "保存");
  await sleep(600);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_EDITOR);
    report.check(
      dialog !== null && ui.hasText([dialog], "主题标识不能为空", { exact: true }),
      "F1 标识为空时内联提示「主题标识不能为空」",
    );
    report.check(dialog !== null, "F1 校验失败时不保存（对话框保持打开）");
  }
  await snap("s8b-f1-id-empty");

  await typeIntoDialog(DLG_EDITOR, "主题标识", CUSTOM_THEME_ID);
  await typeIntoDialog(DLG_EDITOR, "显示名称", "", { clear: true });
  await clickDialogButton(DLG_EDITOR, "保存");
  await sleep(600);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_EDITOR);
    report.check(
      dialog !== null && ui.hasText([dialog], "显示名称不能为空", { exact: true }),
      "F2 显示名称为空时内联提示「显示名称不能为空」",
    );
    report.check(
      dialog !== null && !ui.hasText([dialog], "主题标识不能为空", { exact: true }),
      "F2 标识合法的前提下不再显示标识错误",
    );
  }
  await snap("s8c-f2-name-empty");

  await ui.waitForGone(`主题“${CUSTOM_THEME_NAME}”已保存`, { timeout: 1500 }).catch(() => null);
  await typeIntoDialog(DLG_EDITOR, "显示名称", CUSTOM_THEME_NAME);
  await clickDialogButton(DLG_EDITOR, "保存");
  const savedSnackbar = await ui
    .waitForTextStable(`主题“${CUSTOM_THEME_NAME}”已保存`, { timeout: 10000 })
    .catch(() => null);
  report.check(savedSnackbar !== null, `步骤 9 snackbar「主题“${CUSTOM_THEME_NAME}”已保存」`);
  const editorClosed = await ui
    .waitForDialogGone(DLG_EDITOR, { timeout: 8000 })
    .then(() => true)
    .catch(() => false);
  report.check(editorClosed, "步骤 9 保存成功后编辑器关闭");
  {
    const tree = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return findThemeRow(t, CUSTOM_THEME_NAME) ? t : null;
      },
      { timeout: 8000, interval: 250, label: "主题管理列表新增自定义行" },
    ).catch(() => null);
    const row = tree ? findThemeRow(tree, CUSTOM_THEME_NAME) : null;
    report.check(row !== null, `步骤 9 列表新增「${CUSTOM_THEME_NAME}」行`);
    report.check(
      row !== null && ui.subtreeText(row).includes(`自定义 · ${CUSTOM_THEME_ID}`),
      "步骤 9 新增行带「自定义」标签与主题标识",
      row ? ui.subtreeText(row) : "none",
    );
  }
  await snap("s9-custom-theme-saved");

  await clickDialogButton(DLG_MANAGER, "新建主题");
  await ui.waitForDialog(DLG_EDITOR, { timeout: 10000 });
  await sleep(800);
  await typeIntoDialog(DLG_EDITOR, "主题标识", "dark");
  await clickDialogButton(DLG_EDITOR, "保存");
  await sleep(600);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_EDITOR);
    report.check(
      dialog !== null && ui.hasText([dialog], "名称“dark”与内置主题冲突", { exact: true }),
      "F3 标识与内置主题冲突时内联提示「名称“dark”与内置主题冲突」",
    );
  }
  await snap("s10-f3-reserved-name");

  await typeIntoDialog(DLG_EDITOR, "主题标识", CUSTOM_THEME_ID);
  await clickDialogButton(DLG_EDITOR, "保存");
  await sleep(600);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_EDITOR);
    report.check(
      dialog !== null && ui.hasText([dialog], "已存在同名主题", { exact: true }),
      "F4 标识已存在时内联提示「已存在同名主题」",
    );
  }
  await snap("s11-f4-existing-name");

  await clickDialogButton(DLG_EDITOR, "取消");
  const cancelClosed = await ui
    .waitForDialogGone(DLG_EDITOR, { timeout: 8000 })
    .then(() => true)
    .catch(() => false);
  report.check(cancelClosed, "F4 之后「取消」关闭编辑器并回到主题管理");

  await ui.waitForGone(`主题“${CUSTOM_THEME_NAME}”已保存`, { timeout: 1500 }).catch(() => null);
  {
    const tree = await api.uiTree();
    const row = findThemeRow(tree, CUSTOM_THEME_NAME);
    const buttons = rowButtons(row);
    report.check(
      buttons.length === 3 &&
        buttonByAria(buttons, "导出") !== null &&
        buttonByAria(buttons, "编辑") !== null &&
        buttonByAria(buttons, "删除") !== null,
      "步骤 10 自定义行含「导出」「编辑」「删除」三个操作",
      buttons.map((b) => b.attrs?.["aria-label"] ?? "").join(","),
    );
  }
  {
    const tree = await api.uiTree();
    const row = findThemeRow(tree, CUSTOM_THEME_NAME);
    await ui.clickNode(buttonByAria(rowButtons(row), "编辑"));
  }
  await ui.waitForDialog(DLG_EDITOR, { timeout: 10000 });
  await sleep(800);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_EDITOR);
    report.check(
      dialog !== null && ui.hasText([dialog], "编辑主题", { exact: true }),
      "步骤 10 打开编辑器「编辑主题」",
    );
    const idEditable = dialog ? ui.findEditable([dialog], "主题标识") : null;
    report.check(idEditable?.states?.disabled === true, "步骤 10 编辑态「主题标识」不可修改（disabled）");
  }
  await snap("s12-theme-editor-edit");

  await typeIntoDialog(DLG_EDITOR, "显示名称", CUSTOM_THEME_NAME_2);
  await clickDialogButton(DLG_EDITOR, "保存");
  const savedSnackbar2 = await ui
    .waitForTextStable(`主题“${CUSTOM_THEME_NAME_2}”已保存`, { timeout: 10000 })
    .catch(() => null);
  report.check(savedSnackbar2 !== null, `步骤 10 snackbar「主题“${CUSTOM_THEME_NAME_2}”已保存」`);
  {
    const tree = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return findThemeRow(t, CUSTOM_THEME_NAME_2) ? t : null;
      },
      { timeout: 8000, interval: 250, label: "列表行更新为「测试主题 2」" },
    ).catch(() => null);
    report.check(tree !== null, "步骤 10 列表项更新为「测试主题 2」");
    report.check(
      tree !== null && textNodes(tree, CUSTOM_THEME_NAME).length === 0,
      "步骤 10 旧显示名称「测试主题」不再作为独立文本出现",
    );
    const updatedRow = tree ? findThemeRow(tree, CUSTOM_THEME_NAME_2) : null;
    report.check(
      updatedRow !== null && ui.subtreeText(updatedRow).includes(`自定义 · ${CUSTOM_THEME_ID}`),
      "步骤 10 编辑后主题标识保持为 custom-e2e-test",
      updatedRow ? ui.subtreeText(updatedRow) : "none",
    );
  }
  await snap("s13-theme-edited");

  await ui.waitForGone(`主题“${CUSTOM_THEME_NAME_2}”已保存`, { timeout: 1500 }).catch(() => null);
  {
    const tree = await api.uiTree();
    const row = findThemeRow(tree, CUSTOM_THEME_NAME_2);
    await ui.clickNode(buttonByAria(rowButtons(row), "导出"));
  }
  const exportSnackbar = await ui
    .waitForTextStable(`主题“${CUSTOM_THEME_NAME_2}”的 JSON 已复制到剪贴板`, { timeout: 10000 })
    .catch(() => null);
  report.check(
    exportSnackbar !== null,
    `步骤 11 snackbar「主题“${CUSTOM_THEME_NAME_2}”的 JSON 已复制到剪贴板」`,
  );
  {
    const clip = readClipboard();
    let parsed = null;
    try {
      parsed = clip === null ? null : JSON.parse(clip);
    } catch {
      parsed = null;
    }
    report.check(
      parsed !== null &&
        parsed.name === CUSTOM_THEME_ID &&
        parsed.displayName === CUSTOM_THEME_NAME_2 &&
        parsed.dark === false &&
        parsed.colors !== undefined,
      "步骤 11 剪贴板内容为包含主题标识的 JSON 文本（可解析且字段匹配）",
      clip === null ? "clipboard=null" : `${clip.slice(0, 120).replace(/\s+/g, " ")}…`,
    );
    if (parsed !== null && clip !== null) {
      fs.writeFileSync(THEME_JSON_FILE, clip, "utf8");
    }
  }
  await snap("s14-theme-exported");

  await ui.waitForGone(`主题“${CUSTOM_THEME_NAME_2}”的 JSON 已复制到剪贴板`, { timeout: 1500 }).catch(() => null);
  {
    const tree = await api.uiTree();
    const row = findThemeRow(tree, CUSTOM_THEME_NAME_2);
    await ui.clickNode(buttonByAria(rowButtons(row), "删除"));
  }
  await ui.waitForDialog(DLG_DELETE_CONFIRM, { timeout: 8000 });
  await sleep(700);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_DELETE_CONFIRM);
    report.check(dialog !== null, "步骤 12 打开删除确认框");
    report.check(
      dialog !== null && ui.hasText([dialog], "删除主题", { exact: true }),
      "步骤 12 确认框标题「删除主题」",
    );
    report.check(
      dialog !== null &&
        ui.hasText(
          [dialog],
          `此操作将永久删除自定义主题“${CUSTOM_THEME_NAME_2}”，且无法恢复。`,
          { exact: true },
        ),
      "步骤 12 确认框正文与主题名匹配",
    );
    report.check(
      dialog !== null &&
        ui.findButton(tree, "取消", { root: dialog, exact: true }) !== null &&
        ui.findButton(tree, "删除", { root: dialog, exact: true }) !== null,
      "步骤 12 确认框按钮为「取消」「删除」",
    );
  }
  await snap("s15-delete-confirm");

  await clickDialogButton(DLG_DELETE_CONFIRM, "取消");
  const confirmClosed = await ui
    .waitForDialogGone(DLG_DELETE_CONFIRM, { timeout: 8000 })
    .then(() => true)
    .catch(() => false);
  report.check(confirmClosed, "F6 「取消」关闭删除确认框");
  {
    const tree = await api.uiTree();
    report.check(
      findThemeRow(tree, CUSTOM_THEME_NAME_2) !== null,
      "F6 主题「测试主题 2」保留在列表中",
    );
  }
  await snap("s16-delete-canceled");

  await ui.waitForGone(`主题“${CUSTOM_THEME_NAME_2}”的 JSON 已复制到剪贴板`, { timeout: 1500 }).catch(() => null);
  {
    const tree = await api.uiTree();
    const row = findThemeRow(tree, CUSTOM_THEME_NAME_2);
    await ui.clickNode(buttonByAria(rowButtons(row), "删除"));
  }
  await ui.waitForDialog(DLG_DELETE_CONFIRM, { timeout: 8000 });
  await sleep(700);
  await clickDialogButton(DLG_DELETE_CONFIRM, "删除");
  const deletedSnackbar = await ui
    .waitForTextStable(`主题“${CUSTOM_THEME_NAME_2}”已删除`, { timeout: 10000 })
    .catch(() => null);
  report.check(deletedSnackbar !== null, `步骤 13 snackbar「主题“${CUSTOM_THEME_NAME_2}”已删除」`);
  {
    const tree = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return findThemeRow(t, CUSTOM_THEME_NAME_2) === null ? t : null;
      },
      { timeout: 8000, interval: 250, label: "列表移除「测试主题 2」" },
    ).catch(() => null);
    report.check(tree !== null, "步骤 13 列表移除「测试主题 2」");
    const dialog = tree ? ui.findDialog(tree, DLG_MANAGER) : null;
    const rows = dialog ? ui.findByRole([dialog], "listitem") : [];
    report.check(rows.length === 4, "步骤 13 列表只剩四个内置主题", `rows=${rows.length}`);
    report.check(
      tree !== null && !ui.hasText(tree, "自定义", { exact: true }),
      "步骤 13 列表不再出现「自定义」标签",
    );
  }
  await snap("s17-theme-deleted");

  report.section("S4 导入主题（步骤 14-15 与 F5）");
  await ui.waitForGone(`主题“${CUSTOM_THEME_NAME_2}”已删除`, { timeout: 1500 }).catch(() => null);
  await clickDialogButton(DLG_MANAGER, "导入主题");
  await ui.waitForDialog(DLG_IMPORT, { timeout: 10000 });
  await sleep(800);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_IMPORT);
    const textarea = dialog ? ui.findEditable([dialog], "主题 JSON") : null;
    report.check(dialog !== null, "步骤 14 打开对话框「导入主题」");
    report.check(
      textarea !== null &&
        textarea.tag === "TEXTAREA" &&
        textarea.attrs?.placeholder === "粘贴他人分享的主题 JSON 文本",
      "步骤 14 含 label「主题 JSON」的多行输入与 placeholder",
      `tag=${textarea?.tag ?? "none"} placeholder=${textarea?.attrs?.placeholder ?? "none"}`,
    );
    const importButton = dialog ? ui.findButton(tree, "导入主题", { root: dialog, exact: true }) : null;
    report.check(importButton?.states?.disabled === true, "步骤 14 空输入时「导入主题」按钮禁用");
  }
  await snap("s18-import-dialog");

  await typeIntoDialog(DLG_IMPORT, "主题 JSON", "not-json");
  await sleep(400);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_IMPORT);
    const importButton = dialog ? ui.findButton(tree, "导入主题", { root: dialog, exact: true }) : null;
    report.check(importButton?.states?.disabled === false, "F5 输入非空后「导入主题」按钮可用");
  }
  await clickDialogButton(DLG_IMPORT, "导入主题");
  const invalidSnackbar = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return ui.findByText(tree, "主题导入失败：数据格式不正确（") ?? null;
    },
    { timeout: 10000, interval: 250, label: "导入失败 snackbar" },
  ).catch(() => null);
  report.check(invalidSnackbar !== null, "F5 snackbar「主题导入失败：数据格式不正确（…）」");
  {
    const tree = await api.uiTree();
    report.check(
      ui.findDialog(tree, DLG_IMPORT) !== null,
      "F5 导入失败后对话框保持打开（persistent）",
    );
    report.check(
      !ui.hasText(tree, "测试主题 2", { exact: true }),
      "F5 非法 JSON 未被导入（列表无该主题）",
    );
  }
  await snap("s19-f5-import-invalid");

  await typeIntoDialog(DLG_IMPORT, "主题 JSON", "", { clear: true });
  writeClipboardFromFile(THEME_JSON_FILE);
  await sleep(400);
  await api.postInput([api.keyClick(["Control", "v"])]);
  await sleep(600);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_IMPORT);
    const importButton = dialog ? ui.findButton(tree, "导入主题", { root: dialog, exact: true }) : null;
    report.check(importButton?.states?.disabled === false, "步骤 15 粘贴导出 JSON 后「导入主题」按钮可用");
  }
  await snap("s20-import-dialog-pasted");
  await clickDialogButton(DLG_IMPORT, "导入主题");
  const importSnackbar = await ui
    .waitForTextStable(`主题“${CUSTOM_THEME_NAME_2}”导入成功`, { timeout: 10000 })
    .catch(() => null);
  report.check(importSnackbar !== null, `步骤 15 snackbar「主题“${CUSTOM_THEME_NAME_2}”导入成功」`);
  const importClosed = await ui
    .waitForDialogGone(DLG_IMPORT, { timeout: 8000 })
    .then(() => true)
    .catch(() => false);
  report.check(importClosed, "步骤 15 导入成功后对话框关闭");
  {
    const tree = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return findThemeRow(t, CUSTOM_THEME_NAME_2) ? t : null;
      },
      { timeout: 8000, interval: 250, label: "列表重新出现「测试主题 2」" },
    ).catch(() => null);
    const row = tree ? findThemeRow(tree, CUSTOM_THEME_NAME_2) : null;
    report.check(row !== null, "步骤 15 列表重新出现「测试主题 2」");
    report.check(
      row !== null && ui.subtreeText(row).includes(`自定义 · ${CUSTOM_THEME_ID}`),
      "步骤 15 导入行带「自定义」标签与主题标识",
      row ? ui.subtreeText(row) : "none",
    );
  }
  await snap("s21-import-success");

  await clickDialogButton(DLG_MANAGER, "关闭");
  const managerClosed = await ui
    .waitForDialogGone(DLG_MANAGER, { timeout: 8000 })
    .then(() => true)
    .catch(() => false);
  report.check(managerClosed, "步骤 15 关闭主题管理对话框");
  await sleep(600);

  report.section("S5 主题菜单自定义分组（补充覆盖）");
  await openThemeMenu();
  {
    const tree = await api.uiTree();
    report.check(ui.hasText(tree, "自定义主题", { exact: true }), "存在自定义主题时主题菜单显示「自定义主题」分组");
    report.check(
      findListRow(tree, CUSTOM_THEME_NAME_2) !== null,
      "自定义分组列出「测试主题 2」",
    );
    const lightRow = findListRow(tree, "亮色");
    const buffer = await api.screenshot();
    const light = lightRow ? countPixels(buffer, lightRow.bounds, isGrayHighlight) : { count: -1, total: 0 };
    report.check(
      light.count >= HIGHLIGHT_ACTIVE_MIN,
      "当前主题仍是「亮色」（自定义主题出现后标记不变）",
      `light=${light.count}/${light.total}`,
    );
  }
  await snap("s22-theme-menu-custom-group");
  await closeThemeMenu();

  report.section("S6 设置对话框（步骤 16-20）");
  await openSettings();
  {
    const tree = await api.uiTree();
    report.check(ui.findDialog(tree, DLG_SETTINGS) !== null, "步骤 16 打开对话框「设置」");
    report.check(
      ui.hasText(tree, "自动清空剪贴板等待时间（秒）", { exact: true }),
      "步骤 16 显示「自动清空剪贴板等待时间（秒）」",
    );
    const value = await sliderValueNow();
    report.check(value === "10", "步骤 16 滑块 aria-valuenow 为 10", `valuenow=${value}`);
    report.check(
      ["1秒", "10秒", "30秒", "60秒"].every((t) => textNodes(tree, t).length > 0),
      "步骤 16 滑块刻度显示 1/10/30/60 秒",
    );
  }
  await snap("s23-settings-open");

  const sliderBefore = await focusSlider();
  const beforeLabel = countPixels(await api.screenshot(), thumbLabelRegion(sliderBefore), isDarkGray);
  await api.postInput([api.keyClick(["ArrowLeft"], 5)]);
  await sleep(700);
  {
    const value = await sliderValueNow();
    report.check(value === "5", "步骤 17 按 ArrowLeft 五次后滑块 aria-valuenow 为 5", `valuenow=${value}`);
    const sliderAfter = await waitForCondition(
      async () => ui.findByRole(await api.uiTree(), "slider")[0] ?? null,
      { timeout: 5000, interval: 200, label: "滑块 thumb（调整后）" },
    );
    const buffer = await api.screenshot();
    const afterLabel = countPixels(buffer, thumbLabelRegion(sliderAfter), isDarkGray);
    report.check(
      afterLabel.count >= THUMB_LABEL_MIN,
      "步骤 17 thumb 标签「5秒」显示（thumb 上方深灰标签像素 ≥ 阈值）",
      `before=${beforeLabel.count} after=${afterLabel.count}`,
    );
  }
  await snap("s24-settings-5s");

  bottomTextsBeforeSave = bottomTexts(await api.uiTree());
  await clickDialogButton(DLG_SETTINGS, "保存");
  const settingsClosed = await ui
    .waitForDialogGone(DLG_SETTINGS, { timeout: 8000 })
    .then(() => true)
    .catch(() => false);
  report.check(settingsClosed, "步骤 18 点击「保存」后设置对话框关闭");
  await sleep(1200);
  {
    const after = bottomTexts(await api.uiTree());
    const added = after.filter((text) => !bottomTextsBeforeSave.includes(text));
    report.check(added.length === 0, "步骤 18 保存无 snackbar 提示", added.join(",") || "none");
  }
  await snap("s25-after-settings-save");

  await openSettings();
  {
    const value = await sliderValueNow();
    report.check(value === "5", "步骤 19 重开设置对话框后滑块仍为 5（偏好已持久化）", `valuenow=${value}`);
  }
  await snap("s26-settings-persisted");

  await focusSlider();
  await api.postInput([api.keyClick(["ArrowRight"], 5)]);
  await sleep(700);
  {
    const value = await sliderValueNow();
    report.check(value === "10", "步骤 20 按 ArrowRight 五次恢复为 10", `valuenow=${value}`);
  }
  await clickDialogButton(DLG_SETTINGS, "保存");
  await ui.waitForDialogGone(DLG_SETTINGS, { timeout: 8000 }).catch(() => null);
  await sleep(700);
  await openSettings();
  {
    const value = await sliderValueNow();
    report.check(value === "10", "步骤 20 恢复默认值 10 秒已持久化", `valuenow=${value}`);
  }
  await clickDialogButton(DLG_SETTINGS, "关闭");
  const settingsClosedAgain = await ui
    .waitForDialogGone(DLG_SETTINGS, { timeout: 8000 })
    .then(() => true)
    .catch(() => false);
  report.check(settingsClosedAgain, "步骤 20 关闭设置对话框");
  await snap("s27-settings-restored");

  report.section("S7 关于对话框（步骤 21-22）");
  await clickAria("关于");
  await ui.waitForDialog(DLG_ABOUT, { timeout: 10000 });
  await sleep(900);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, DLG_ABOUT);
    report.check(dialog !== null, "步骤 21 打开对话框「关于 I-Net」");
    /** 读取标签所属列表行内除标签外的值文本 */
    const valueOf = (label) => {
      if (!dialog) return "";
      const row = flattenNodes([dialog]).find(
        (node) => node.role === "listitem" && ui.subtreeText(node).includes(label),
      );
      if (!row) return "";
      const texts = flattenNodes([row])
        .filter((node) => node.tag === "#text")
        .map((node) => (node.text ?? "").trim())
        .filter((text) => text !== "" && text !== label);
      return texts[0] ?? "";
    };
    const labels = ["版本", "作者", "GitHub 仓库", "Rust 版本", "Tauri 版本", "Vue 版本", "Vuetify 版本"];
    report.check(
      labels.every((label) => dialog !== null && ui.subtreeText(dialog).includes(label)),
      "步骤 21 显示版本/作者/GitHub 仓库/Rust/Tauri/Vue/Vuetify 标签",
      labels.join(","),
    );
    report.check(/^\d+\.\d+\.\d+/.test(valueOf("版本")), "步骤 21 版本号非空", valueOf("版本"));
    report.check(valueOf("作者") !== "", "步骤 21 作者非空", valueOf("作者"));
    const repoRow = dialog
      ? flattenNodes([dialog]).find(
          (node) => node.role === "listitem" && ui.subtreeText(node).includes("GitHub 仓库"),
        )
      : null;
    const repoLink = repoRow ? flattenNodes([repoRow]).find((node) => node.role === "button") : null;
    report.check(
      repoLink !== null && ui.ownText(repoLink).startsWith("https://"),
      "步骤 21 GitHub 仓库地址非空（span role=button，仅断言文本）",
      repoLink ? ui.ownText(repoLink) : "none",
    );
    report.check(
      /^\d+\.\d+/.test(valueOf("Rust 版本")) &&
        /^\d+\.\d+/.test(valueOf("Tauri 版本")) &&
        /^\d+\.\d+/.test(valueOf("Vue 版本")) &&
        /^\d+\.\d+/.test(valueOf("Vuetify 版本")),
      "步骤 21 Rust/Tauri/Vue/Vuetify 版本号非空",
      [valueOf("Rust 版本"), valueOf("Tauri 版本"), valueOf("Vue 版本"), valueOf("Vuetify 版本")].join(","),
    );
    report.check(
      dialog !== null && ui.subtreeText(dialog).includes("I-Net 是一款个人敏感数据管理软件"),
      "步骤 21 显示简介文本",
    );
  }
  await snap("s28-about-dialog");

  await clickDialogButton(DLG_ABOUT, "关闭");
  const aboutClosed = await ui
    .waitForDialogGone(DLG_ABOUT, { timeout: 8000 })
    .then(() => true)
    .catch(() => false);
  report.check(aboutClosed, "步骤 22 关于对话框关闭（全程未点击「GitHub 仓库」链接）");
  await snap("s29-after-about-close");

  report.section("S8 收尾");
  {
    await sleep(600);
    const tree = await api.uiTree();
    report.check(ui.findAttr(tree, "title", "搜索") !== null, "用例结束时界面仍为中文");
    report.check(
      ui.findAttr(tree, "aria-label", "切换语言") !== null &&
        ui.findAttr(tree, "aria-label", "设置") !== null &&
        ui.findAttr(tree, "aria-label", "切换主题") !== null &&
        ui.findAttr(tree, "aria-label", "关于") !== null,
      "用例结束时右上角四个入口仍可交互",
    );
    const brightness = await canvasBrightness();
    report.check(brightness >= BRIGHTNESS_LIGHT_MIN, "用例结束时主题恢复为亮色", `brightness=${brightness.toFixed(1)}`);
    report.check(ui.hasText(tree, "账号 A"), "用例结束时根画布内容完好（「账号 A」仍在）");
  }
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
