// case_005 节点字段与字典。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）
// → 编辑节点对话框字段区：添加字段 / 字段名内联编辑（Enter 提交 / Esc 取消且不提交）/ 值输入 / 类型切换为密码
// → 密码生成器（默认 20 位、F5 取消全部字符集后「使用」禁用、使用回填）
// → 字段名 Esc 取消与 Enter 提交的持久化对比 / 字段校验 F1 空名 / F2 重名（两行标错）/ 删除重复字段行 → 保存
// → 重开验证：口令值（20 位字母数字）复制到剪贴板、倒计时结束清空；用户名值 alice 复制验证
// → 字典管理：空态、添加「平台 / 微信 / 邮箱」、F3 空值保存拦截、删除确认（无/有子条目两种文案）
// → F4 未保存修改关闭确认（取消恢复编辑、确认放弃）→ 保存并重开验证持久化
// → 字段绑定字典（齿轮面板 TreeSelect）→ 候选选择「微信」→ 保存后剪贴板验证
// → 追加子条目「钉钉」并在候选中验证 → 邮箱字段 F6 格式校验与剪贴板验证 → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 断言优先使用 UI 树（文本、bounds、states），截图用于视觉留档；顶部剪贴板倒计时条的
//   出现/消失使用截图像素轮询断言，以顶部高饱和色带像素为识别依据（覆盖进度条由绿到红的
//   渐变全过程并适配对话框遮罩压暗），规避渲染延迟与颜色阶段的单帧采样抖动；
// - 字段行内按钮无 title/aria-label：按「字段名文本 bounds 与本行元素相对位置」定位
//   （齿轮/删除为名称行右侧按钮按 left 升序；复制为值行最右 BUTTON；生成器为值行内第 2 个 I role=button）；
// - 字典管理对话框的行内元素按 treeitem.bounds 相对坐标定位（值输入框 left+120，
//   行内按钮自右向左 right-40/-69/-99/-130）；子条目默认折叠，添加子条目后先点击行左端展开箭头（left+27）再输入；
// - 字段名内联编辑：Enter 提交；Esc 在 keydown 阶段取消编辑且不提交（应用已修复此前
//   「Esc 提交草稿」的缺陷），本脚本对 Esc 取消做通过性断言（展示恢复原名、对话框不关闭、保存重开后草稿未写库）；
// - 应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾；
// - 关键步骤截图存 output；流程中断时额外截取 fatal 截图并写入 report.json。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_005\case_005.js`

import { execSync } from "node:child_process";
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

/** 字段类型下拉的显示名集合（用于从对话框 combobox 中识别「类型下拉」而非「值编辑器」）。 */
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

/** 点击对话框外画布空白区的坐标（用于关闭字段齿轮扩展面板，窗口物理像素）。 */
const CANVAS_BLANK_POINT = { x: 700, y: 150 };

/** 字典树条目行内元素的相对偏移（相对 treeitem 左端/右端，窗口物理像素）。 */
const DICT_OFFSETS = { input: 120, toggle: 27, up: -130, down: -99, add: -69, del: -40 };

/** 剪贴板清空倒计时时长（默认设置 10 秒）与等待余量（毫秒） */
const CLIPBOARD_TIMEOUT_MS = 10000;
const CLIPBOARD_WAIT_MS = CLIPBOARD_TIMEOUT_MS + 1500;

const report = new Report("case_005 节点字段与字典");

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
 * 查找 UI 树中文本精确匹配的第 index 个 #text 节点。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 目标文本。
 * @param {number} [index=0] 出现序号（按 top 升序）。
 * @returns {object|null} 命中的文本节点；未命中时为 null。
 */
function findTextNode(tree, text, index = 0) {
  return textNodes(tree, text)[index] ?? null;
}

/**
 * 在 UI 树中查找 role=option 且文本精确匹配的候选项。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 候选项文本。
 * @returns {object|null} 命中的候选项；未命中时为 null。
 */
function findOption(tree, text) {
  return (
    flattenNodes(tree).find(
      (node) => node.role === "option" && (node.text ?? "").trim() === text,
    ) ?? null
  );
}

/**
 * 等待 role=option 且文本精确匹配的候选项出现。
 * @param {string} text 候选项文本。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeout=6000] 超时毫秒数。
 * @returns {Promise<object>} 命中的候选项。
 */
async function waitForOption(text, { timeout = 6000 } = {}) {
  return waitForCondition(
    async () => findOption(await api.uiTree(), text),
    { timeout, interval: 200, label: `option "${text}"` },
  );
}

/**
 * 解析编辑节点对话框中指定字段名文本对应的行元素集合。
 *
 * 行模型：字段名文本节点 + 名称行右侧按钮（left 升序为齿轮、删除）+ 类型下拉（combobox）+
 * 值编辑器（类型下拉右侧的可编辑控件或 combobox）+ 复制按钮（值行最右 BUTTON）+
 * 值行图标（I role=button，left 升序为显示/隐藏、密码生成器）。
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
    node.bounds.top < b.top + b.height + pad &&
    node.bounds.top + node.bounds.height > b.top - pad;
  const buttons = nodes
    .filter(
      (node) =>
        node.role === "button" &&
        overlapY(node, 20) &&
        node.bounds.left > b.left + b.width,
    )
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
  const rowOverlap = (node) =>
    node.bounds.top < c.top + c.height && node.bounds.top + node.bounds.height > c.top;
  const valueEditor =
    nodes.find(
      (node) =>
        node !== typeCombo &&
        (node.role === "textbox" || node.role === "combobox") &&
        rowOverlap(node) &&
        node.bounds.left >= c.left + c.width - 5,
    ) ?? null;
  const valueRight = valueEditor
    ? valueEditor.bounds.left + valueEditor.bounds.width - 5
    : c.left + c.width;
  const copyButton =
    nodes
      .filter(
        (node) =>
          node.tag === "BUTTON" && node.role === "button" && rowOverlap(node) && node.bounds.left >= valueRight,
      )
      .sort((a, c2) => c2.bounds.left - a.bounds.left)[0] ?? null;
  const icons = nodes
    .filter((node) => node.tag === "I" && node.role === "button" && rowOverlap(node))
    .sort((a, c2) => a.bounds.left - c2.bounds.left);
  return { name: nameText, buttons, typeCombo, valueEditor, copyButton, icons };
}

/**
 * 单击最后一个「未命名字段」占位文本进入名称编辑态。
 * @returns {Promise<void>} 无返回值。
 */
async function clickLastPlaceholder() {
  const list = textNodes(await api.uiTree(), "未命名字段");
  if (list.length === 0) {
    throw new Error("clickLastPlaceholder: no placeholder found");
  }
  await ui.clickNode(list[list.length - 1]);
}

/**
 * 点击「添加字段」按钮并等待新占位行出现。
 * @returns {Promise<object>} 新增字段行的「未命名字段」占位文本节点。
 */
async function addFieldPlaceholder() {
  const tree = await api.uiTree();
  await ui.clickNode(ui.findButton(tree, "添加字段", { exact: true }));
  return waitForCondition(
    async () => {
      const list = textNodes(await api.uiTree(), "未命名字段");
      return list.length > 0 ? list[list.length - 1] : null;
    },
    { timeout: 5000, interval: 150, label: "new field placeholder" },
  );
}

/**
 * 将字段名改为指定文本：单击名称 -> 输入 -> Enter 提交，并等待提交完成。
 * @param {object} nameText 字段名文本节点（#text）。
 * @param {string} newName 新字段名。
 * @returns {Promise<void>} 无返回值。
 */
async function renameField(nameText, newName) {
  const nameTop = nameText.bounds.top;
  await ui.clickNode(nameText);
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
  await api.postInput([api.typeText(newName), api.keyClick(["Enter"])]);
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const stillEditing = flattenNodes(tree).some(
        (node) =>
          node.role === "textbox" &&
          node.tag === "INPUT" &&
          Math.abs(node.bounds.top - nameTop) < 25 &&
          node.bounds.left < 600,
      );
      return !stillEditing && textNodes(tree, newName).length > 0;
    },
    { timeout: 5000, interval: 200, label: `field renamed to "${newName}"` },
  );
}

/**
 * 清空字段名并提交（用于删除前把重复字段改回未命名）。
 * @param {object} nameText 字段名文本节点（#text）。
 * @returns {Promise<void>} 无返回值。
 */
async function clearFieldName(nameText) {
  const nameTop = nameText.bounds.top;
  await ui.clickNode(nameText);
  await waitForCondition(
    async () =>
      flattenNodes(await api.uiTree()).find(
        (node) =>
          node.role === "textbox" &&
          node.tag === "INPUT" &&
          Math.abs(node.bounds.top - nameTop) < 25 &&
          node.bounds.left < 600,
      ) ?? null,
    { timeout: 4000, interval: 150, label: "name edit input (clear)" },
  );
  await api.postInput([
    api.keyClick(["Control", "a"]),
    api.keyClick(["Delete"]),
    api.keyClick(["Enter"]),
  ]);
  await waitForCondition(
    async () => textNodes(await api.uiTree(), "未命名字段").length > 0,
    { timeout: 5000, interval: 200, label: "field name cleared" },
  );
}

/**
 * 切换指定字段行的类型：点击类型下拉 -> 点击目标类型选项 -> 等待下拉显示新类型。
 * @param {object} nameText 字段名文本节点（#text）。
 * @param {string} typeName 目标类型显示名（如「密码」「邮箱地址」）。
 * @returns {Promise<void>} 无返回值。
 */
async function selectFieldType(nameText, typeName) {
  const tree = await api.uiTree();
  const row = fieldRow(tree, nameText);
  if (!row?.typeCombo) {
    throw new Error(`selectFieldType: type combo not found for "${row?.name?.text}"`);
  }
  await ui.clickNode(row.typeCombo);
  const option = await waitForOption(typeName, { timeout: 6000 });
  await ui.clickNode(option);
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const r = fieldRow(t, findTextNode(t, nameText.text ?? "", 0) ?? nameText);
      return (r?.typeCombo?.text ?? "").trim() === typeName;
    },
    { timeout: 5000, interval: 200, label: `type switched to "${typeName}"` },
  );
}

/**
 * 点击对话框内按钮（对话框内查找以避免与嵌套对话框的同名按钮混淆）。
 * @param {string} dialogTitle 对话框标题/内容片段。
 * @param {string} buttonText 按钮文本。
 * @param {object} [options] 可选参数。
 * @param {boolean} [options.exact=false] 是否精确匹配按钮子树文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickDialogButton(dialogTitle, buttonText, { exact = false } = {}) {
  const tree = await api.uiTree();
  const dialog = ui.findDialog(tree, dialogTitle);
  if (!dialog) {
    throw new Error(`clickDialogButton: dialog "${dialogTitle}" not found`);
  }
  const button = ui.findButton(tree, buttonText, { root: dialog, exact });
  if (!button) {
    throw new Error(`clickDialogButton: button "${buttonText}" not found in "${dialogTitle}"`);
  }
  await ui.clickNode(button);
}

/**
 * 打开「账号 A」节点的编辑对话框。
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
  await clickDialogButton("编辑节点", "取消");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(400);
}

/**
 * 通过画布悬浮菜单打开「字典管理」对话框。
 * @returns {Promise<void>} 无返回值。
 */
async function openDictionary() {
  const button = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "title", "字典管理"),
    { timeout: 8000, interval: 200, label: "「字典管理」按钮可见" },
  );
  await ui.clickNode(button);
  await ui.waitForDialog("字典管理", { timeout: 8000 });
  await sleep(600);
}

/**
 * 读取字典管理对话框中的树条目列表（按 top 排序）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} treeitem 节点数组。
 */
function dictItems(tree) {
  const dialog = ui.findDialog(tree, "字典管理");
  if (!dialog) {
    return [];
  }
  return ui
    .flatten([dialog])
    .map(({ node }) => node)
    .filter((node) => node.role === "treeitem")
    .sort((a, b) => a.bounds.top - b.bounds.top);
}

/**
 * 计算字典树条目行内元素坐标（基于 treeitem bounds 的相对偏移）。
 * @param {object} item treeitem 节点。
 * @param {"input"|"toggle"|"up"|"down"|"add"|"del"} kind 元素类型。
 * @returns {{x: number, y: number}} 目标坐标（窗口物理像素）。
 */
function itemPoint(item, kind) {
  const right = item.bounds.left + item.bounds.width;
  const cy = item.bounds.top + item.bounds.height / 2;
  if (kind === "input") {
    return { x: item.bounds.left + DICT_OFFSETS.input, y: cy };
  }
  if (kind === "toggle") {
    return { x: item.bounds.left + DICT_OFFSETS.toggle, y: cy };
  }
  return { x: right + DICT_OFFSETS[kind], y: cy };
}

/**
 * 点击字典树条目行内元素。
 * @param {object} item treeitem 节点。
 * @param {"input"|"toggle"|"up"|"down"|"add"|"del"} kind 元素类型。
 * @returns {Promise<void>} 无返回值。
 */
async function itemClick(item, kind) {
  const point = itemPoint(item, kind);
  await api.postInput([api.mouseMove(point.x, point.y), api.mouseClick("left", 1)]);
  await sleep(400);
}

/**
 * 在字典树条目行的值输入框输入文本。
 * @param {object} item treeitem 节点。
 * @param {string} text 要输入的文本。
 * @returns {Promise<void>} 无返回值。
 */
async function typeIntoItem(item, text) {
  await itemClick(item, "input");
  await api.postInput([api.typeText(text)]);
  await sleep(400);
}

/**
 * 等待字典树条目数量不少于指定值。
 * @param {number} count 目标条目数量。
 * @param {string} label 等待条件名。
 * @returns {Promise<object[]>} 满足条件的 treeitem 数组。
 */
async function waitForDictItems(count, label) {
  return waitForCondition(
    async () => {
      const list = dictItems(await api.uiTree());
      return list.length >= count ? list : null;
    },
    { timeout: 6000, interval: 200, label },
  );
}

/**
 * 确保字典管理对话框中的「平台」条目处于展开状态（子条目可见）。
 * @returns {Promise<object[]>} 展开后的 treeitem 数组（平台、微信、邮箱…）。
 */
async function expandPlatform() {
  let items = dictItems(await api.uiTree());
  if (items.length >= 3) {
    return items;
  }
  if (items.length === 0) {
    throw new Error("expandPlatform: dictionary is empty");
  }
  await itemClick(items[0], "toggle");
  items = await waitForCondition(
    async () => {
      const list = dictItems(await api.uiTree());
      return list.length >= 3 ? list : null;
    },
    { timeout: 5000, interval: 200, label: "platform expanded" },
  );
  return items;
}

/**
 * 点击字典管理对话框底部的「保存」按钮（精确匹配，避免误点其他含「保存」的按钮）。
 * @returns {Promise<void>} 无返回值。
 */
async function clickDictionarySave() {
  await clickDialogButton("字典管理", "保存", { exact: true });
}

/**
 * 读取系统剪贴板文本（仅用于验证脚本自身写入的测试值）。
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
 * 统计整窗截图顶部区域内的进度条像素数量。
 *
 * 进度条颜色在动画期间由绿渐变到红，且对话框遮罩会将其压暗，故以「高饱和色带像素」
 * 作为出现依据（覆盖整个渐变过程）；绿色系/红色系分量保留供无条基线断言使用。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @returns {{green: number, red: number, saturated: number}} 绿色系、红色系与高饱和色带像素数量。
 */
function topBarStats(png) {
  let green = 0;
  let red = 0;
  let saturated = 0;
  const yMax = Math.min(png.height, 90);
  for (let y = 36; y < yMax; y++) {
    for (let x = 0; x < png.width; x++) {
      const i = (png.width * y + x) << 2;
      const r = png.data[i];
      const g = png.data[i + 1];
      const b = png.data[i + 2];
      if (g > 110 && g > r + 25 && g > b + 15) {
        green += 1;
      }
      if (r > 170 && r > g + 60 && r > b + 60) {
        red += 1;
      }
      const max = Math.max(r, g, b);
      const min = Math.min(r, g, b);
      if (max > 80 && max - min >= 35) {
        saturated += 1;
      }
    }
  }
  return { green, red, saturated };
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
 * 截取当前界面并统计顶部进度条像素。
 * @returns {Promise<{green: number, red: number, saturated: number}>} 像素统计。
 */
async function captureTopBarStats() {
  return topBarStats(image.decodePng(await api.screenshot()));
}

/**
 * 轮询截图的顶部进度条像素统计，直到满足判定条件或超时。
 * 规避 /screenshot 相对 DOM 约一帧到数帧的渲染延迟造成的单帧像素采样抖动。
 * @param {(stats: {green: number, red: number, saturated: number}) => boolean} predicate 判定函数。
 * @param {object} [options] 可选参数。
 * @param {number} [options.timeoutMs=3000] 超时毫秒数。
 * @param {number} [options.intervalMs=150] 轮询间隔毫秒数。
 * @param {string} [options.label="top bar"] 轮询条件名（超时日志使用）。
 * @returns {Promise<{ok: boolean, stats: {green: number, red: number, saturated: number}}>} 是否满足条件与最后一次像素统计。
 */
async function waitForTopBar(predicate, { timeoutMs = 3000, intervalMs = 150, label = "top bar" } = {}) {
  const deadline = Date.now() + timeoutMs;
  let stats = await captureTopBarStats();
  while (!predicate(stats) && Date.now() < deadline) {
    await sleep(intervalMs);
    stats = await captureTopBarStats();
  }
  const ok = predicate(stats);
  if (!ok) {
    console.log(`  (waitForTopBar "${label}" timed out after ${timeoutMs}ms: ${JSON.stringify(stats)})`);
  }
  return { ok, stats };
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
  report.check(ui.hasText(unlockedTree, "备注 B"), "解锁后出现节点「备注 B」");
  await sleep(600);
  await snap("unlocked-root-canvas");

  report.section("S1 打开编辑对话框与添加字段（步骤 1）");
  await openAccountDialog();
  let tree = await api.uiTree();
  report.check(ui.findDialog(tree, "编辑节点") !== null, "步骤 1 双击卡片打开「编辑节点」对话框");
  report.check(ui.findEditable(tree, "标题") !== null, "步骤 1 对话框包含「标题」输入框");
  const preStats = await captureTopBarStats();
  report.check(preStats.green < 100 && preStats.red < 100, "剪贴板倒计时开始前顶部无进度条", JSON.stringify(preStats));
  const placeholder = await addFieldPlaceholder();
  tree = await api.uiTree();
  {
    const row = fieldRow(tree, placeholder);
    report.check((row?.typeCombo?.text ?? "").trim() === "单行文本", "步骤 1 新字段类型默认「单行文本」", row?.typeCombo?.text);
    report.check(row?.valueEditor != null, "步骤 1 新字段行出现值编辑器");
  }
  await snap("step1-field-added");

  report.section("S2 字段名内联编辑（步骤 2）");
  await clickLastPlaceholder();
  const editInput = await waitForCondition(
    async () =>
      flattenNodes(await api.uiTree()).find(
        (node) =>
          node.role === "textbox" &&
          node.tag === "INPUT" &&
          Math.abs(node.bounds.top - placeholder.bounds.top) < 25 &&
          node.bounds.left < 600,
      ) ?? null,
    { timeout: 4000, interval: 150, label: "name edit input" },
  );
  report.check(editInput !== null, "步骤 2 单击字段名进入内联编辑态（出现输入框）");
  await api.postInput([api.typeText("用户名"), api.keyClick(["Enter"])]);
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const editing = flattenNodes(t).some(
        (node) =>
          node.role === "textbox" &&
          node.tag === "INPUT" &&
          Math.abs(node.bounds.top - placeholder.bounds.top) < 25 &&
          node.bounds.left < 600,
      );
      return !editing && textNodes(t, "用户名").length === 1;
    },
    { timeout: 5000, interval: 200, label: "name committed" },
  );
  report.check(true, "步骤 2 Enter 提交后字段名显示「用户名」且内联输入框消失");
  await snap("step2-name-committed");

  report.section("S3 值输入（步骤 3）");
  tree = await api.uiTree();
  let row = fieldRow(tree, findTextNode(tree, "用户名"));
  await ui.clickNode(row.valueEditor);
  await api.postInput([api.typeText("alice")]);
  await sleep(400);
  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "用户名"));
  report.check(
    row.copyButton?.states?.disabled === false,
    "步骤 3 输入值 alice 后该行复制按钮可用（值非空）",
  );
  await snap("step3-value-alice");

  report.section("S4 第二字段与密码类型（步骤 4-5）");
  const ph2 = await addFieldPlaceholder();
  await renameField(ph2, "口令");
  tree = await api.uiTree();
  report.check(textNodes(tree, "口令").length === 1, "步骤 4 出现第二个字段行「口令」");
  await selectFieldType(findTextNode(tree, "口令"), "密码");
  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "口令"));
  report.check((row.typeCombo?.text ?? "").trim() === "密码", "步骤 5 类型显示「密码」", row.typeCombo?.text);
  report.check(row.icons.length === 2, "步骤 5 密码编辑器出现显示/隐藏与生成器图标", JSON.stringify(row.icons.map((n) => n.bounds)));
  await snap("step5-password-type");

  report.section("S5 密码生成器（步骤 6-7、F5）");
  await ui.clickNode(row.icons[row.icons.length - 1]);
  await ui.waitForDialog("密码生成器", { timeout: 8000 });
  await sleep(600);
  tree = await api.uiTree();
  report.check(ui.findDialog(tree, "密码生成器") !== null, "步骤 6 打开「密码生成器」对话框");
  report.check(ui.hasText(tree, "长度: 20"), "步骤 6 长度滑块默认 20");
  {
    const checkboxes = ui
      .findByRole(tree, "checkbox")
      .sort((a, b) => a.bounds.top - b.bounds.top);
    const byName = (name) => checkboxes.find((c) => c.name === name);
    report.check(checkboxes.length === 4, "步骤 6 包含四个字符集勾选项");
    report.check(
      byName("大写字母 (A-Z)")?.states?.checked === true &&
        byName("小写字母 (a-z)")?.states?.checked === true &&
        byName("数字 (0-9)")?.states?.checked === true &&
        byName("符号（特殊字符）")?.states?.checked !== true,
      "步骤 6 默认勾选大写/小写/数字，符号未勾选",
      JSON.stringify(checkboxes.map((c) => ({ n: c.name, c: c.states?.checked }))),
    );
  }
  await snap("step6-password-generator");

  for (const label of ["大写字母 (A-Z)", "小写字母 (a-z)", "数字 (0-9)"]) {
    const checkbox = await waitForCondition(
      async () =>
        flattenNodes(await api.uiTree()).find(
          (node) => node.role === "checkbox" && node.name === label,
        ) ?? null,
      { timeout: 4000, interval: 150, label: `checkbox "${label}"` },
    );
    await ui.clickNode(checkbox);
    await sleep(400);
  }
  const useButton = await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const dialog = ui.findDialog(t, "密码生成器");
      return dialog ? ui.findButton(t, "使用", { root: dialog, exact: true }) : null;
    },
    { timeout: 4000, interval: 200, label: "「使用」按钮" },
  );
  report.check(useButton.states?.disabled === true, "F5 取消全部字符集后「使用」按钮禁用");
  await snap("f5-no-charset-selected");

  const uppercaseBox = await waitForCondition(
    async () =>
      flattenNodes(await api.uiTree()).find(
        (node) => node.role === "checkbox" && node.name === "大写字母 (A-Z)",
      ) ?? null,
    { timeout: 4000, interval: 150, label: "uppercase checkbox" },
  );
  await ui.clickNode(uppercaseBox);
  await sleep(500);
  const useButton2 = await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const dialog = ui.findDialog(t, "密码生成器");
      return dialog ? ui.findButton(t, "使用", { root: dialog, exact: true }) : null;
    },
    { timeout: 4000, interval: 200, label: "「使用」按钮恢复" },
  );
  report.check(useButton2.states?.disabled === false, "F5 重新勾选字符集后「使用」按钮恢复可用");
  await ui.clickNode(useButton2);
  await ui.waitForDialogGone("密码生成器", { timeout: 6000 });
  report.check(true, "步骤 7 点击「使用」后密码生成器关闭并回填值");
  await snap("step7-password-filled");

  report.section("S6 字段名内联编辑取消与校验 F1/F2（步骤 8-10）");
  const ph3 = await addFieldPlaceholder();
  // Esc 取消：输入与原名不同的草稿后按 Esc，草稿不生效、编辑态退出、对话框保持打开。
  const escTop = ph3.bounds.top;
  await ui.clickNode(ph3);
  const escInput = await waitForCondition(
    async () =>
      flattenNodes(await api.uiTree()).find(
        (node) =>
          node.role === "textbox" &&
          node.tag === "INPUT" &&
          Math.abs(node.bounds.top - escTop) < 25 &&
          node.bounds.left < 600,
      ) ?? null,
    { timeout: 4000, interval: 150, label: "name edit input (esc)" },
  );
  report.check(escInput !== null, "S6 单击占位文本进入字段名编辑态");
  await api.postInput([api.typeText("临时草稿")]);
  await sleep(300);
  await api.postInput([api.keyClick(["Escape"])]);
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const editing = flattenNodes(t).some(
        (node) =>
          node.role === "textbox" &&
          node.tag === "INPUT" &&
          Math.abs(node.bounds.top - escTop) < 25 &&
          node.bounds.left < 600,
      );
      return !editing && textNodes(t, "未命名字段").length === 1;
    },
    { timeout: 5000, interval: 200, label: "esc cancelled" },
  );
  tree = await api.uiTree();
  report.check(
    textNodes(tree, "临时草稿").length === 0,
    "Esc 取消后草稿未写回（无「临时草稿」文本）",
  );
  report.check(
    textNodes(tree, "未命名字段").length === 1,
    "Esc 取消后字段名展示恢复为「未命名字段」",
  );
  report.check(ui.findDialog(tree, "编辑节点") !== null, "Esc 取消后编辑节点对话框保持打开");
  await snap("s6-esc-cancelled");

  // Enter 提交：再次编辑后按 Enter 正常提交新名。
  await renameField(findTextNode(await api.uiTree(), "未命名字段"), "暂存字段");
  tree = await api.uiTree();
  report.check(textNodes(tree, "暂存字段").length === 1, "Enter 提交后字段名显示「暂存字段」");
  await snap("s6-enter-committed");

  // 保存并重开：Esc 草稿未写库、Enter 修改已持久化。
  await clickDialogButton("编辑节点", "确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(700);
  await openAccountDialog();
  await waitForCondition(
    async () => textNodes(await api.uiTree(), "用户名").length > 0,
    { timeout: 5000, interval: 200, label: "dialog fields loaded" },
  );
  tree = await api.uiTree();
  report.check(textNodes(tree, "暂存字段").length === 1, "重开后 Enter 提交的「暂存字段」已持久化");
  report.check(textNodes(tree, "临时草稿").length === 0, "重开后 Esc 取消的草稿未写库");
  await snap("s6-enter-persisted");

  // 删除临时字段行恢复现场，继续进行 F1/F2 校验。
  const stashRow = fieldRow(tree, findTextNode(tree, "暂存字段"));
  await ui.clickNode(stashRow.buttons[1]);
  await waitForCondition(
    async () => textNodes(await api.uiTree(), "暂存字段").length === 0,
    { timeout: 5000, interval: 200, label: "stash row removed" },
  );
  report.check(true, "删除临时字段行恢复现场（剩余「用户名」「口令」）");

  await addFieldPlaceholder();
  await clickDialogButton("编辑节点", "确认");
  const f1 = await ui.waitForTextStable("字段名不能为空", { timeout: 6000 }).catch(() => null);
  report.check(f1 !== null, "F1 空字段名提交提示「字段名不能为空」");
  tree = await api.uiTree();
  report.check(ui.findDialog(tree, "编辑节点") !== null, "F1 对话框保持打开，未提交");
  report.check(textNodes(tree, "未命名字段").length === 1, "F1 未命名字段行仍然存在");
  await snap("f1-empty-name");

  const dupPh = findTextNode(await api.uiTree(), "未命名字段");
  await renameField(dupPh, "用户名");
  tree = await api.uiTree();
  report.check(textNodes(tree, "用户名").length === 2, "步骤 9 两个字段行同名「用户名」");
  await clickDialogButton("编辑节点", "确认");
  const f2 = await ui.waitForTextStable("字段名重复", { timeout: 6000 }).catch(() => null);
  report.check(f2 !== null, "F2 重名提交提示「字段名重复」");
  tree = await api.uiTree();
  report.check(
    textNodes(tree, "字段名重复").length >= 1,
    "F2 出现「字段名重复」错误文本",
    `count=${textNodes(tree, "字段名重复").length}`,
  );
  {
    // 第二个同名字段行的错误文本初始位于字段滚动区视口之外（被 UI 树裁剪过滤），
    // 先用截图像素断言两个同名字段行的字段名均已标红。
    const f2Png = image.decodePng(await api.screenshot());
    const dupNameNodes = textNodes(tree, "用户名").slice(0, 2);
    const dupRedCounts = dupNameNodes.map((node) => redPixelsIn(f2Png, node.bounds));
    report.check(
      dupNameNodes.length === 2 && dupRedCounts.every((count) => count > 20),
      "F2 两个同名字段行的字段名均标红（像素断言）",
      JSON.stringify(dupRedCounts),
    );
  }
  report.check(ui.findDialog(tree, "编辑节点") !== null, "F2 对话框保持打开，未提交");
  await snap("f2-duplicated");

  // 滚动字段区使第二个错误文本进入视口，断言两个同名字段行均渲染了「字段名重复」。
  await api.postInput([api.mouseMove(700, 900), api.mouseScroll("down", 5)]);
  await sleep(1000);
  const bothErrors = await waitForCondition(
    async () => {
      const t = await api.uiTree();
      return textNodes(t, "字段名重复").length === 2 ? t : null;
    },
    { timeout: 4000, interval: 200, label: "两条重名错误文本同时可见" },
  );
  report.check(bothErrors !== null, "F2 两个同名字段行均显示「字段名重复」错误文本（滚动后同时可见）");
  await snap("f2-scrolled");

  // 在滚动位置直接操作第二个同名字段行（不回滚，避免滚动中间态导致点击偏移）。
  const dupNames = textNodes(bothErrors, "用户名");
  const dupName = dupNames[dupNames.length - 1];
  await clearFieldName(dupName);
  tree = await api.uiTree();
  report.check(textNodes(tree, "未命名字段").length === 1, "步骤 10 重复字段名已改回未命名");
  const emptyRow = fieldRow(tree, findTextNode(tree, "未命名字段"));
  await ui.clickNode(emptyRow.buttons[1]);
  await waitForCondition(
    async () => textNodes(await api.uiTree(), "未命名字段").length === 0,
    { timeout: 5000, interval: 200, label: "row removed" },
  );
  tree = await api.uiTree();
  report.check(
    textNodes(tree, "用户名").length === 1 && textNodes(tree, "口令").length === 1,
    "步骤 10 删除重复字段行后仅剩「用户名」「口令」两行",
  );
  await snap("step10-row-removed");
  await clickDialogButton("编辑节点", "确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  report.check(true, "步骤 10 点击「确认」后对话框关闭（字段已保存）");

  report.section("S7 重开验证与口令值复制（步骤 10、12b）");
  await sleep(700);
  await openAccountDialog();
  tree = await api.uiTree();
  report.check(
    textNodes(tree, "用户名").length === 1 && textNodes(tree, "口令").length === 1,
    "步骤 10 重开对话框后「用户名」「口令」字段均存在（字段已持久化）",
  );
  const pwRow = fieldRow(tree, findTextNode(tree, "口令"));
  report.check(pwRow.icons.length === 2, "步骤 10 重开对话框后「口令」行仍为密码编辑器");
  await ui.clickNode(pwRow.copyButton);
  const pwSnack = await ui
    .waitForTextStable("字段值已复制到剪贴板", { timeout: 6000 })
    .catch(() => null);
  report.check(pwSnack !== null, "步骤 10 口令值复制后出现「字段值已复制到剪贴板」");
  await sleep(400);
  const pwAppeared = await waitForTopBar((stats) => stats.saturated > 500, {
    timeoutMs: 3000,
    label: "口令复制后顶部进度条出现",
  });
  report.check(
    pwAppeared.ok,
    "复制口令值后顶部出现倒计时进度条（顶部色带像素断言）",
    JSON.stringify(pwAppeared.stats),
  );
  await snap("step10-password-copied");
  const clipPw = readClipboard();
  report.check(
    /^[A-Za-z0-9]{20}$/.test(clipPw ?? ""),
    "口令字段值为 20 位字母数字（生成器回填值已持久化）",
    `length=${clipPw?.length ?? "null"}`,
  );
  await sleep(CLIPBOARD_WAIT_MS);
  report.check(readClipboard() === "", "步骤 12 倒计时结束后系统剪贴板被清空（clipboard_clear 已执行）");
  const pwGone = await waitForTopBar((stats) => stats.saturated < 100, {
    timeoutMs: 3000,
    label: "口令倒计时结束后顶部进度条消失",
  });
  report.check(
    pwGone.ok,
    "步骤 12 倒计时结束后顶部进度条消失",
    JSON.stringify(pwGone.stats),
  );
  await snap("step12-password-cleared");

  report.section("S8 用户名值复制与剪贴板倒计时（步骤 11-12）");
  tree = await api.uiTree();
  const userRow = fieldRow(tree, findTextNode(tree, "用户名"));
  await ui.clickNode(userRow.copyButton);
  const userSnack = await ui
    .waitForTextStable("字段值已复制到剪贴板", { timeout: 6000 })
    .catch(() => null);
  report.check(userSnack !== null, "步骤 11 出现 snackbar「字段值已复制到剪贴板」");
  await sleep(400);
  const userAppeared = await waitForTopBar((stats) => stats.saturated > 500, {
    timeoutMs: 3000,
    label: "用户名复制后顶部进度条出现",
  });
  report.check(
    userAppeared.ok,
    "步骤 11 App 顶部出现全宽倒计时进度条（像素断言）",
    JSON.stringify(userAppeared.stats),
  );
  await snap("step11-username-copied");
  const clipUser = readClipboard();
  report.check(clipUser === "alice", "步骤 11 系统剪贴板内容为 alice", `length=${clipUser?.length ?? "null"}`);
  await sleep(CLIPBOARD_WAIT_MS);
  report.check(readClipboard() === "", "步骤 12 倒计时结束后剪贴板再次清空");
  await snap("step12-username-cleared");
  await closeEditDialog();

  report.section("S9 字典管理：空态与添加条目（步骤 13-15）");
  await sleep(400);
  await openDictionary();
  tree = await api.uiTree();
  report.check(ui.hasText(tree, "暂无字典条目，点击下方按钮添加"), "步骤 13 空字典提示「暂无字典条目，点击下方按钮添加」");
  {
    const saveButton = ui.findButton(tree, "保存", {
      root: ui.findDialog(tree, "字典管理"),
      exact: true,
    });
    report.check(saveButton?.states?.disabled === true, "步骤 13「保存」按钮在无修改时禁用");
  }
  await snap("step13-dictionary-empty");

  await ui.clickNode(ui.findButton(await api.uiTree(), "添加根条目", { exact: true }));
  let items = await waitForDictItems(1, "root item added");
  await typeIntoItem(items[0], "平台");
  items = dictItems(await api.uiTree());
  await itemClick(items[0], "add");
  let expanded = await waitForCondition(
    async () => {
      const list = dictItems(await api.uiTree());
      return list.length >= 2 ? list : null;
    },
    { timeout: 4000, interval: 200, label: "child added" },
  ).catch(() => null);
  if (!expanded) {
    await itemClick(items[0], "toggle");
    expanded = await waitForDictItems(2, "child visible");
  }
  await typeIntoItem(expanded[1], "微信");
  items = dictItems(await api.uiTree());
  await itemClick(items[0], "add");
  const three = await waitForDictItems(3, "second child visible");
  await typeIntoItem(three[2], "邮箱");
  tree = await api.uiTree();
  items = dictItems(tree);
  report.check(items.length === 3, "步骤 15「平台」下出现「微信」「邮箱」两个子条目", `items=${items.length}`);
  await snap("step15-dictionary-tree");

  report.section("S10 字典空值校验 F3（步骤 16）");
  await ui.clickNode(ui.findButton(await api.uiTree(), "添加根条目", { exact: true }));
  items = await waitForDictItems(4, "empty root added");
  report.check(items.length === 4, "步骤 16 已添加空值根条目");
  await clickDictionarySave();
  const f3 = await ui
    .waitForTextStable("存在值为空的条目，请填写或删除后再保存", { timeout: 6000 })
    .catch(() => null);
  report.check(f3 !== null, "F3 空值条目保存被拦截并提示「存在值为空的条目，请填写或删除后再保存」");
  report.check(ui.findDialog(await api.uiTree(), "字典管理") !== null, "F3 字典未保存，对话框保持打开");
  await snap("f3-empty-value-save");

  report.section("S11 删除空条目确认（步骤 17）");
  items = dictItems(await api.uiTree());
  await itemClick(items[items.length - 1], "del");
  await ui.waitForDialog("删除条目", { timeout: 6000 });
  const emptyDeleteText = '确定要删除条目""吗？引用它的字段将解除字典绑定。';
  const emptyDeleteShown = await ui
    .waitForTextStable(emptyDeleteText, { timeout: 5000 })
    .catch(() => null);
  report.check(emptyDeleteShown !== null, "步骤 17 无子条目删除确认文案与预期一致", emptyDeleteText);
  await snap("step17-delete-empty-confirm");
  await clickDialogButton("删除条目", "取消");
  await waitForCondition(
    async () => ui.findDialog(await api.uiTree(), "字典管理") !== null,
    { timeout: 5000, interval: 200, label: "dictionary dialog visible again" },
  );
  report.check(dictItems(await api.uiTree()).length === 4, "步骤 17 点「取消」后空条目保留");
  items = dictItems(await api.uiTree());
  await itemClick(items[items.length - 1], "del");
  await ui.waitForDialog("删除条目", { timeout: 6000 });
  await sleep(400);
  await clickDialogButton("删除条目", "确认");
  await waitForCondition(
    async () => dictItems(await api.uiTree()).length === 3,
    { timeout: 5000, interval: 200, label: "empty item deleted" },
  );
  report.check(true, "步骤 17 确认后空条目消失（条目数回到 3）");
  await snap("step17-empty-removed");

  report.section("S12 未保存修改确认 F4（步骤 18）");
  items = dictItems(await api.uiTree());
  await itemClick(items[0], "del");
  await ui.waitForDialog("删除条目", { timeout: 6000 });
  const platformDeleteText = '确定要删除条目"平台"吗？其全部子条目将一并删除，引用它们的字段将解除字典绑定。';
  const platformDeleteShown = await ui
    .waitForTextStable(platformDeleteText, { timeout: 5000 })
    .catch(() => null);
  report.check(platformDeleteShown !== null, "步骤 18 含子条目删除确认文案与预期一致", platformDeleteText);
  await snap("step18-delete-platform-confirm");
  await clickDialogButton("删除条目", "取消");
  await waitForCondition(
    async () => ui.findDialog(await api.uiTree(), "字典管理") !== null,
    { timeout: 5000, interval: 200, label: "dictionary dialog visible again" },
  );
  report.check(dictItems(await api.uiTree()).length === 3, "步骤 18 点「取消」后「平台」及其子条目保留");
  await clickDialogButton("字典管理", "取消");
  const unsavedText = "当前编辑有未保存的修改，确定要放弃吗？";
  const unsavedShown = await ui
    .waitForTextStable(unsavedText, { timeout: 6000 })
    .catch(() => null);
  report.check(unsavedShown !== null, "F4 关闭时弹出「未保存的修改」确认框", unsavedText);
  await snap("f4-unsaved-changes");
  await clickDialogButton("未保存的修改", "取消");
  await waitForCondition(
    async () => ui.findDialog(await api.uiTree(), "字典管理") !== null,
    { timeout: 5000, interval: 200, label: "dictionary dialog restored" },
  );
  report.check(dictItems(await api.uiTree()).length === 3, "F4 点「取消」后恢复编辑（条目数 3）");

  report.section("S13 保存字典（步骤 19）");
  await clickDictionarySave();
  const savedSnack = await ui
    .waitForTextStable("字典已保存", { timeout: 8000 })
    .catch(() => null);
  report.check(savedSnack !== null, "步骤 19 保存后提示「字典已保存」");
  await ui.waitForDialogGone("字典管理", { timeout: 8000 });
  report.check(true, "步骤 19 字典管理对话框关闭");
  await snap("step19-dictionary-saved");

  report.section("S14 字典持久化验证与 F4 放弃路径");
  await sleep(600);
  await openDictionary();
  let persisted = await expandPlatform();
  report.check(persisted.length === 3, "重开字典后条目树含「平台/微信/邮箱」3 条（已持久化）", `items=${persisted.length}`);
  await itemClick(persisted[1], "del");
  await ui.waitForDialog("删除条目", { timeout: 6000 });
  const wechatDeleteText = '确定要删除条目"微信"吗？引用它的字段将解除字典绑定。';
  const wechatDeleteShown = await ui
    .waitForTextStable(wechatDeleteText, { timeout: 5000 })
    .catch(() => null);
  report.check(wechatDeleteShown !== null, "重开后「微信」条目删除确认文案正确（含条目值）", wechatDeleteText);
  await clickDialogButton("删除条目", "取消");
  await waitForCondition(
    async () => ui.findDialog(await api.uiTree(), "字典管理") !== null,
    { timeout: 5000, interval: 200, label: "dictionary dialog visible again" },
  );
  await snap("step19b-reopen-verified");
  await clickDialogButton("字典管理", "取消");
  await ui.waitForDialogGone("字典管理", { timeout: 8000 });

  await sleep(500);
  await openDictionary();
  await ui.clickNode(ui.findButton(await api.uiTree(), "添加根条目", { exact: true }));
  await waitForDictItems(1, "temp root added");
  await clickDialogButton("字典管理", "取消");
  await ui.waitForDialog("未保存的修改", { timeout: 6000 });
  await clickDialogButton("未保存的修改", "确认");
  await ui.waitForDialogGone("字典管理", { timeout: 8000 });
  report.check(true, "F4b 未保存确认框点「确认」后放弃修改并关闭对话框");
  await sleep(600);
  await openDictionary();
  persisted = await expandPlatform();
  report.check(persisted.length === 3, "F4b 放弃修改后重开条目数仍为 3（未保存数据未写库）", `items=${persisted.length}`);
  await clickDialogButton("字典管理", "取消");
  await ui.waitForDialogGone("字典管理", { timeout: 8000 });

  report.section("S15 字段绑定字典与候选（步骤 20-21）");
  await sleep(500);
  await openAccountDialog();
  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "用户名"));
  report.check(row.buttons.length >= 2, "步骤 20「用户名」行存在齿轮与删除按钮");
  report.check(row.buttons[0].states?.disabled === false, "步骤 20 齿轮按钮可用（字典非空）");
  await ui.clickNode(row.buttons[0]);
  const dictInput = await waitForCondition(
    async () => ui.findEditable(await api.uiTree(), "字典"),
    { timeout: 6000, interval: 200, label: "字典 TreeSelect 出现" },
  );
  report.check(dictInput !== null, "步骤 20 展开字典选择面板，出现 label「字典」的树形选择控件");
  await snap("step20-gear-panel");
  await ui.clickNode(dictInput);
  const platformItem = await waitForCondition(
    async () =>
      flattenNodes(await api.uiTree()).find(
        (node) => node.role === "treeitem" && ui.subtreeText(node).includes("平台"),
      ) ?? null,
    { timeout: 6000, interval: 200, label: "TreeSelect 树节点「平台」" },
  );
  report.check(platformItem !== null, "步骤 21 树形选择下拉出现「平台」节点");
  await ui.clickNode(platformItem);
  const boundTree = await waitForCondition(
    async () => {
      const t = await api.uiTree();
      return flattenNodes(t).some((node) => node.name === "清除 字典") ? t : null;
    },
    { timeout: 6000, interval: 200, label: "字典已选中（清除按钮出现）" },
  );
  report.check(boundTree !== null, "步骤 21 选择「平台」后控件出现清除按钮（已绑定）");
  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "用户名"));
  report.check(row.valueEditor?.role === "combobox", "步骤 21 值编辑器变为带候选的下拉输入框", row.valueEditor?.role);
  await snap("step21-bound-platform");

  await api.postInput([api.mouseMove(CANVAS_BLANK_POINT.x, CANVAS_BLANK_POINT.y), api.mouseClick("left", 1)]);
  await waitForCondition(
    async () => !flattenNodes(await api.uiTree()).some((node) => node.name === "清除 字典"),
    { timeout: 5000, interval: 200, label: "齿轮面板关闭" },
  );
  report.check(true, "单击画布空白后齿轮扩展面板关闭");
  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "用户名"));
  await ui.clickNode(row.valueEditor);
  const wechatOption = await waitForOption("微信");
  report.check(wechatOption !== null, "步骤 21 值编辑器候选出现「微信」");
  report.check(findOption(await api.uiTree(), "邮箱") !== null, "步骤 21 候选同时包含「邮箱」");
  await snap("step21b-candidates");
  await ui.clickNode(wechatOption);
  await sleep(500);
  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "用户名"));
  report.check((row.valueEditor?.text ?? "").trim() === "微信", "步骤 22 选择候选「微信」后值编辑器显示「微信」");
  await snap("step22-wechat-selected");

  report.section("S16 保存绑定值并验证（步骤 22）");
  await clickDialogButton("编辑节点", "确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  report.check(true, "步骤 22 确认后对话框关闭（字典绑定与值已保存）");
  await snap("step22-saved");
  await sleep(700);
  await openAccountDialog();
  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "用户名"));
  report.check(
    (row.valueEditor?.text ?? "").trim() === "微信",
    "步骤 22 重开对话框后「用户名」值为「微信」（已持久化）",
    row.valueEditor?.text,
  );
  await snap("step22-reopened");
  await ui.clickNode(row.copyButton);
  await ui.waitForTextStable("字段值已复制到剪贴板", { timeout: 6000 });
  const clipWechat = readClipboard();
  report.check(clipWechat === "微信", "步骤 22 剪贴板验证值为「微信」", `length=${clipWechat?.length ?? "null"}`);
  await sleep(CLIPBOARD_WAIT_MS);
  report.check(readClipboard() === "", "步骤 22 倒计时结束后剪贴板清空");
  await closeEditDialog();

  report.section("S17 追加子条目「钉钉」（步骤 23）");
  await sleep(400);
  await openDictionary();
  let items4 = await expandPlatform();
  await itemClick(items4[0], "add");
  items4 = await waitForDictItems(4, "dingtalk row added");
  await typeIntoItem(items4[3], "钉钉");
  await clickDictionarySave();
  const savedSnack2 = await ui
    .waitForTextStable("字典已保存", { timeout: 8000 })
    .catch(() => null);
  report.check(savedSnack2 !== null, "步骤 23 追加「钉钉」后保存成功并提示「字典已保存」");
  await ui.waitForDialogGone("字典管理", { timeout: 8000 });
  await snap("step23-dingtalk-saved");

  await sleep(600);
  await openAccountDialog();
  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "用户名"));
  await ui.clickNode(row.valueEditor);
  const dingOption = await waitForOption("钉钉", { timeout: 6000 }).catch(() => null);
  report.check(dingOption !== null, "步骤 23 重开值候选包含新子条目「钉钉」");
  await snap("step23-candidates-dingtalk");
  await api.postInput([api.keyClick(["Escape"])]);
  await sleep(400);
  await closeEditDialog();

  report.section("S18 邮箱字段与格式校验 F6（步骤 24-25）");
  await sleep(400);
  await openAccountDialog();
  const ph4 = await addFieldPlaceholder();
  await renameField(ph4, "联系邮箱");
  await selectFieldType(findTextNode(await api.uiTree(), "联系邮箱"), "邮箱地址");
  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "联系邮箱"));
  report.check((row.typeCombo?.text ?? "").trim() === "邮箱地址", "步骤 24 类型切换为「邮箱地址」", row.typeCombo?.text);
  await ui.clickNode(row.valueEditor);
  await api.postInput([api.typeText("not-an-email")]);
  await sleep(400);
  await clickDialogButton("编辑节点", "确认");
  const f6 = await ui.waitForTextStable("邮箱地址格式不正确", { timeout: 6000 }).catch(() => null);
  report.check(f6 !== null, "F6 非法邮箱值提交提示「邮箱地址格式不正确」");
  report.check(ui.findDialog(await api.uiTree(), "编辑节点") !== null, "F6 对话框保持打开，未提交");
  await snap("f6-invalid-email");

  tree = await api.uiTree();
  row = fieldRow(tree, findTextNode(tree, "联系邮箱"));
  await ui.clickNode(row.valueEditor);
  await api.postInput([api.keyClick(["Control", "a"]), api.typeText("alice@example.com")]);
  await sleep(400);
  await clickDialogButton("编辑节点", "确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  report.check(true, "步骤 25 改为合法邮箱后保存成功，对话框关闭");
  await sleep(700);
  await openAccountDialog();
  tree = await api.uiTree();
  report.check(textNodes(tree, "联系邮箱").length === 1, "步骤 25 重开对话框「联系邮箱」字段存在");
  row = fieldRow(tree, findTextNode(tree, "联系邮箱"));
  await ui.clickNode(row.copyButton);
  await ui.waitForTextStable("字段值已复制到剪贴板", { timeout: 6000 });
  const clipEmail = readClipboard();
  report.check(clipEmail === "alice@example.com", "步骤 25 剪贴板验证邮箱值为 alice@example.com", `length=${clipEmail?.length ?? "null"}`);
  await snap("step25-email-copied");
  await sleep(CLIPBOARD_WAIT_MS);
  report.check(readClipboard() === "", "步骤 25 倒计时结束后剪贴板清空");
  await closeEditDialog();
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
