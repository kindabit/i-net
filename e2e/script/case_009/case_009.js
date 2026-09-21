// case_009 模板与模板面板。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）
// → 模板面板空态（「新建节点」打开：仅「空节点」行与两个拖拽手柄、管理模板入口）
// → 模板管理对话框：F1 空名禁用 → 新建「登录凭证」→ 添加字段「用户名」与「密码」
//   （F3 空名/重名保存拦截并标红）→ 保存（snackbar + 保存按钮转禁用）
//   → 重命名「登录凭证 2」→ F2 重名新建被拒（snackbar）→ F4 关闭时未保存确认
//   （取消留在编辑器 / 确认后放弃）→ 重开验证放弃 → F4 切换模板时机（取消留在当前模板）
// → 从「账号 A」保存为模板：添加字段「邮箱」→「保存为模板」→「账号模板」→ snackbar
//   → 编辑对话框保持打开 → 模板面板出现两行模板
// → 拖拽创建数据节点（副标题为模板名）→ 编辑对话框验证字段结构继承（值按类型为空）
// → 拖拽创建画布数据节点「新画布」→ 双击进入子画布验证 → 面包屑返回根画布
// → 导出模板（默认文件名 templates.sqlite）→ 断言输出文件存在且为 SQLite 格式
// → 删除「账号模板」→ F5 导入确认取消 → 导入恢复「账号模板」→ 面板同步更新
// → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 模板面板行与画布节点副标题都会渲染「空节点」文本：面板内元素用 left < 400 判定；
//   面板行的拖拽手柄按行文本垂直中心定位（「空节点」行与模板行的手柄 title 相同）；
// - 模板管理对话框为嵌套结构（名称对话框 / 确认框 / 文件选择器）：名称对话框用「模板名称」
//   label 定位，确认框用含精确标题文本的最小 dialog 定位；
// - 模板字段行以字段名输入框为锚点定位（模板字段名为普通输入框，值不在 UI 树，读值走剪贴板；
//   类型下拉为行内与输入框垂直中心接近的 combobox；删除按钮为行内最右侧 button）；
// - 子画布导航：双击「新画布」进入，面包屑「根画布」为 link 节点（无对应 #text）；
// - 导出/导入走文件选择器：路径框输入目录后回车导航，open 模式双击文件；
//   文件列表为虚拟滚动（VVirtualScroll，视口外的行不在 UI 树中）：目标文件在长目录列表中
//   不可见时需滚动至可见（scrollPickerToFile）；导出默认文件名用 Ctrl+A/Ctrl+C 复制后读取剪贴板验证；
// - 全部等待使用 lib 等待函数；应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_009\case_009.js`

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

/** 导出模板产出的 SQLite 文件路径 */
const EXPORT_SQLITE = path.join(OUTPUT_DIR, "templates.sqlite");
/** 导出/导入文件选择器中的模板文件名 */
const EXPORT_NAME = "templates.sqlite";

/** 字段类型下拉的显示名集合（用于从对话框 combobox 中识别类型下拉）。 */
const FIELD_TYPE_NAMES = [
  "单行文本",
  "邮箱地址",
  "网址",
  "多行文本",
  "密码",
  "单行敏感文本",
  "数字",
  "时间点",
  "时间区间",
];

/** 模板面板内文本的 left 上限（面板位于窗口左上角，宽约 260px） */
const PANEL_MAX_LEFT = 400;

/** 输入框标红断言下限：输入框 bounds 内红色像素数 */
const RED_TINT_MIN = 20;

const report = new Report("case_009 模板与模板面板");

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
 * 返回模板面板区域内的指定文本节点（按 top、left 升序）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 目标文本。
 * @returns {object[]} 面板区域内的文本节点数组。
 */
function panelTextNodes(tree, text) {
  return textNodes(tree, text).filter((node) => node.bounds.left < PANEL_MAX_LEFT);
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
 * 在 UI 树中查找模板管理对话框（含「模板管理」文本的最小 dialog）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 模板管理对话框；未命中时为 null。
 */
function templateDialog(tree) {
  return ui.findDialog(tree, "模板管理");
}

/**
 * 在 UI 树中查找名称输入对话框（特征：包含名称为「模板名称」的可编辑控件）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 名称输入对话框；未命中时为 null。
 */
function nameDialogOf(tree) {
  return ui.findDialog(tree, "模板名称");
}

/**
 * 在 UI 树中查找文件选择器对话框（特征：包含名称为「路径」的可编辑控件）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 文件选择器对话框；未命中时为 null。
 */
function dialogWithPathInput(tree) {
  return ui.findByRole(tree, "dialog").find((d) => ui.findEditable([d], "路径") !== null) ?? null;
}

/**
 * 在 UI 树中查找 role=option 且文本精确匹配的候选项。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 候选项文本。
 * @returns {object|null} 命中的候选项；未命中时为 null。
 */
function findOption(tree, text) {
  return flattenNodes(tree).find((node) => node.role === "option" && (node.text ?? "").trim() === text) ?? null;
}

/**
 * 等待 role=option 且文本精确匹配的候选项出现。
 * @param {string} text 候选项文本。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=6000] 超时毫秒数。
 * @returns {Promise<object>} 命中的候选项。
 */
async function waitForOption(text, { timeout = 6000 } = {}) {
  return waitForCondition(async () => findOption(await api.uiTree(), text), {
    timeout,
    interval: 200,
    label: `option "${text}"`,
  });
}

/**
 * 解析节点编辑对话框中指定字段名文本对应的行元素集合（节点字段名称为 #text 节点）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} nameText 字段名文本节点（#text）。
 * @returns {{name: object, buttons: object[], typeCombo: object|null, valueEditor: object|null, copyButton: object|null, icons: object[]}|null} 行元素集合；名称缺失时为 null。
 */
function fieldRow(tree, nameText) {
  if (!nameText) {
    return null;
  }
  const nodes = flattenNodes(tree);
  const b = nameText.bounds;
  const overlapY = (node, pad) =>
    node.bounds.top < b.top + b.height + pad && node.bounds.top + node.bounds.height > b.top - pad;
  const buttons = nodes
    .filter((node) => node.role === "button" && overlapY(node, 20) && node.bounds.left > b.left + b.width)
    .sort((a, c) => a.bounds.left - c.bounds.left);
  const typeCombo =
    nodes
      .filter(
        (node) =>
          node.role === "combobox" &&
          FIELD_TYPE_NAMES.includes((node.text ?? "").trim()) &&
          node.bounds.top > b.top + 10,
      )
      .sort((a, c) => a.bounds.top - c.bounds.top)[0] ?? null;
  if (!typeCombo) {
    return { name: nameText, buttons, typeCombo: null, valueEditor: null, copyButton: null, icons: [] };
  }
  const c = typeCombo.bounds;
  const rowOverlap = (node) => node.bounds.top < c.top + c.height && node.bounds.top + node.bounds.height > c.top;
  const valueEditor =
    nodes.find(
      (node) =>
        node !== typeCombo &&
        (node.role === "textbox" || node.role === "combobox") &&
        rowOverlap(node) &&
        node.bounds.left >= c.left + c.width - 5,
    ) ?? null;
  const valueRight = valueEditor ? valueEditor.bounds.left + valueEditor.bounds.width - 5 : c.left + c.width;
  const copyButton =
    nodes
      .filter((node) => node.tag === "BUTTON" && node.role === "button" && rowOverlap(node) && node.bounds.left >= valueRight)
      .sort((a, c2) => c2.bounds.left - a.bounds.left)[0] ?? null;
  const icons = nodes
    .filter((node) => node.tag === "I" && node.role === "button" && rowOverlap(node))
    .sort((a, c2) => a.bounds.left - c2.bounds.left);
  return { name: nameText, buttons, typeCombo, valueEditor, copyButton, icons };
}

/**
 * 返回模板管理对话框中的字段名输入框（按 top 升序）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 字段名输入框数组。
 */
function fieldNameInputs(tree) {
  const dialog = templateDialog(tree);
  if (!dialog) {
    return [];
  }
  return flattenNodes([dialog])
    .filter((node) => node.editable && ui.ownText(node).includes("字段名"))
    .sort((a, b) => a.bounds.top - b.bounds.top);
}

/**
 * 返回模板字段行内与字段名输入框垂直中心接近的 combobox（类型下拉）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} nameInput 字段名输入框节点。
 * @returns {object|null} 类型下拉 combobox；未命中时为 null。
 */
function templateRowCombo(tree, nameInput) {
  return (
    flattenNodes([templateDialog(tree)])
      .filter((node) => node.role === "combobox" && Math.abs(node.bounds.top - nameInput.bounds.top) < 15)
      .sort((a, b) => a.bounds.left - b.bounds.left)[0] ?? null
  );
}

/**
 * 返回模板字段行内位于字段名输入框右侧的按钮（按 left 升序）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} nameInput 字段名输入框节点。
 * @returns {object[]} 行内按钮数组（升序，最后一个为删除按钮）。
 */
function templateRowButtons(tree, nameInput) {
  const cy = nameInput.bounds.top + nameInput.bounds.height / 2;
  return flattenNodes(tree)
    .filter(
      (node) =>
        node.role === "button" &&
        Math.abs(node.bounds.top + node.bounds.height / 2 - cy) < 30 &&
        node.bounds.left > nameInput.bounds.left + nameInput.bounds.width,
    )
    .sort((a, b) => a.bounds.left - b.bounds.left);
}

/**
 * 点击模板管理对话框内按钮（对话框内查找以避免与嵌套对话框同名按钮混淆）。
 * @param {string} buttonText 按钮文本。
 * @param {object} [options] 可选参数。
 * @param {boolean} [options.exact=false] 是否精确匹配按钮子树文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickTemplateDialogButton(buttonText, { exact = false } = {}) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = templateDialog(tree);
      return dialog ? ui.findButton(tree, buttonText, { root: dialog, exact }) : null;
    },
    { timeout: 8000, interval: 250, label: `模板管理按钮「${buttonText}」` },
  );
  await ui.clickNode(button);
}

/**
 * 点击名称输入对话框内按钮。
 * @param {string} buttonText 按钮文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickNameDialogButton(buttonText) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = nameDialogOf(tree);
      return dialog ? ui.findButton(tree, buttonText, { root: dialog, exact: true }) : null;
    },
    { timeout: 8000, interval: 250, label: `名称对话框按钮「${buttonText}」` },
  );
  await ui.clickNode(button);
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
    { timeout: 20000, label: "home page name input" },
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
 * 点击悬浮菜单「新建节点」打开模板面板并等待「空节点」行出现。
 * @returns {Promise<void>} 无返回值。
 */
async function openTemplatePanel() {
  const button = await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "新建节点"), {
    timeout: 8000,
    interval: 200,
    label: "「新建节点」按钮",
  });
  await ui.clickNode(button);
  await waitForCondition(async () => panelTextNodes(await api.uiTree(), "空节点").length > 0, {
    timeout: 6000,
    interval: 200,
    label: "模板面板打开",
  });
  await sleep(600);
}

/**
 * 点击面板「管理模板」按钮打开模板管理对话框并等待其出现。
 * @returns {Promise<void>} 无返回值。
 */
async function clickManageTemplates() {
  const button = await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "管理模板"), {
    timeout: 8000,
    interval: 200,
    label: "「管理模板」按钮",
  });
  await ui.clickNode(button);
  await waitForCondition(async () => templateDialog(await api.uiTree()) !== null, {
    timeout: 8000,
    interval: 250,
    label: "模板管理对话框",
  });
  await sleep(700);
}

/**
 * 打开模板面板并进入模板管理对话框。
 * @returns {Promise<void>} 无返回值。
 */
async function openTemplateManager() {
  await openTemplatePanel();
  await clickManageTemplates();
}

/**
 * 点击模板管理对话框「添加字段」并在新增字段行输入字段名。
 * @param {string} name 字段名。
 * @returns {Promise<void>} 无返回值。
 */
async function addTemplateField(name) {
  await clickTemplateDialogButton("添加字段");
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return fieldNameInputs(tree).length > 0 ? tree : null;
    },
    { timeout: 5000, interval: 200, label: "模板字段行出现" },
  );
  const inputs = fieldNameInputs(await api.uiTree());
  await ui.clickNode(inputs[inputs.length - 1]);
  await api.postInput([api.typeText(name)]);
  await sleep(400);
}

/**
 * 删除指定的模板字段行（点击行内最右侧按钮）。
 * @param {object} nameInput 字段名输入框节点。
 * @returns {Promise<void>} 无返回值。
 */
async function removeTemplateFieldRow(nameInput) {
  const buttons = templateRowButtons(await api.uiTree(), nameInput);
  if (buttons.length === 0) {
    throw new Error("removeTemplateFieldRow: no row buttons found");
  }
  await ui.clickNode(buttons[buttons.length - 1]);
  await sleep(500);
}

/**
 * 切换模板字段行的类型：点击该行类型下拉后选择目标类型。
 * @param {object} nameInput 字段名输入框节点。
 * @param {string} typeName 目标类型显示名（如「密码」）。
 * @returns {Promise<void>} 无返回值。
 */
async function selectTemplateFieldType(nameInput, typeName) {
  const combo = templateRowCombo(await api.uiTree(), nameInput);
  if (!combo) {
    throw new Error(`selectTemplateFieldType: type combo not found for row at top ${nameInput.bounds.top}`);
  }
  await ui.clickNode(combo);
  const option = await waitForOption(typeName);
  await ui.clickNode(option);
  await sleep(500);
}

/**
 * 在模板管理对话框的模板列表中点击指定模板项（按左栏区域过滤）。
 * @param {string} name 模板名称。
 * @returns {Promise<void>} 无返回值。
 */
async function clickTemplateListItem(name) {
  const item = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = templateDialog(tree);
      if (!dialog) {
        return null;
      }
      return textNodes([dialog], name).filter((n) => n.bounds.left < dialog.bounds.left + 300)[0] ?? null;
    },
    { timeout: 6000, interval: 200, label: `模板列表项「${name}」` },
  );
  await ui.clickNode(item);
  await sleep(700);
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
async function readEditableValue(editable) {
  await ui.clickNode(editable);
  await api.postInput([api.keyClick(["Control", "a"]), api.keyClick(["Control", "c"])]);
  await sleep(400);
  return readClipboard();
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
      if (r > 150 && r > g + 50 && r > b + 50) {
        count += 1;
      }
    }
  }
  return count;
}

/**
 * 返回模板面板中指定行的拖拽手柄（按 title 与行文本垂直中心接近定位）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} rowText 行文本节点（模板名或「空节点」）。
 * @param {string} title 手柄 title（拖拽创建数据节点 / 拖拽创建画布数据节点）。
 * @returns {object|null} 命中的手柄；未命中时为 null。
 */
function panelHandle(tree, rowText, title) {
  const cy = rowText.bounds.top + rowText.bounds.height / 2;
  return (
    flattenNodes(tree)
      .filter((node) => node.attrs?.title === title)
      .filter((node) => Math.abs(node.bounds.top + node.bounds.height / 2 - cy) < 24)
      .sort((a, b) => a.bounds.left - b.bounds.left)[0] ?? null
  );
}

/**
 * 在画布区域中选取一个不靠近任何文本节点的空白落点。
 * @returns {Promise<{x: number, y: number}>} 空白点坐标（窗口物理像素）。
 */
async function pickBlankPoint() {
  const info = await api.health();
  const tree = await api.uiTree();
  const texts = flattenNodes(tree).filter((n) => n.tag === "#text" && n.bounds.width > 0);
  const pad = 100;
  for (let y = 320; y < info.height - 220; y += 90) {
    for (let x = 520; x < info.width - 260; x += 90) {
      const hit = texts.some(
        (n) =>
          x > n.bounds.left - pad &&
          x < n.bounds.left + n.bounds.width + pad &&
          y > n.bounds.top - pad &&
          y < n.bounds.top + n.bounds.height + pad,
      );
      if (!hit) {
        return { x, y };
      }
    }
  }
  throw new Error("pickBlankPoint: no blank point found");
}

/**
 * 从模板面板指定行的拖拽手柄拖拽到目标点（模板面板行文本按面板区域过滤）。
 * @param {string} rowName 模板行名称。
 * @param {string} handleTitle 手柄 title。
 * @param {{x: number, y: number}} to 目标点坐标。
 * @returns {Promise<void>} 无返回值。
 */
async function dragTemplateRow(rowName, handleTitle, to) {
  const rowText = await waitForCondition(
    async () => panelTextNodes(await api.uiTree(), rowName)[0] ?? null,
    { timeout: 6000, interval: 200, label: `模板面板行「${rowName}」` },
  );
  const handle = panelHandle(await api.uiTree(), rowText, handleTitle);
  if (!handle) {
    throw new Error(`dragTemplateRow: handle "${handleTitle}" not found for "${rowName}"`);
  }
  await api.postInput([api.mouseDrag(ui.boundsCenter(handle), to)]);
  await sleep(1200);
}

/**
 * 在画布中双击「账号 A」打开编辑节点对话框。
 * @returns {Promise<void>} 无返回值。
 */
async function openAccountDialog() {
  await ui.clickText("账号 A", { count: 2 });
  await ui.waitForDialog("编辑节点", { timeout: 8000 });
  await sleep(600);
}

/**
 * 关闭编辑节点对话框（点击「取消」），并等待关闭。
 * @returns {Promise<void>} 无返回值。
 */
async function closeEditDialog() {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = ui.findDialog(tree, "编辑节点");
      return dialog ? ui.findButton(tree, "取消", { root: dialog, exact: true }) : null;
    },
    { timeout: 8000, interval: 250, label: "编辑节点取消按钮" },
  );
  await ui.clickNode(button);
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(500);
}

/**
 * 在编辑节点对话框中添加字段并输入字段名：点击「添加字段」→ 单击占位符进入名称编辑态 → 输入 → Enter 提交。
 * @param {string} name 字段名。
 * @returns {Promise<void>} 无返回值。
 */
async function addNodeField(name) {
  await ui.clickNode(ui.findButton(await api.uiTree(), "添加字段", { exact: true }));
  const placeholder = await waitForCondition(
    async () => {
      const list = textNodes(await api.uiTree(), "未命名字段");
      return list.length > 0 ? list[list.length - 1] : null;
    },
    { timeout: 5000, interval: 150, label: "new field placeholder" },
  );
  const nameTop = placeholder.bounds.top;
  await ui.clickNode(placeholder);
  await waitForCondition(
    async () =>
      flattenNodes(await api.uiTree()).find(
        (node) =>
          node.role === "textbox" &&
          node.tag === "INPUT" &&
          Math.abs(node.bounds.top - nameTop) < 25 &&
          node.bounds.left < 600,
      ) ?? null,
    { timeout: 4000, interval: 150, label: "name edit input" },
  );
  await api.postInput([api.typeText(name), api.keyClick(["Enter"])]);
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const editing = flattenNodes(t).some(
        (node) =>
          node.role === "textbox" &&
          node.tag === "INPUT" &&
          Math.abs(node.bounds.top - nameTop) < 25 &&
          node.bounds.left < 600,
      );
      return !editing && textNodes(t, name).length > 0;
    },
    { timeout: 5000, interval: 200, label: `field renamed to "${name}"` },
  );
}

/**
 * 在文件选择器中输入目录路径、可选文件名，并点击「确认」。
 * @param {string} dir 目录绝对路径（路径框内容）。
 * @param {string|null} fileName 目标文件名；null 表示不填写文件名（保留默认值）。
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
    const fresh = dialogWithPathInput(await api.uiTree());
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
 * 返回文件选择器列表区域内的行文本节点（按 top 升序）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {object} picker 文件选择器对话框节点。
 * @returns {object[]} 行文本节点数组。
 */
function pickerRowTexts(tree, picker) {
  const pathInput = ui.findEditable([picker], "路径");
  const top = pathInput.bounds.top + pathInput.bounds.height + 8;
  const bottom = top + 480;
  return flattenNodes([picker])
    .filter((n) => n.tag === "#text" && n.bounds.width > 0 && n.bounds.top > top && n.bounds.top < bottom)
    .sort((a, b) => a.bounds.top - b.bounds.top);
}

/**
 * 在文件选择器列表中滚动直至目标文件行可见（列表为虚拟滚动，视口外的行不在 UI 树中）。
 * @param {string} name 目标文件名。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=20000] 超时毫秒数。
 * @returns {Promise<object>} 目标文件行文本节点。
 */
async function scrollPickerToFile(name, { timeout = 20000 } = {}) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const tree = await api.uiTree();
    const picker = dialogWithPathInput(tree);
    if (!picker) {
      throw new Error("scrollPickerToFile: file picker not found");
    }
    const hit = ui.findByText([picker], name, { exact: true });
    if (hit) {
      return hit;
    }
    const rows = pickerRowTexts(tree, picker);
    const last = rows[rows.length - 1];
    if (last) {
      const x = picker.bounds.left + picker.bounds.width / 2;
      const y = last.bounds.top + last.bounds.height / 2;
      await api.postInput([api.mouseMove(x, y), api.mouseScroll("down", 5)]);
    }
    await sleep(350);
    if (Date.now() > deadline) {
      throw new Error(`scrollPickerToFile("${name}") timed out after ${timeout}ms`);
    }
  }
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
  await unlock();
  const unlockedTree = await api.uiTree();
  report.check(ui.hasText(unlockedTree, "账号 A"), "解锁后进入根画布并出现节点「账号 A」");
  await sleep(600);
  await snap("s0-unlocked-root");

  report.section("S1 模板面板与空态（步骤 1）");
  await openTemplatePanel();
  let tree = await api.uiTree();
  report.check(panelTextNodes(tree, "空节点").length === 1, "步骤 1 面板出现「空节点」行（列表为空仅有空节点）");
  report.check(
    titledNodes(tree, "拖拽创建数据节点").length === 1 && titledNodes(tree, "拖拽创建画布数据节点").length === 1,
    "步骤 1 空节点行带两个拖拽手柄（数据节点 / 画布数据节点）",
    `data=${titledNodes(tree, "拖拽创建数据节点").length} canvas=${titledNodes(tree, "拖拽创建画布数据节点").length}`,
  );
  report.check(ui.findAttr(tree, "title", "管理模板") !== null, "步骤 1 面板含「管理模板」按钮");
  await snap("s1-template-panel-empty");

  report.section("S2 打开模板管理对话框（步骤 2）");
  await clickManageTemplates();
  tree = await api.uiTree();
  report.check(panelTextNodes(tree, "空节点").length === 0, "步骤 2 点击「管理模板」后模板面板关闭");
  report.check(templateDialog(tree) !== null, "步骤 2 打开对话框「模板管理」");
  report.check(
    ui.findButton(tree, "新建模板", { root: templateDialog(tree), exact: true }) !== null,
    "步骤 2 左栏含「新建模板」按钮",
  );
  report.check(ui.hasText(tree, "请选择或新建一个模板"), "步骤 2 右侧提示「请选择或新建一个模板」");
  await snap("s2-template-manager");

  report.section("S3 F1 空名校验与新建模板（步骤 3-4）");
  await clickTemplateDialogButton("新建模板");
  await waitForCondition(async () => nameDialogOf(await api.uiTree()) !== null, {
    timeout: 6000,
    interval: 200,
    label: "名称对话框",
  });
  await sleep(600);
  tree = await api.uiTree();
  {
    const nameDialog = nameDialogOf(tree);
    report.check(
      ui.findByText([nameDialog], "新建模板", { exact: true }) !== null,
      '步骤 3 弹出名称输入对话框，标题「新建模板」',
    );
    report.check(ui.findEditable([nameDialog], "模板名称") !== null, "步骤 3 名称输入框 label 为「模板名称」");
    const confirm = ui.findButton(tree, "确认", { root: nameDialog, exact: true });
    report.check(confirm?.states?.disabled === true, "F1 名称为空时「确认」按钮禁用（无法提交）");
  }
  await snap("s3-f1-name-required");
  {
    const input = ui.findEditable([nameDialogOf(await api.uiTree())], "模板名称");
    await ui.clickNode(input);
    await api.postInput([api.typeText("登录凭证")]);
    await sleep(400);
    await clickNameDialogButton("确认");
  }
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const dialog = templateDialog(t);
      return dialog && textNodes([dialog], "登录凭证").length >= 2 ? t : null;
    },
    { timeout: 6000, interval: 250, label: "模板「登录凭证」创建并选中" },
  );
  await sleep(800);
  tree = await api.uiTree();
  {
    const dialog = templateDialog(tree);
    const inList = textNodes([dialog], "登录凭证").filter((n) => n.bounds.left < dialog.bounds.left + 300);
    const inTitle = textNodes([dialog], "登录凭证").filter((n) => n.bounds.left >= dialog.bounds.left + 300);
    report.check(inList.length === 1 && inTitle.length === 1, "步骤 4 列表出现「登录凭证」且被选中（列表项与右侧标题）");
  }
  report.check(fieldNameInputs(tree).length === 0, "步骤 4 新模板字段区为空");
  await snap("s4-template-created");

  report.section("S4 模板字段结构与 F3（步骤 5-7）");
  await addTemplateField("用户名");
  tree = await api.uiTree();
  {
    const inputs = fieldNameInputs(tree);
    report.check(inputs.length === 1, "步骤 5 点击「添加字段」后出现 1 个字段行");
    const value = await readEditableValue(inputs[0]);
    report.check(value === "用户名", "步骤 5 字段名输入框内容为「用户名」（剪贴板验证）", `clip=${value}`);
    tree = await api.uiTree();
    const combo = templateRowCombo(tree, fieldNameInputs(tree)[0]);
    report.check((combo?.text ?? "").trim() === "单行文本", "步骤 5 字段类型默认「单行文本」", combo?.text);
  }
  await snap("s5-field-username");

  // F3a：空字段名保存被拦截
  await clickTemplateDialogButton("添加字段");
  await sleep(500);
  await clickTemplateDialogButton("保存", { exact: true });
  const f3a = await ui.waitForTextStable("字段名不能为空", { timeout: 6000 }).catch(() => null);
  report.check(f3a !== null, "F3 空字段名保存提示「字段名不能为空」");
  await sleep(500);
  tree = await api.uiTree();
  report.check(templateDialog(tree) !== null, "F3 空字段名保存被阻止，对话框未关闭");
  {
    const inputs = fieldNameInputs(tree);
    const emptyInput = inputs[1] ?? inputs[0];
    const png = image.decodePng(await api.screenshot());
    const red = emptyInput ? redPixelsIn(png, emptyInput.bounds) : 0;
    report.check(red >= RED_TINT_MIN, "F3 空名字段行的字段名输入框标红（像素断言）", `redPixels=${red}`);
  }
  await snap("s6-f3-empty-name");
  {
    const inputs = fieldNameInputs(await api.uiTree());
    await removeTemplateFieldRow(inputs[inputs.length - 1]);
  }
  report.check(fieldNameInputs(await api.uiTree()).length === 1, "F3 删除空字段行后仅剩 1 行");

  // 步骤 6：添加「密码」字段并切换类型
  await addTemplateField("密码");
  {
    tree = await api.uiTree();
    const inputs = fieldNameInputs(tree);
    const value = await readEditableValue(inputs[inputs.length - 1]);
    report.check(value === "密码", "步骤 6 第二个字段名输入框内容为「密码」（剪贴板验证）", `clip=${value}`);
    tree = await api.uiTree();
    await selectTemplateFieldType(fieldNameInputs(tree)[1], "密码");
    tree = await api.uiTree();
    const combo = templateRowCombo(tree, fieldNameInputs(tree)[1]);
    report.check((combo?.text ?? "").trim() === "密码", "步骤 6 第二行类型下拉已切换为「密码」", combo?.text);
  }
  await snap("s6b-field-password");

  // F3b：重复字段名保存被拦截
  await addTemplateField("用户名");
  await clickTemplateDialogButton("保存", { exact: true });
  const f3b = await ui.waitForTextStable("字段名重复", { timeout: 6000 }).catch(() => null);
  report.check(f3b !== null, "F3 重复字段名保存提示「字段名重复」");
  await sleep(500);
  tree = await api.uiTree();
  report.check(templateDialog(tree) !== null, "F3 重复字段名保存被阻止，对话框未关闭");
  {
    const inputs = fieldNameInputs(tree);
    const duplicated = [inputs[0], inputs[inputs.length - 1]];
    const png = image.decodePng(await api.screenshot());
    const reds = duplicated.map((n) => (n ? redPixelsIn(png, n.bounds) : 0));
    report.check(reds.every((count) => count >= RED_TINT_MIN), "F3 两个同名字段行的字段名均标红（像素断言）", JSON.stringify(reds));
  }
  await snap("s7-f3-duplicated");
  {
    const inputs = fieldNameInputs(await api.uiTree());
    await removeTemplateFieldRow(inputs[inputs.length - 1]);
  }
  report.check(fieldNameInputs(await api.uiTree()).length === 2, "F3 删除重复字段行后回到 2 行");

  // 步骤 7：保存模板
  await clickTemplateDialogButton("保存", { exact: true });
  const savedSnack = await ui.waitForTextStable("模板已保存", { timeout: 8000 }).catch(() => null);
  report.check(savedSnack !== null, "步骤 7 保存提示「模板已保存」");
  await sleep(800);
  tree = await api.uiTree();
  {
    const saveButton = ui.findButton(tree, "保存", { root: templateDialog(tree), exact: true });
    report.check(saveButton?.states?.disabled === true, "步骤 7 保存后「保存」按钮变为不可用（无未保存修改）");
  }
  await snap("s8-template-saved");

  report.section("S5 重命名与 F2 重名（步骤 8 + F2）");
  {
    const button = await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "重命名模板"), {
      timeout: 8000,
      interval: 200,
      label: "「重命名模板」按钮",
    });
    await ui.clickNode(button);
  }
  await waitForCondition(async () => nameDialogOf(await api.uiTree()) !== null, {
    timeout: 6000,
    interval: 200,
    label: "名称对话框（重命名）",
  });
  await sleep(600);
  tree = await api.uiTree();
  report.check(
    ui.findByText([nameDialogOf(tree)], "重命名模板", { exact: true }) !== null,
    "步骤 8 弹出名称输入对话框，标题「重命名模板」",
  );
  await snap("s9-rename-dialog");
  {
    const input = ui.findEditable([nameDialogOf(await api.uiTree())], "模板名称");
    const initial = await readEditableValue(input);
    report.check(initial === "登录凭证", "步骤 8 重命名对话框预填当前模板名「登录凭证」（剪贴板验证）", `clip=${initial}`);
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText("登录凭证 2")]);
    await sleep(400);
    await clickNameDialogButton("确认");
  }
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const dialog = templateDialog(t);
      return dialog && textNodes([dialog], "登录凭证 2").length >= 2 ? t : null;
    },
    { timeout: 6000, interval: 250, label: "重命名生效" },
  );
  await sleep(800);
  tree = await api.uiTree();
  {
    const dialog = templateDialog(tree);
    const inList = textNodes([dialog], "登录凭证 2").filter((n) => n.bounds.left < dialog.bounds.left + 300);
    const inTitle = textNodes([dialog], "登录凭证 2").filter((n) => n.bounds.left >= dialog.bounds.left + 300);
    report.check(inList.length === 1 && inTitle.length === 1, "步骤 8 列表项与右侧标题均更新为「登录凭证 2」");
    report.check(textNodes([dialog], "登录凭证").length === 0, "步骤 8 旧名「登录凭证」不再出现");
  }
  await snap("s10-renamed");

  // F2：与已有模板重名
  await clickTemplateDialogButton("新建模板");
  await waitForCondition(async () => nameDialogOf(await api.uiTree()) !== null, {
    timeout: 6000,
    interval: 200,
    label: "名称对话框（F2）",
  });
  await sleep(500);
  {
    const input = ui.findEditable([nameDialogOf(await api.uiTree())], "模板名称");
    await ui.clickNode(input);
    await api.postInput([api.typeText("登录凭证 2")]);
    await sleep(400);
    await clickNameDialogButton("确认");
  }
  const f2Snack = await ui.waitForTextStable("模板名称已存在", { timeout: 8000 }).catch(() => null);
  await sleep(300);
  tree = await api.uiTree();
  report.check(f2Snack !== null, "F2 重名新建提示「模板名称已存在」");
  report.check(ui.hasText(tree, "名称：登录凭证 2"), "F2 提示正文「名称：登录凭证 2」");
  {
    const dialog = templateDialog(tree);
    const inList = dialog ? textNodes([dialog], "登录凭证 2").filter((n) => n.bounds.left < dialog.bounds.left + 300) : [];
    report.check(inList.length === 1, "F2 重名模板未被创建（列表仍只有「登录凭证 2」）");
  }
  await snap("s11-f2-duplicate-name");

  report.section("S6 未保存修改确认 F4（步骤 9-10）");
  await clickTemplateDialogButton("添加字段");
  await sleep(500);
  await clickTemplateDialogButton("取消");
  await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "未保存的修改") !== null, {
    timeout: 8000,
    interval: 250,
    label: "「未保存的修改」确认框",
  });
  await sleep(600);
  tree = await api.uiTree();
  report.check(ui.hasText(tree, "未保存的修改"), "步骤 9 关闭时弹出确认框「未保存的修改」");
  report.check(
    ui.hasText(tree, "当前编辑有未保存的修改，确定要放弃吗？"),
    "步骤 9 确认框正文与预期一致",
  );
  await snap("s12-unsaved-confirm");
  await clickConfirmDialogButton("未保存的修改", "取消");
  await sleep(800);
  tree = await api.uiTree();
  report.check(templateDialog(tree) !== null, "步骤 10 确认框点「取消」后留在编辑器（对话框仍打开）");
  report.check(fieldNameInputs(tree).length === 3, "步骤 10 未保存的字段行仍保留", `rows=${fieldNameInputs(tree).length}`);
  await snap("s13-unsaved-cancel-stay");
  await clickTemplateDialogButton("取消");
  await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "未保存的修改") !== null, {
    timeout: 8000,
    interval: 250,
    label: "「未保存的修改」确认框（二次）",
  });
  await sleep(500);
  await clickConfirmDialogButton("未保存的修改", "确认");
  await ui.waitForDialogGone("模板管理", { timeout: 8000 });
  await sleep(700);
  report.check(true, "步骤 10 确认框点「确认」后对话框关闭、修改被放弃");
  await openTemplateManager();
  tree = await api.uiTree();
  report.check(fieldNameInputs(tree).length === 2, "步骤 10 重开对话框后字段仍为 2 行（未保存修改未写库）", `rows=${fieldNameInputs(tree).length}`);
  await snap("s14-reopen-verified");
  await clickTemplateDialogButton("取消");
  await ui.waitForDialogGone("模板管理", { timeout: 8000 });
  await sleep(600);

  report.section("S7 从节点保存为模板（步骤 11-14）");
  await openAccountDialog();
  await addNodeField("邮箱");
  tree = await api.uiTree();
  {
    const dialog = ui.findDialog(tree, "编辑节点");
    report.check(textNodes([dialog], "邮箱").length === 1, "步骤 11 「账号 A」编辑对话框出现新字段行「邮箱」");
  }
  await snap("s15-node-field-email");
  {
    const button = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        const dialog = ui.findDialog(t, "编辑节点");
        return dialog ? ui.findButton(t, "保存为模板", { root: dialog, exact: true }) : null;
      },
      { timeout: 6000, interval: 250, label: "「保存为模板」按钮" },
    );
    await ui.clickNode(button);
  }
  await waitForCondition(async () => nameDialogOf(await api.uiTree()) !== null, {
    timeout: 10000,
    interval: 250,
    label: "「保存为模板」名称对话框",
  });
  await sleep(700);
  tree = await api.uiTree();
  {
    const nameDialog = nameDialogOf(tree);
    report.check(
      ui.findByText([nameDialog], "保存为模板", { exact: true }) !== null,
      "步骤 12 名称对话框标题「保存为模板」",
    );
    report.check(ui.findEditable([nameDialog], "模板名称") !== null, "步骤 12 名称输入框 label「模板名称」");
    report.check(
      ui.findButton(tree, "保存为模板", { root: nameDialog, exact: true }) !== null,
      "步骤 12 确认按钮为「保存为模板」",
    );
  }
  await snap("s16-save-as-template-dialog");
  {
    const input = ui.findEditable([nameDialogOf(await api.uiTree())], "模板名称");
    await ui.clickNode(input);
    await api.postInput([api.typeText("账号模板")]);
    await sleep(400);
    await clickNameDialogButton("保存为模板");
  }
  const created = await ui.waitForTextStable("模板已创建", { timeout: 10000 }).catch(() => null);
  report.check(created !== null, "步骤 13 提示「模板已创建」");
  await sleep(800);
  tree = await api.uiTree();
  report.check(nameDialogOf(tree) === null, "步骤 13 名称对话框关闭");
  report.check(ui.findDialog(tree, "编辑节点") !== null, "步骤 13 编辑节点对话框保持打开");
  await snap("s17-template-created-from-node");
  await closeEditDialog();
  await openTemplatePanel();
  await waitForCondition(async () => panelTextNodes(await api.uiTree(), "账号模板").length > 0, {
    timeout: 6000,
    interval: 250,
    label: "面板出现「账号模板」",
  });
  tree = await api.uiTree();
  report.check(panelTextNodes(tree, "登录凭证 2").length === 1 && panelTextNodes(tree, "账号模板").length === 1, "步骤 14 面板出现「登录凭证 2」「账号模板」两行");
  report.check(
    titledNodes(tree, "拖拽创建数据节点").length === 3 && titledNodes(tree, "拖拽创建画布数据节点").length === 3,
    "步骤 14 两个模板行各带两个拖拽手柄（含空节点行共 3 组）",
    `data=${titledNodes(tree, "拖拽创建数据节点").length} canvas=${titledNodes(tree, "拖拽创建画布数据节点").length}`,
  );
  await snap("s18-panel-two-templates");

  report.section("S7b F4 切换模板时机（F4）");
  await clickManageTemplates();
  tree = await api.uiTree();
  {
    const dialog = templateDialog(tree);
    const inTitle = dialog ? textNodes([dialog], "登录凭证 2").filter((n) => n.bounds.left >= dialog.bounds.left + 300) : [];
    report.check(inTitle.length === 1, "F4 打开模板管理后自动选中第一个模板「登录凭证 2」");
  }
  await clickTemplateDialogButton("添加字段");
  await sleep(500);
  await clickTemplateListItem("账号模板");
  await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "未保存的修改") !== null, {
    timeout: 8000,
    interval: 250,
    label: "切换模板时的「未保存的修改」确认框",
  });
  await sleep(600);
  tree = await api.uiTree();
  report.check(ui.hasText(tree, "未保存的修改"), "F4 有未保存修改时切换模板弹出确认框「未保存的修改」");
  await snap("s18b-f4-switch-confirm");
  await clickConfirmDialogButton("未保存的修改", "取消");
  await sleep(800);
  tree = await api.uiTree();
  {
    const dialog = templateDialog(tree);
    const inTitle = dialog ? textNodes([dialog], "登录凭证 2").filter((n) => n.bounds.left >= dialog.bounds.left + 300) : [];
    report.check(inTitle.length === 1, "F4 点「取消」后留在当前模板（右侧标题仍为「登录凭证 2」）");
  }
  await snap("s18c-f4-switch-cancel");
  await clickTemplateDialogButton("取消");
  await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "未保存的修改") !== null, {
    timeout: 8000,
    interval: 250,
    label: "「未保存的修改」确认框（切换后关闭）",
  });
  await sleep(500);
  await clickConfirmDialogButton("未保存的修改", "确认");
  await ui.waitForDialogGone("模板管理", { timeout: 8000 });
  await sleep(600);
  report.check(true, "F4 放弃修改后对话框关闭");

  report.section("S8 拖拽创建数据节点与字段继承（步骤 15-16）");
  await openTemplatePanel();
  {
    const blank = await pickBlankPoint();
    await dragTemplateRow("登录凭证 2", "拖拽创建数据节点", blank);
  }
  await waitForCondition(async () => textNodes(await api.uiTree(), "新节点").length > 0, {
    timeout: 8000,
    interval: 250,
    label: "新节点出现",
  });
  await sleep(800);
  tree = await api.uiTree();
  report.check(textNodes(tree, "新节点").length === 1, "步骤 15 画布出现节点「新节点」");
  report.check(textNodes(tree, "登录凭证 2").length === 1, "步骤 15 新节点副标题为模板名「登录凭证 2」");
  report.check(panelTextNodes(tree, "空节点").length === 0, "步骤 15 模板面板自动关闭");
  await snap("s19-drag-created-node");
  {
    const nodeText = await waitForCondition(async () => textNodes(await api.uiTree(), "新节点")[0] ?? null, {
      timeout: 6000,
      interval: 200,
      label: "「新节点」文本",
    });
    await ui.clickNode(nodeText, { count: 2 });
  }
  await ui.waitForDialog("编辑节点", { timeout: 8000 });
  await sleep(800);
  tree = await api.uiTree();
  {
    const dialog = ui.findDialog(tree, "编辑节点");
    const userText = textNodes([dialog], "用户名")[0];
    const pwText = textNodes([dialog], "密码")[0];
    report.check(userText !== undefined && pwText !== undefined, "步骤 16 编辑对话框包含字段「用户名」与「密码」");
    const userRow = fieldRow(tree, userText);
    const pwRow = fieldRow(tree, pwText);
    report.check((userRow?.typeCombo?.text ?? "").trim() === "单行文本", "步骤 16 「用户名」类型为「单行文本」", userRow?.typeCombo?.text);
    report.check((pwRow?.typeCombo?.text ?? "").trim() === "密码", "步骤 16 「密码」类型为「密码」", pwRow?.typeCombo?.text);
    report.check(
      userRow?.copyButton?.states?.disabled === true && pwRow?.copyButton?.states?.disabled === true,
      "步骤 16 两字段值均为空（复制按钮禁用）",
    );
    report.check(pwRow?.icons.length === 2, "步骤 16 「密码」行带显示/隐藏与生成器图标", `icons=${pwRow?.icons.length}`);
  }
  await snap("s20-inherited-fields");
  await closeEditDialog();

  report.section("S9 画布数据节点与子画布（步骤 17）");
  await openTemplatePanel();
  {
    const blank = await pickBlankPoint();
    await dragTemplateRow("登录凭证 2", "拖拽创建画布数据节点", blank);
  }
  await waitForCondition(async () => textNodes(await api.uiTree(), "新画布").length > 0, {
    timeout: 8000,
    interval: 250,
    label: "新画布出现",
  });
  await sleep(800);
  tree = await api.uiTree();
  report.check(textNodes(tree, "新画布").length === 1, "步骤 17 画布出现画布数据节点「新画布」");
  report.check(textNodes(tree, "登录凭证 2").length === 2, "步骤 17 画布数据节点副标题为「登录凭证 2」（与数据节点副标题共 2 处）");
  report.check(panelTextNodes(tree, "空节点").length === 0, "步骤 17 模板面板自动关闭");
  await snap("s21-drag-created-canvas-node");
  {
    const nodeText = await waitForCondition(async () => textNodes(await api.uiTree(), "新画布")[0] ?? null, {
      timeout: 6000,
      interval: 200,
      label: "「新画布」文本",
    });
    await ui.clickNode(nodeText, { count: 2 });
  }
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      return flattenNodes(t).some((n) => n.role === "link" && (n.text ?? "").trim() === "根画布") ? t : null;
    },
    { timeout: 10000, interval: 250, label: "子画布面包屑" },
  );
  await sleep(1200);
  tree = await api.uiTree();
  report.check(ui.hasText(tree, "新画布"), "步骤 17 双击「新画布」进入子画布（面包屑出现「新画布」）");
  report.check(textNodes(tree, "账号 A").length === 0, "步骤 17 子画布为空（不含根画布节点「账号 A」）");
  await snap("s22-sub-canvas");
  {
    const crumb = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        return flattenNodes(t).find((n) => n.role === "link" && (n.text ?? "").trim() === "根画布") ?? null;
      },
      { timeout: 6000, interval: 200, label: "面包屑链接「根画布」" },
    );
    await ui.clickNode(crumb);
  }
  await ui.waitForTextStable("账号 A", { timeout: 10000, settleMs: 800 });
  await sleep(800);
  report.check(true, "步骤 17 点击面包屑「根画布」返回根画布");
  await snap("s23-back-root");

  report.section("S10 导出模板（步骤 18-19）");
  await openTemplateManager();
  await clickTemplateDialogButton("导出模板");
  await waitForCondition(async () => dialogWithPathInput(await api.uiTree()) !== null, {
    timeout: 10000,
    interval: 250,
    label: "导出文件选择器",
  });
  await sleep(1000);
  tree = await api.uiTree();
  report.check(dialogWithPathInput(tree) !== null, "步骤 18 打开文件选择器（save 模式）");
  report.check(ui.hasText(tree, "导出模板"), "步骤 18 文件选择器标题「导出模板」");
  {
    const picker = dialogWithPathInput(tree);
    const nameInput = ui.findEditable([picker], "文件名");
    report.check(nameInput !== null, "步骤 18 存在文件名输入框");
    const defaultName = await readEditableValue(nameInput);
    report.check(defaultName === EXPORT_NAME, "步骤 18 默认文件名为 templates.sqlite（剪贴板验证）", `clip=${defaultName}`);
  }
  await snap("s24-export-picker");
  await fillPickerAndConfirm(OUTPUT_DIR, null);
  const exported = await ui.waitForTextStable("模板已导出", { timeout: 10000 }).catch(() => null);
  report.check(exported !== null, "步骤 19 导出成功提示「模板已导出」");
  await sleep(800);
  {
    const exists = fs.existsSync(EXPORT_SQLITE);
    report.check(exists, "步骤 19 导出文件存在", EXPORT_SQLITE);
    const head = exists ? fs.readFileSync(EXPORT_SQLITE).subarray(0, 16).toString("latin1") : "";
    report.check(head === "SQLite format 3\u0000", "步骤 19 导出文件头为 SQLite 格式（SQLite format 3）", JSON.stringify(head));
  }
  await snap("s25-exported");

  report.section("S11 删除模板（步骤 20-21）");
  await clickTemplateListItem("账号模板");
  tree = await api.uiTree();
  {
    const dialog = templateDialog(tree);
    const inTitle = dialog ? textNodes([dialog], "账号模板").filter((n) => n.bounds.left >= dialog.bounds.left + 300) : [];
    report.check(inTitle.length === 1, "步骤 20 选中「账号模板」（右侧标题显示）");
  }
  {
    const button = await waitForCondition(async () => ui.findAttr(await api.uiTree(), "title", "删除模板"), {
      timeout: 8000,
      interval: 200,
      label: "「删除模板」按钮",
    });
    await ui.clickNode(button);
  }
  await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "删除模板") !== null, {
    timeout: 8000,
    interval: 250,
    label: "删除模板确认框",
  });
  await sleep(600);
  tree = await api.uiTree();
  report.check(ui.hasText(tree, "删除模板"), "步骤 20 弹出确认框「删除模板」");
  report.check(
    ui.hasText(tree, '确定要删除模板"账号模板"吗？其字段结构将一并删除。'),
    "步骤 20 删除确认正文与预期一致",
  );
  await snap("s26-delete-confirm");
  await clickConfirmDialogButton("删除模板", "确认");
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const dialog = templateDialog(t);
      return dialog && textNodes([dialog], "账号模板").length === 0 ? t : null;
    },
    { timeout: 8000, interval: 250, label: "账号模板消失" },
  );
  await sleep(600);
  tree = await api.uiTree();
  report.check(textNodes([templateDialog(tree)], "账号模板").length === 0, "步骤 21 列表中「账号模板」消失");
  await snap("s27-deleted");

  report.section("S12 F5 导入确认取消");
  await clickTemplateDialogButton("导入模板");
  await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "导入模板") !== null, {
    timeout: 8000,
    interval: 250,
    label: "导入确认框",
  });
  await sleep(600);
  tree = await api.uiTree();
  report.check(ui.hasText(tree, "导入模板"), "F5 弹出确认框「导入模板」");
  report.check(
    ui.hasText(tree, "导入将替换当前所有模板和字典数据，确定继续吗？"),
    "F5 导入确认正文与预期一致",
  );
  await snap("s28-import-confirm");
  await clickConfirmDialogButton("导入模板", "取消");
  await sleep(900);
  tree = await api.uiTree();
  report.check(dialogWithPathInput(tree) === null, "F5 点「取消」后不打开文件选择器");
  report.check(templateDialog(tree) !== null, "F5 点「取消」后模板管理对话框保持打开");
  report.check(textNodes([templateDialog(tree)], "账号模板").length === 0, "F5 取消后模板数据不变（无「账号模板」）");
  await snap("s29-f5-import-cancel");

  report.section("S13 导入模板（步骤 22-24）");
  await clickTemplateDialogButton("导入模板");
  await waitForCondition(async () => dialogWithExactText(await api.uiTree(), "导入模板") !== null, {
    timeout: 8000,
    interval: 250,
    label: "导入确认框（二次）",
  });
  await sleep(500);
  await clickConfirmDialogButton("导入模板", "确认");
  await waitForCondition(async () => dialogWithPathInput(await api.uiTree()) !== null, {
    timeout: 10000,
    interval: 250,
    label: "导入文件选择器",
  });
  await sleep(1000);
  tree = await api.uiTree();
  report.check(dialogWithPathInput(tree) !== null, "步骤 23 打开文件选择器（open 模式）");
  {
    const picker = dialogWithPathInput(await api.uiTree());
    await ui.clickNode(ui.findEditable([picker], "路径"));
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText(OUTPUT_DIR), api.keyClick(["Enter"])]);
    await sleep(1200);
  }
  {
    await scrollPickerToFile(EXPORT_NAME);
    await sleep(400);
    const fileText = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        const picker = dialogWithPathInput(t);
        return picker ? ui.findByText([picker], EXPORT_NAME, { exact: true }) : null;
      },
      { timeout: 6000, interval: 250, label: "文件选择器列出 templates.sqlite" },
    );
    report.check(fileText !== null, "步骤 24 路径框导航到 output 并滚动列表后可看到 templates.sqlite");
    await snap("s30-import-picker");
    await ui.clickNode(fileText, { count: 2 });
  }
  const imported = await ui.waitForTextStable("模板已导入", { timeout: 10000 }).catch(() => null);
  report.check(imported !== null, "步骤 24 导入成功提示「模板已导入」");
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const dialog = templateDialog(t);
      return dialog && textNodes([dialog], "账号模板").length > 0 ? t : null;
    },
    { timeout: 8000, interval: 250, label: "「账号模板」重新出现" },
  );
  await sleep(800);
  tree = await api.uiTree();
  report.check(textNodes([templateDialog(tree)], "账号模板").length >= 1, "步骤 24 列表中重新出现「账号模板」（导入替换生效）");
  await snap("s31-imported");
  await clickTemplateDialogButton("取消");
  await ui.waitForDialogGone("模板管理", { timeout: 8000 });
  await sleep(600);
  await openTemplatePanel();
  await waitForCondition(async () => panelTextNodes(await api.uiTree(), "账号模板").length > 0, {
    timeout: 6000,
    interval: 250,
    label: "面板同步「账号模板」",
  });
  report.check(panelTextNodes(await api.uiTree(), "账号模板").length === 1, "步骤 24 模板面板中模板行同步更新（出现「账号模板」）");
  await snap("s32-panel-after-import");
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
