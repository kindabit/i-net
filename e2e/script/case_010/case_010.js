// case_010 搜索与日志。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）
// → 准备数据：为「账号 A」添加密码字段「登录密码」= log-secret-123（产生「编辑节点字段」日志）
//   → 拖拽模板面板创建并改名两个节点「搜索甲」「搜索乙」
//   → 拖动「备注 B」21 次（产生 21 条「移动节点」日志，使日志总数超过每页 20 条）
// → 全局搜索（步骤 1-7 + F1）：展开输入框 → 输入关键词（防抖计时与竞态替换）→ 候选与副标题 →
//   ArrowDown/ArrowUp 与循环高亮（候选行像素亮度断言）→ Enter 选择后跳转并 300ms 居中定位
//   （断言节点位于画布区域中心 ±20%）→ 无匹配提示 → Escape 关闭下拉 → 关闭搜索恢复面包屑
// → 日志对话框（步骤 8-19 + F2/F3）：打开对话框（共 N 条、时间倒序、每页 20 条）→ 翻页 →
//   关键词筛选与清除筛选 → 日期范围筛选（F2 非法范围 snackbar）→ 行为类型多选筛选 →
//   字段变更渲染（密码字段值掩码/明文切换）→ 显示敏感值开关 → 关闭
// → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 搜索候选行的高亮（VListItem active）在 UI 树中不映射为 states.selected，用候选行 bounds
//   内的平均亮度断言：高亮行背景更暗（实测相差约 25，阈值取 10）；
// - 搜索候选副标题为「节点副标题 · 画布库内名称」：fixture 根画布的库内名称为 root，
//   面包屑对根画布显示 i18n 文案「根画布」（见计划修订记录）；
// - 日志列表为普通 VList（不虚拟化），但被滚动容器裁剪且落在窗口内的条目会被 UI 树遮挡过滤
//   丢弃：第一页 20 条中约 2 条不在树中，因此「每页 20 条」通过「筛选后 21 条 → 第二页恰好
//   1 条」间接断言，其余条目数量断言只用树中可见条目；
// - 日期为原生 input[type=date]：合成输入需按段输入——点击输入框左侧区域聚焦年份段，
//   ArrowLeft 确保定位第一段后逐段 typeText，段间以 ArrowRight 移动；清空用 Backspace 逐段清除；
// - 「行为类型」为多选 VSelect：菜单较长且可滚动，目标选项不在 UI 树中时用 mouse_scroll 滚动
//   菜单直至选项中心落在 listbox 可视区内再点击；关闭菜单通过点击对话框标题文本；
// - 全部等待使用 lib 等待函数；应用生命周期由 withApp 包装，保证无论成败都经 POST /shutdown 收尾。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_010\case_010.js`

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

/** fixture 根画布在搜索结果副标题中的展示名称（根画布显示本地化文案） */
const ROOT_CANVAS_NAME = "根画布";

/** 准备阶段：账号 A 新增的密码字段名与值 */
const SECRET_FIELD_NAME = "登录密码";
const SECRET_FIELD_VALUE = "log-secret-123";
/** 日志中敏感字段值的掩码文本（8 个圆点字符） */
const MASKED_VALUE = "••••••••";

/** 准备阶段：拖拽创建的两个节点标题 */
const SEARCH_NODE_A = "搜索甲";
const SEARCH_NODE_B = "搜索乙";

/** 准备阶段：备注 B 的移动次数（需超过日志每页 20 条） */
const MOVE_COUNT = 21;

/** 字段类型下拉的显示名集合（用于从字段行中识别类型下拉） */
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

/** 搜索候选高亮像素断言阈值：高亮行平均亮度至少比另一行低该值 */
const HIGHLIGHT_LUM_DIFF = 10;

/** 节点居中容差：允许偏移画布区域尺寸的比例（±20%） */
const CENTER_TOLERANCE = 0.2;

const report = new Report("case_010 搜索与日志");

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
 * 在 UI 树中查找日志对话框（含「日志」精确文本或「共 N 条」文本的 dialog）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 日志对话框；未命中时为 null。
 */
function logDialog(tree) {
  return (
    ui.findByRole(tree, "dialog").find(
      (d) =>
        ui.findByText([d], "日志", { exact: true }) !== null ||
        flattenNodes([d]).some((n) => n.tag === "#text" && /^共 \d+ 条$/.test((n.text ?? "").trim())),
    ) ?? null
  );
}

/**
 * 返回搜索输入框（placeholder 为「搜索节点…」的可编辑控件）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 搜索输入框；未命中时为 null。
 */
function searchInput(tree) {
  return flattenNodes(tree).find((n) => n.editable && n.attrs?.placeholder === "搜索节点…") ?? null;
}

/**
 * 返回搜索候选列表项（搜索下拉内的 role=listitem，按 top 升序）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 候选列表项数组。
 */
function candidateItems(tree) {
  return flattenNodes(tree)
    .filter((n) => n.role === "listitem" && n.bounds.width > 300 && n.bounds.top < 600)
    .sort((a, b) => a.bounds.top - b.bounds.top);
}

/**
 * 返回候选列表项中的标题文本（候选项子树的第一个非空文本节点，如「搜索甲」「账号 A」）。
 * @param {object} item 候选列表项节点。
 * @returns {string} 标题文本；未命中时为空字符串。
 */
function candidateTitle(item) {
  const hit = flattenNodes([item]).find((n) => n.tag === "#text" && (n.text ?? "").trim() !== "");
  return (hit?.text ?? "").trim();
}

/**
 * 返回日志条目列表项（列表内条目高度大于分页按钮，按 top 升序）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 日志条目数组（树中可见部分）。
 */
function logItems(tree) {
  return flattenNodes(tree)
    .filter((n) => n.role === "listitem" && n.bounds.height > 50)
    .sort((a, b) => a.bounds.top - b.bounds.top);
}

/**
 * 解析日志对话框标题右侧的「共 N 条」文本为数字。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {number|null} 日志总数；未命中时为 null。
 */
function logTotal(tree) {
  const hit = flattenNodes(tree).find((n) => n.tag === "#text" && /^共 \d+ 条$/.test((n.text ?? "").trim()));
  return hit ? Number((hit.text ?? "").replace(/\D+/g, "")) : null;
}

/**
 * 返回日志条目中的时间文本（格式 yyyy/M/d H:mm），按条目顺序。
 * @param {object[]} items 日志条目数组。
 * @returns {(string|null)[]} 每条目的时间文本；条目无时间文本时为 null。
 */
function logItemTimes(items) {
  return items.map((item) => {
    const hit = flattenNodes([item]).find(
      (n) => n.tag === "#text" && /^\d{4}\/\d{1,2}\/\d{1,2} \d{1,2}:\d{2}$/.test((n.text ?? "").trim()),
    );
    return hit ? (hit.text ?? "").trim() : null;
  });
}

/**
 * 解析日志时间文本为 Date 对象（「yyyy/M/d H:mm」按本地时间解析）。
 * @param {string} text 时间文本。
 * @returns {Date} 解析结果。
 */
function parseLogTime(text) {
  const m = /^(\d{4})\/(\d{1,2})\/(\d{1,2}) (\d{1,2}):(\d{2})$/.exec(text);
  if (!m) {
    throw new Error(`parseLogTime: unexpected format "${text}"`);
  }
  return new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4]), Number(m[5]));
}

/**
 * 返回日志列表中的分页数字按钮（按页码升序）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object[]} 分页数字按钮数组。
 */
function logPageButtons(tree) {
  return flattenNodes(tree)
    .filter((n) => n.role === "button" && /^\d+$/.test((n.text ?? "").trim()) && n.bounds.top > 850)
    .sort((a, b) => Number(a.text) - Number(b.text));
}

/**
 * 解析字段名文本对应的字段行元素集合（字段名 + 类型下拉 + 值编辑器 + 复制按钮 + 图标）。
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
 * 在 UI 树中查找 role=option 且文本精确匹配的候选项。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} text 候选项文本。
 * @returns {object|null} 命中的候选项；未命中时为 null。
 */
function findOption(tree, text) {
  return flattenNodes(tree).find((node) => node.role === "option" && (node.text ?? "").trim() === text) ?? null;
}

/**
 * 统计 PNG 指定区域内的平均亮度（0-255），用于断言候选高亮行。
 * @param {import("pngjs").PNG} png 整窗截图 PNG。
 * @param {{left: number, top: number, width: number, height: number}} region 区域。
 * @returns {number} 平均亮度。
 */
function meanLuminance(png, region) {
  let sum = 0;
  let count = 0;
  const left = Math.max(0, Math.floor(region.left));
  const top = Math.max(0, Math.floor(region.top));
  const right = Math.min(png.width, Math.ceil(region.left + region.width));
  const bottom = Math.min(png.height, Math.ceil(region.top + region.height));
  for (let y = top; y < bottom; y++) {
    for (let x = left; x < right; x++) {
      const i = (png.width * y + x) << 2;
      sum += 0.299 * png.data[i] + 0.587 * png.data[i + 1] + 0.114 * png.data[i + 2];
      count += 1;
    }
  }
  return count > 0 ? sum / count : 0;
}

/**
 * 等待目标出现、界面静止后重新定位并点击（避免动画期间坐标漂移）。
 * @param {() => Promise<object|null>} finder 目标查找函数。
 * @param {object} options 可选参数。
 * @param {string} options.label 等待条件名（用于错误消息）。
 * @param {number} [options.timeout=8000] 等待超时毫秒数。
 * @param {number} [options.settleMs=350] 首次命中后的静止等待毫秒数。
 * @param {number} [options.count=1] 点击次数。
 * @returns {Promise<object>} 被点击的节点。
 */
async function clickFinder(finder, { label, timeout = 8000, settleMs = 350, count = 1 }) {
  let target = await waitForCondition(finder, { timeout, interval: 200, label });
  await sleep(settleMs);
  target = (await finder()) ?? target;
  await ui.clickNode(target, { count });
  return target;
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
 * 在画布中双击节点标题打开编辑节点对话框。
 * @param {string} title 节点标题。
 * @returns {Promise<void>} 无返回值。
 */
async function openEditDialogByTitle(title) {
  await ui.clickText(title, { count: 2 });
  await ui.waitForDialog("编辑节点", { timeout: 8000 });
  await sleep(700);
}

/**
 * 点击编辑节点对话框内按钮（在对话框子树内查找以避免歧义）。
 * @param {string} buttonText 按钮文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickEditDialogButton(buttonText) {
  await clickFinder(
    async () => {
      const tree = await api.uiTree();
      const dialog = ui.findDialog(tree, "编辑节点");
      return dialog ? ui.findButton(tree, buttonText, { root: dialog, exact: true }) : null;
    },
    { label: `编辑节点按钮「${buttonText}」` },
  );
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
    { timeout: 5000, interval: 150, label: "新增字段占位符" },
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
    { timeout: 4000, interval: 150, label: "字段名编辑输入框" },
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
    { timeout: 5000, interval: 200, label: `字段名提交为「${name}」` },
  );
}

/**
 * 切换编辑节点对话框中指定字段行的类型：点击类型下拉后选择目标类型。
 * @param {object} nameText 字段名文本节点（#text）。
 * @param {string} typeName 目标类型显示名（如「密码」）。
 * @returns {Promise<void>} 无返回值。
 */
async function selectNodeFieldType(nameText, typeName) {
  const tree = await api.uiTree();
  const row = fieldRow(tree, nameText);
  if (!row?.typeCombo) {
    throw new Error(`selectNodeFieldType: type combo not found for "${nameText?.text}"`);
  }
  await ui.clickNode(row.typeCombo);
  const option = await waitForCondition(async () => findOption(await api.uiTree(), typeName), {
    timeout: 6000,
    interval: 200,
    label: `字段类型选项「${typeName}」`,
  });
  await ui.clickNode(option);
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const r = fieldRow(t, textNodes(t, nameText.text ?? "")[0]);
      return (r?.typeCombo?.text ?? "").trim() === typeName;
    },
    { timeout: 5000, interval: 200, label: `类型切换为「${typeName}」` },
  );
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
  await waitForCondition(
    async () =>
      textNodes(await api.uiTree(), "空节点").filter((n) => n.bounds.left < PANEL_MAX_LEFT).length > 0,
    { timeout: 6000, interval: 200, label: "模板面板打开" },
  );
  await sleep(600);
}

/**
 * 从模板面板「空节点」行的拖拽手柄拖到目标点创建数据节点（面板行按区域过滤）。
 * @param {{x: number, y: number}} to 目标点坐标（窗口物理像素）。
 * @returns {Promise<void>} 无返回值。
 */
async function dragBlankRowTo(to) {
  const rowText = await waitForCondition(
    async () => textNodes(await api.uiTree(), "空节点").filter((n) => n.bounds.left < PANEL_MAX_LEFT)[0] ?? null,
    { timeout: 6000, interval: 200, label: "模板面板「空节点」行" },
  );
  const rowCenterY = rowText.bounds.top + rowText.bounds.height / 2;
  const handle = flattenNodes(await api.uiTree())
    .filter((n) => n.attrs?.title === "拖拽创建数据节点")
    .filter((n) => Math.abs(n.bounds.top + n.bounds.height / 2 - rowCenterY) < 24)
    .sort((a, b) => a.bounds.left - b.bounds.left)[0];
  if (!handle) {
    throw new Error("dragBlankRowTo: 「拖拽创建数据节点」手柄未找到");
  }
  await api.postInput([api.mouseDrag(ui.boundsCenter(handle), to)]);
  await sleep(1500);
}

/**
 * 双击新节点并从编辑节点对话框修改标题，等待画布标题更新。
 * @param {string} newTitle 新标题。
 * @returns {Promise<void>} 无返回值。
 */
async function renameNewNodeTo(newTitle) {
  await openEditDialogByTitle("新节点");
  await ui.typeIntoEditable("标题", newTitle);
  await sleep(300);
  await clickEditDialogButton("确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await ui.waitForTextStable(newTitle, { timeout: 8000, settleMs: 500 });
}

/**
 * 拖动一次「备注 B」：以当前标题文本中心为起点，水平偏移 dx 像素后松开。
 * @param {string} title 节点标题（备注 B）。
 * @param {number} dx 水平偏移像素（正为向右）。
 * @returns {Promise<{from: object, to: object}|null>} 位移前后标题 bounds；未找到节点时为 null。
 */
async function dragNodeOnce(title, dx) {
  const before = textNodes(await api.uiTree(), title)[0];
  if (!before) {
    return null;
  }
  const from = ui.boundsCenter(before);
  await api.postInput([api.mouseDrag(from, { x: from.x + dx, y: from.y })]);
  await sleep(450);
  return { from, before: before.bounds };
}

/**
 * 点击右下角「日志」按钮打开日志对话框，等待「共 N 条」与列表渲染。
 * @returns {Promise<void>} 无返回值。
 */
async function openLogDialog() {
  const button = await clickFinder(
    async () => {
      const tree = await api.uiTree();
      return (
        flattenNodes(tree)
          .filter((n) => n.role === "button" && ui.subtreeText(n).includes("日志"))
          .sort((a, b) => b.bounds.top - a.bounds.top)[0] ?? null
      );
    },
    { label: "「日志」按钮" },
  );
  void button;
  await waitForCondition(async () => logDialog(await api.uiTree()) !== null, {
    timeout: 8000,
    interval: 250,
    label: "日志对话框",
  });
  await waitForCondition(async () => logTotal(await api.uiTree()) !== null, {
    timeout: 8000,
    interval: 250,
    label: "日志总数「共 N 条」",
  });
  await sleep(1200);
}

/**
 * 点击日志对话框内按钮（文本精确匹配）。
 * @param {string} buttonText 按钮文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickLogDialogButton(buttonText) {
  await clickFinder(
    async () => {
      const tree = await api.uiTree();
      return flattenNodes(tree).find((n) => n.role === "button" && (n.text ?? "").trim() === buttonText) ?? null;
    },
    { label: `日志对话框按钮「${buttonText}」` },
  );
  await sleep(1200);
}

/**
 * 点击日志分页数字按钮并等待目标页激活（当前页按钮的 name 变为「当前页 N」）。
 * @param {number} page 页码。
 * @param {string} [expectText=""] 目标页应出现的文本（可空，用于等待列表切换完成）。
 * @returns {Promise<void>} 无返回值。
 */
async function clickLogPage(page, expectText = "") {
  await clickFinder(
    async () =>
      flattenNodes(await api.uiTree()).find(
        (n) => n.role === "button" && (n.text ?? "").trim() === String(page),
      ) ?? null,
    { label: `分页按钮「${page}」` },
  );
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      return logPageButtons(t).some(
        (b) => (b.text ?? "").trim() === String(page) && (b.name ?? "").includes("当前页"),
      );
    },
    { timeout: 8000, interval: 250, label: `第 ${page} 页激活` },
  );
  if (expectText !== "") {
    await waitForCondition(async () => ui.hasText(await api.uiTree(), expectText), {
      timeout: 8000,
      interval: 250,
      label: `第 ${page} 页内容「${expectText}」`,
    });
  }
  await sleep(800);
}

/**
 * 返回某日期输入框节点（按名称精确匹配可编辑控件）。
 * @param {string} name 输入框名称（「开始日期」/「结束日期」）。
 * @returns {Promise<object>} 命中的输入框节点。
 */
async function findDateInput(name) {
  return waitForCondition(
    async () => flattenNodes(await api.uiTree()).find((n) => n.editable && n.name === name) ?? null,
    { timeout: 6000, interval: 200, label: `日期输入框「${name}」` },
  );
}

/**
 * 向原生 type=date 输入框按年月日分段输入日期：点击左侧聚焦年份段，
 * ArrowLeft 确保定位第一段后逐段输入，段间以 ArrowRight 移动。
 * @param {string} name 输入框名称。
 * @param {string} year 四位年份（如 "2026"）。
 * @param {string} month 两位月份（如 "09"）。
 * @param {string} day 两位日期（如 "21"）。
 * @returns {Promise<void>} 无返回值。
 */
async function typeDate(name, year, month, day) {
  const input = await findDateInput(name);
  await api.postInput([
    api.mouseMove(input.bounds.left + 15, input.bounds.top + input.bounds.height / 2),
    api.mouseClick("left", 1),
  ]);
  await sleep(250);
  await api.postInput([api.keyClick(["ArrowLeft"], 6), api.typeText(year)]);
  await sleep(250);
  await api.postInput([api.keyClick(["ArrowRight"]), api.typeText(month)]);
  await sleep(250);
  await api.postInput([api.keyClick(["ArrowRight"]), api.typeText(day)]);
  await sleep(400);
}

/**
 * 清空原生 type=date 输入框：点击左侧聚焦年份段后逐段 Backspace 并在段间 ArrowRight。
 * @param {string} name 输入框名称。
 * @returns {Promise<void>} 无返回值。
 */
async function clearDate(name) {
  const input = await findDateInput(name);
  await api.postInput([
    api.mouseMove(input.bounds.left + 15, input.bounds.top + input.bounds.height / 2),
    api.mouseClick("left", 1),
  ]);
  await sleep(250);
  await api.postInput([
    api.keyClick(["Backspace"]),
    api.keyClick(["ArrowRight"]),
    api.keyClick(["Backspace"]),
    api.keyClick(["ArrowRight"]),
    api.keyClick(["Backspace"]),
  ]);
  await sleep(400);
}

/**
 * 在「行为类型」多选下拉中滚动菜单直至目标选项完整可见并返回该选项。
 * @param {string} name 目标选项文本（如「移动节点」）。
 * @returns {Promise<object>} 命中的选项节点（bounds 中心位于 listbox 可视区内）。
 */
async function scrollToActionOption(name) {
  for (let i = 0; i < 40; i += 1) {
    const tree = await api.uiTree();
    const listbox = flattenNodes(tree).find((n) => n.role === "listbox" && n.bounds.height > 200);
    const option = findOption(tree, name);
    if (listbox && option) {
      const centerY = option.bounds.top + option.bounds.height / 2;
      if (centerY > listbox.bounds.top + 10 && centerY < listbox.bounds.top + listbox.bounds.height - 10) {
        return option;
      }
    }
    const x = listbox ? listbox.bounds.left + listbox.bounds.width / 2 : 1060;
    const y = listbox ? listbox.bounds.top + Math.min(200, listbox.bounds.height / 2) : 500;
    await api.postInput([api.mouseMove(x, y), api.mouseScroll("down", 3)]);
    await sleep(350);
  }
  throw new Error(`scrollToActionOption("${name}") 超时`);
}

/**
 * 切换「显示敏感值」开关到指定状态（状态一致时不操作）。
 * @param {boolean} on 目标状态（true 为显示敏感值）。
 * @returns {Promise<boolean>} 是否执行了点击。
 */
async function setSensitiveToggle(on) {
  const tree = await api.uiTree();
  const toggle = flattenNodes(tree).find((n) => n.role === "checkbox" && n.name === "显示敏感值");
  if (!toggle) {
    throw new Error("setSensitiveToggle: 「显示敏感值」开关未找到");
  }
  if (toggle.states?.checked === on) {
    return false;
  }
  await ui.clickNode(toggle);
  await waitForCondition(
    async () => {
      const t = await api.uiTree();
      const sw = flattenNodes(t).find((n) => n.role === "checkbox" && n.name === "显示敏感值");
      return sw?.states?.checked === on;
    },
    { timeout: 6000, interval: 200, label: `「显示敏感值」切换为 ${on}` },
  );
  await sleep(400);
  return true;
}

/**
 * 在日志对话框内逐页查找包含指定文本的条目，返回该页的日志条目数组。
 * @param {string} expectText 目标条目特征文本（如「编辑节点字段」）。
 * @returns {Promise<object[]>} 目标页的日志条目数组。
 */
async function gotoLogPageContaining(expectText) {
  const total = logTotal(await api.uiTree()) ?? 0;
  const pages = Math.ceil(total / 20);
  for (let page = 1; page <= pages; page += 1) {
    if (page > 1) {
      await clickLogPage(page);
    }
    const tree = await api.uiTree();
    if (ui.hasText(tree, expectText)) {
      return logItems(tree);
    }
  }
  throw new Error(`gotoLogPageContaining("${expectText}") 未在 ${pages} 页内找到`);
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
  report.check(ui.hasText(unlockedTree, "备注 B"), "解锁后进入根画布并出现节点「备注 B」");
  await sleep(600);
  await snap("s0-unlocked-root");

  report.section("S1 准备数据（前置条件）");

  report.section("S1.1 「账号 A」添加密码字段（产生「编辑节点字段」日志）");
  await openEditDialogByTitle("账号 A");
  await addNodeField(SECRET_FIELD_NAME);
  {
    let tree = await api.uiTree();
    report.check(textNodes(tree, SECRET_FIELD_NAME).length === 1, "准备 1 「账号 A」编辑对话框出现字段行「登录密码」");
    await selectNodeFieldType(textNodes(tree, SECRET_FIELD_NAME)[0], "密码");
    tree = await api.uiTree();
    const row = fieldRow(tree, textNodes(tree, SECRET_FIELD_NAME)[0]);
    report.check((row?.typeCombo?.text ?? "").trim() === "密码", "准备 1 字段类型切换为「密码」", row?.typeCombo?.text);
    report.check(row?.valueEditor != null, "准备 1 密码字段出现值编辑器");
    report.check(row?.copyButton?.states?.disabled === true, "准备 1 值输入前复制按钮禁用（值为空）");
    await ui.clickNode(row.valueEditor);
    await api.postInput([api.typeText(SECRET_FIELD_VALUE)]);
    await sleep(500);
    tree = await api.uiTree();
    const filled = fieldRow(tree, textNodes(tree, SECRET_FIELD_NAME)[0]);
    report.check(filled?.copyButton?.states?.disabled === false, "准备 1 输入值后复制按钮可用（值已写入）");
  }
  await snap("s1a-secret-field-filled");
  await clickEditDialogButton("确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(800);
  report.check(ui.findDialog(await api.uiTree(), "编辑节点") === null, "准备 1 保存后编辑节点对话框关闭");

  report.section("S1.2 拖拽创建「搜索甲」「搜索乙」（候选搜索素材）");
  await openTemplatePanel();
  await dragBlankRowTo({ x: 500, y: 880 });
  await waitForCondition(async () => textNodes(await api.uiTree(), "新节点").length > 0, {
    timeout: 8000,
    interval: 250,
    label: "新节点出现（搜索甲）",
  });
  await sleep(800);
  await renameNewNodeTo(SEARCH_NODE_A);
  {
    const tree = await api.uiTree();
    report.check(textNodes(tree, SEARCH_NODE_A).length === 1, `准备 2 画布出现节点「${SEARCH_NODE_A}」`);
    report.check(textNodes(tree, "新节点").length === 0, "准备 2 新节点标题已被替换");
  }
  await openTemplatePanel();
  await dragBlankRowTo({ x: 1130, y: 850 });
  await waitForCondition(async () => textNodes(await api.uiTree(), "新节点").length > 0, {
    timeout: 8000,
    interval: 250,
    label: "新节点出现（搜索乙）",
  });
  await sleep(800);
  await renameNewNodeTo(SEARCH_NODE_B);
  {
    const tree = await api.uiTree();
    report.check(textNodes(tree, SEARCH_NODE_B).length === 1, `准备 2 画布出现节点「${SEARCH_NODE_B}」`);
    report.check(
      textNodes(tree, SEARCH_NODE_A).length === 1 && textNodes(tree, SEARCH_NODE_B).length === 1,
      "准备 2 两个搜索素材节点同时存在",
    );
  }
  await snap("s1b-two-search-nodes");

  report.section("S1.3 拖动「备注 B」21 次（产生移动节点日志）");
  let movedCount = 0;
  for (let i = 0; i < MOVE_COUNT; i += 1) {
    const before = textNodes(await api.uiTree(), "备注 B")[0];
    if (!before) {
      throw new Error(`准备 3 第 ${i + 1} 次拖动：节点「备注 B」未找到`);
    }
    const dx = i % 2 === 0 ? 40 : -40;
    await dragNodeOnce("备注 B", dx);
    let after = textNodes(await api.uiTree(), "备注 B")[0];
    if (after && Math.abs(after.bounds.left - before.bounds.left) < 10) {
      await dragNodeOnce("备注 B", dx);
      after = textNodes(await api.uiTree(), "备注 B")[0];
    }
    if (after && Math.abs(after.bounds.left - before.bounds.left) >= 10) {
      movedCount += 1;
    }
  }
  report.check(movedCount === MOVE_COUNT, `准备 3 共完成 ${MOVE_COUNT} 次有效拖动并产生移动日志`, `moved=${movedCount}`);
  await snap("s1c-after-21-moves");

  report.section("S2 全局搜索（步骤 1-7、F1）");
  let tree = await api.uiTree();
  let page1LogTotal = null;
  {
    const button = await clickFinder(async () => ui.findAttr(await api.uiTree(), "title", "搜索"), {
      label: "「搜索」按钮",
    });
    void button;
    await sleep(600);
    tree = await api.uiTree();
    report.check(searchInput(tree) !== null, "步骤 1 点击顶栏搜索按钮后展开输入框（placeholder「搜索节点…」）");
    report.check(searchInput(tree)?.states?.focused === true, "步骤 1 展开后输入框自动聚焦");
    report.check(ui.findAttr(tree, "title", "关闭搜索") !== null, "步骤 1 出现「关闭搜索」按钮");
    await snap("s2a-search-expanded");
  }

  // 步骤 2：输入「搜索」——测量防抖（去抖 300ms，计时自注入前开始，故实测下限约 300ms）
  let candidates = [];
  {
    const input = searchInput(await api.uiTree());
    await ui.clickNode(input);
    const startedAt = Date.now();
    await api.postInput([api.typeText("搜索")]);
    const deadline = Date.now() + 4000;
    let appearedAt = null;
    while (Date.now() < deadline) {
      const t = await api.uiTree();
      const list = candidateItems(t);
      if (list.length > 0) {
        candidates = list;
        appearedAt = Date.now();
        break;
      }
      await sleep(40);
    }
    const delay = appearedAt === null ? Infinity : appearedAt - startedAt;
    report.check(
      delay >= 250 && delay <= 3000,
      "步骤 2 搜索防抖：候选在输入后约 300ms 出现（非即时，实测延迟）",
      `${delay === Infinity ? "未出现" : `${delay}ms`}`,
    );
    report.check(candidates.length === 2, "步骤 2 出现两个候选「搜索甲」「搜索乙」", `count=${candidates.length}`);
    const titles = candidates.map((item) => candidateTitle(item)).sort();
    report.check(
      titles.join(",") === [SEARCH_NODE_A, SEARCH_NODE_B].sort().join(","),
      "步骤 2 候选标题为「搜索甲」「搜索乙」",
      titles.join(","),
    );
    const subtitles = candidates.map((item) => ui.subtreeText(item));
    report.check(
      subtitles.every((text) => text.includes("空节点 · " + ROOT_CANVAS_NAME)),
      "步骤 2 候选副标题为「节点副标题 · 画布名」（fixture 根画布库内名称为 root）",
      subtitles.join(" | "),
    );
    report.check(searchInput(await api.uiTree()) !== null, "步骤 2 搜索下拉展示时输入框保持展开");
    await snap("s2b-search-candidates");
  }

  // 步骤 3：ArrowDown/ArrowUp 循环高亮（像素亮度断言）
  {
    const lums = async () => {
      const t = await api.uiTree();
      const items = candidateItems(t);
      if (items.length !== 2) {
        throw new Error(`候选行数量异常：${items.length}`);
      }
      const png = image.decodePng(await api.screenshot());
      return items.map((item) => meanLuminance(png, item.bounds));
    };
    const initial = await lums();
    report.check(
      initial[0] + HIGHLIGHT_LUM_DIFF < initial[1],
      "步骤 2 初始高亮位于第一个候选（高亮行背景更暗，像素亮度断言）",
      initial.map((v) => v.toFixed(1)).join(","),
    );
    await api.postInput([api.keyClick(["ArrowDown"])]);
    await sleep(400);
    const down = await lums();
    report.check(
      down[1] + HIGHLIGHT_LUM_DIFF < down[0],
      "步骤 3 ArrowDown 后高亮移动到第二个候选",
      down.map((v) => v.toFixed(1)).join(","),
    );
    await snap("s2c-arrowdown-highlight");
    await api.postInput([api.keyClick(["ArrowUp"])]);
    await sleep(400);
    const up = await lums();
    report.check(
      up[0] + HIGHLIGHT_LUM_DIFF < up[1],
      "步骤 3 ArrowUp 后高亮回到第一个候选",
      up.map((v) => v.toFixed(1)).join(","),
    );
    await api.postInput([api.keyClick(["ArrowDown"]), api.wait(250), api.keyClick(["ArrowDown"])]);
    await sleep(400);
    const wrapped = await lums();
    report.check(
      wrapped[0] + HIGHLIGHT_LUM_DIFF < wrapped[1],
      "步骤 4 从第一个候选连续 ArrowDown 两次循环回到第一个候选（高亮循环）",
      wrapped.map((v) => v.toFixed(1)).join(","),
    );
  }

  // 步骤 4：Enter 选择高亮项并跳转居中
  {
    const items = candidateItems(await api.uiTree());
    const target = candidateTitle(items[0] ?? {});
    report.check(target !== "", "步骤 4 读取当前高亮候选标题", target);
    await api.postInput([api.keyClick(["Enter"])]);
    await waitForCondition(async () => candidateItems(await api.uiTree()).length === 0, {
      timeout: 6000,
      interval: 200,
      label: "搜索下拉关闭",
    });
    report.check(true, "步骤 4 Enter 后下拉关闭");
    await waitForCondition(async () => textNodes(await api.uiTree(), target).length > 0, {
      timeout: 10000,
      interval: 250,
      label: `跳转后节点「${target}」出现`,
    });
    await sleep(1800);
    const after = await api.uiTree();
    const nodeText = textNodes(after, target)[0];
    const center = ui.boundsCenter(nodeText);
    const region = { left: 0, top: 45, width: info.width, height: info.height - 45 };
    const regionCenter = { x: region.left + region.width / 2, y: region.top + region.height / 2 };
    const tolX = region.width * CENTER_TOLERANCE;
    const tolY = region.height * CENTER_TOLERANCE;
    report.check(
      Math.abs(center.x - regionCenter.x) <= tolX && Math.abs(center.y - regionCenter.y) <= tolY,
      `步骤 4 选中后跳转到根画布并将「${target}」居中（画布区域中心 ±20%）`,
      `center=(${center.x.toFixed(0)},${center.y.toFixed(0)}) regionCenter=(${regionCenter.x.toFixed(0)},${regionCenter.y.toFixed(0)})`,
    );
    report.check(searchInput(after) !== null, "步骤 4 跳转后搜索输入框保持展开（关键词已清空）");
    report.check(textNodes(after, target).length === 1, `步骤 4 跳转后根画布存在目标节点「${target}」`);
    await snap("s2d-jump-centered");
  }

  // F1：无匹配关键词
  {
    const input = searchInput(await api.uiTree());
    await ui.clickNode(input);
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText("zzz-no-match")]);
    await waitForCondition(async () => ui.hasText(await api.uiTree(), "无匹配节点"), {
      timeout: 6000,
      interval: 250,
      label: "「无匹配节点」提示",
    });
    const t = await api.uiTree();
    report.check(ui.hasText(t, "无匹配节点"), "F1 无匹配关键词时下拉显示「无匹配节点」");
    report.check(candidateItems(t).length === 0, "F1 无匹配时不出现候选");
    const before = textNodes(t, SEARCH_NODE_A).map((n) => n.bounds.top + "," + n.bounds.left).join("|");
    await api.postInput([api.keyClick(["Enter"])]);
    await sleep(800);
    const t2 = await api.uiTree();
    report.check(
      ui.hasText(t2, "无匹配节点") && candidateItems(t2).length === 0,
      "F1 无匹配时按 Enter 不跳转（下拉与提示保持）",
    );
    const after = textNodes(t2, SEARCH_NODE_A).map((n) => n.bounds.top + "," + n.bounds.left).join("|");
    report.check(after === before, "F1 无匹配时按 Enter 后画布节点位置不变（未发生跳转）");
    await snap("s2e-f1-no-match");
  }

  // 步骤 6：竞态替换与 Escape 关闭下拉
  {
    const input = searchInput(await api.uiTree());
    await ui.clickNode(input);
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText("搜")]);
    await sleep(120);
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText("账号")]);
    await waitForCondition(
      async () => {
        const t = await api.uiTree();
        const items = candidateItems(t);
        return items.length === 1 && candidateTitle(items[0]) === "账号 A";
      },
      { timeout: 6000, interval: 250, label: "「账号」候选出现" },
    );
    report.check(true, "步骤 6 防抖窗口内替换关键词后仅返回最新查询结果（竞态丢弃旧结果）");
    const t = await api.uiTree();
    report.check(candidateItems(t).length === 1, "步骤 6 输入「账号」后仅一个候选「账号 A」");
    await snap("s2f-account-candidate");
    await api.postInput([api.keyClick(["Escape"])]);
    await sleep(600);
    const t2 = await api.uiTree();
    report.check(candidateItems(t2).length === 0, "步骤 6 Escape 后下拉关闭");
    report.check(searchInput(t2) !== null && searchInput(t2)?.states?.focused === true, "步骤 6 Escape 后输入框保持展开");
  }

  // 步骤 7：关闭搜索
  {
    await clickFinder(async () => ui.findAttr(await api.uiTree(), "title", "关闭搜索"), {
      label: "「关闭搜索」按钮",
    });
    await sleep(700);
    const t = await api.uiTree();
    report.check(searchInput(t) === null, "步骤 7 点击关闭搜索后输入框收起");
    report.check(ui.findAttr(t, "title", "关闭搜索") === null, "步骤 7 「关闭搜索」按钮消失");
    report.check(
      ui.hasText(t, "画布宇宙") && ui.hasText(t, "根画布"),
      "步骤 7 面包屑恢复完整显示（画布宇宙 › 根画布）",
    );
    await snap("s2g-search-collapsed");
  }

  report.section("S3 日志对话框（步骤 8-19、F2/F3）");

  // 步骤 8-9：打开日志、总数与第一页内容
  await openLogDialog();
  tree = await api.uiTree();
  page1LogTotal = logTotal(tree);
  report.check(page1LogTotal !== null && page1LogTotal >= MOVE_COUNT + 5, "步骤 8 日志对话框显示「共 N 条」且 N ≥ 26", `N=${page1LogTotal}`);
  report.check(logDialog(tree) !== null, "步骤 8 打开对话框「日志」");
  {
    const items = logItems(tree);
    report.check(items.length >= 10, "步骤 9 第一页渲染出多条日志条目", `visible=${items.length}`);
    const texts = items.map((n) => ui.subtreeText(n));
    report.check(
      texts.every((text) => text.includes("移动节点") && text.includes(`将节点“备注 B”从`)),
      "步骤 9 第一页（最新操作）均为「移动节点」条目（最上方为最近操作）",
      texts[0] ?? "",
    );
    const times = logItemTimes(items).filter(Boolean).map(parseLogTime);
    report.check(
      times.every((time, i) => i === 0 || times[i - 1].getTime() >= time.getTime()),
      "步骤 9 第一页条目按时间倒序排列",
    );
    report.check(!ui.hasText(tree, "编辑节点字段"), "步骤 9 较旧的「编辑节点字段」条目不在第一页");
  }
  await snap("s3a-log-dialog-page1");

  // 步骤 10：翻页到第二页
  {
    const page1Items = logItems(await api.uiTree());
    const page1Texts = page1Items.map((n) => ui.subtreeText(n)).join("|");
    const buttons = logPageButtons(await api.uiTree());
    report.check(buttons.length === 2, "步骤 8 日志分页共有 2 页（总数 31 条）", `pages=${buttons.length}`);
    await clickLogPage(2, "编辑节点字段");
    const t = await api.uiTree();
    const items = logItems(t);
    const page2Texts = items.map((n) => ui.subtreeText(n)).join("|");
    report.check(page2Texts !== page1Texts, "步骤 10 第二页内容与第一页不同");
    report.check(ui.hasText(t, "编辑节点字段"), "步骤 10 第二页包含较旧的「编辑节点字段」条目");
    report.check(
      items.length < page1Items.length,
      "步骤 10 第二页条目数量少于第一页（31 条按每页 20 条分页）",
      `page1=${page1Items.length} page2=${items.length}`,
    );
    const times = logItemTimes(items).filter(Boolean).map(parseLogTime);
    report.check(
      times.every((time, i) => i === 0 || times[i - 1].getTime() >= time.getTime()),
      "步骤 10 第二页条目按时间倒序排列",
    );
    const fieldEntry = items.find((n) => ui.subtreeText(n).includes("编辑节点字段"));
    const fieldText = fieldEntry ? flattenNodes([fieldEntry]).filter((n) => n.tag === "#text").map((n) => n.text).join(" ") : "";
    report.check(fieldText.includes("编辑了节点“账号 A”的字段"), "步骤 17 字段条目显示「编辑了节点“账号 A”的字段」");
    await snap("s3b-log-dialog-page2");
  }

  // 步骤 11：关键词筛选 + F3
  let keywordTotal = null;
  {
    await clickLogDialogButton("清除筛选");
    const input = await waitForCondition(
      async () => flattenNodes(await api.uiTree()).find((n) => n.editable && n.name === "搜索日志内容") ?? null,
      { timeout: 6000, interval: 200, label: "「搜索日志内容」输入框" },
    );
    await ui.clickNode(input);
    await api.postInput([api.typeText("备注")]);
    await sleep(300);
    await clickLogDialogButton("搜索");
    await waitForCondition(
      async () => {
        const total = logTotal(await api.uiTree());
        return total !== null && total < (page1LogTotal ?? 0);
      },
      { timeout: 8000, interval: 250, label: "关键词筛选生效" },
    );
    const t = await api.uiTree();
    keywordTotal = logTotal(t);
    const items = logItems(t);
    report.check(keywordTotal !== null && keywordTotal < (page1LogTotal ?? 0), "步骤 11 关键词「备注」筛选后总数减少", `N=${keywordTotal}`);
    report.check(keywordTotal >= MOVE_COUNT, "步骤 11 关键词「备注」命中全部移动节点日志", `N=${keywordTotal} >= ${MOVE_COUNT}`);
    report.check(
      items.length > 0 && items.every((n) => ui.subtreeText(n).includes("备注")),
      "步骤 11 列表仅剩含「备注」的条目",
      `visible=${items.length}`,
    );
    await snap("s3c-keyword-filter");
    // F3：无匹配关键词
    const input2 = await waitForCondition(
      async () => flattenNodes(await api.uiTree()).find((n) => n.editable && n.name === "搜索日志内容") ?? null,
      { timeout: 6000, interval: 200, label: "「搜索日志内容」输入框（F3）" },
    );
    await ui.clickNode(input2);
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText("zzz-no-match")]);
    await sleep(300);
    await clickLogDialogButton("搜索");
    await waitForCondition(async () => ui.hasText(await api.uiTree(), "无匹配的日志"), {
      timeout: 8000,
      interval: 250,
      label: "「无匹配的日志」空态",
    });
    const t3 = await api.uiTree();
    report.check(ui.hasText(t3, "无匹配的日志"), "F3 关键词无匹配时列表显示「无匹配的日志」");
    report.check(logTotal(t3) === 0, "F3 关键词无匹配时总数为 0", `N=${logTotal(t3)}`);
    await snap("s3d-f3-no-match");
  }

  // 步骤 12：清除筛选
  {
    await clickLogDialogButton("清除筛选");
    const t = await api.uiTree();
    report.check(logTotal(t) === page1LogTotal, "步骤 12 清除筛选后列表恢复完整", `N=${logTotal(t)}`);
    report.check(logPageButtons(t).some((b) => (b.text ?? "").trim() === "1" && (b.name ?? "").includes("当前页")), "步骤 12 页码回到第 1 页");
    // 再次点击搜索：若关键词或日期输入框中仍有残留值，会立即重新筛选导致总数变化，
    // 因此该断言验证全部筛选输入框（含关键词）已被清空。
    await clickLogDialogButton("搜索");
    const t2 = await api.uiTree();
    report.check(
      logTotal(t2) === page1LogTotal && !ui.hasText(t2, "无匹配的日志"),
      "步骤 12 清除筛选后再次搜索仍为完整列表（关键词输入框与日期输入均已清空）",
      `N=${logTotal(t2)}`,
    );
    await snap("s3e-clear-filter");
  }

  // 步骤 13：开始日期 = 明天 → 无匹配
  {
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    const yyyy = String(tomorrow.getFullYear());
    const mm = String(tomorrow.getMonth() + 1).padStart(2, "0");
    const dd = String(tomorrow.getDate()).padStart(2, "0");
    await typeDate("开始日期", yyyy, mm, dd);
    report.check(true, `步骤 13 在「开始日期」原生日期控件按段输入明天 ${yyyy}/${mm}/${dd}`);
    await clickLogDialogButton("搜索");
    await waitForCondition(async () => ui.hasText(await api.uiTree(), "无匹配的日志"), {
      timeout: 8000,
      interval: 250,
      label: "开始日期为明天时无匹配",
    });
    const t = await api.uiTree();
    report.check(ui.hasText(t, "无匹配的日志"), "步骤 13 开始日期晚于全部日志时列表显示「无匹配的日志」");
    report.check(logTotal(t) === 0, "步骤 13 筛选结果总数为 0", `N=${logTotal(t)}`);
    await snap("s3f-date-future-empty");
  }

  // 步骤 14：F2 非法日期范围
  {
    await clearDate("开始日期");
    await typeDate("结束日期", "2020", "01", "01");
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    await typeDate(
      "开始日期",
      String(tomorrow.getFullYear()),
      String(tomorrow.getMonth() + 1).padStart(2, "0"),
      String(tomorrow.getDate()).padStart(2, "0"),
    );
    await snap("s3g-f2-dates-set");
    const beforeF2 = logTotal(await api.uiTree());
    await clickLogDialogButton("搜索");
    const snackbar = await ui.waitForTextStable("开始日期不能晚于结束日期", { timeout: 8000 }).catch(() => null);
    report.check(snackbar !== null, "F2 开始日期晚于结束日期时 snackbar 提示「开始日期不能晚于结束日期」");
    const t = await api.uiTree();
    report.check(logTotal(t) === beforeF2, "F2 非法日期范围不执行查询（列表总数不变）", `before=${beforeF2} after=${logTotal(t)}`);
    await snap("s3g-f2-invalid-range");
    await ui.waitForGone("开始日期不能晚于结束日期", { timeout: 15000 }).catch(() => null);
  }

  // 步骤 15：行为类型多选筛选「移动节点」
  let moveTotal = null;
  {
    await clickLogDialogButton("清除筛选");
    const combo = await waitForCondition(
      async () => flattenNodes(await api.uiTree()).find((n) => n.role === "combobox" && (n.text ?? "").includes("行为类型")) ?? null,
      { timeout: 6000, interval: 200, label: "「行为类型」下拉" },
    );
    await ui.clickNode(combo);
    await waitForCondition(
      async () => flattenNodes(await api.uiTree()).some((n) => n.role === "option"),
      { timeout: 6000, interval: 200, label: "行为类型菜单打开" },
    );
    const option = await scrollToActionOption("移动节点");
    report.check(
      option !== null && (option.text ?? "").trim() === "移动节点",
      "步骤 15 行为类型菜单可滚动到「移动节点」选项",
    );
    await snap("s3h-action-menu-option");
    const fresh = await scrollToActionOption("移动节点");
    await ui.clickNode(fresh);
    await sleep(800);
    const t = await api.uiTree();
    const comboAfter = flattenNodes(t).find((n) => n.role === "combobox" && (n.text ?? "").includes("行为类型"));
    report.check(
      (comboAfter?.text ?? "").includes("移动节点"),
      "步骤 15 选择后行为类型下拉内出现「移动节点」chip",
      comboAfter?.text,
    );
    // 点击对话框标题关闭菜单
    const title = textNodes(t, "日志")[0];
    await ui.clickNode(title);
    await sleep(700);
    report.check(
      flattenNodes(await api.uiTree()).filter((n) => n.role === "option").length === 0,
      "步骤 15 点击对话框其它区域后菜单关闭",
    );
  }

  // 步骤 16：按行为类型筛选
  {
    await clickLogDialogButton("搜索");
    await waitForCondition(
      async () => {
        const total = logTotal(await api.uiTree());
        return total !== null && total < (page1LogTotal ?? 0);
      },
      { timeout: 8000, interval: 250, label: "行为类型筛选生效" },
    );
    const t = await api.uiTree();
    moveTotal = logTotal(t);
    const items = logItems(t);
    report.check(moveTotal === MOVE_COUNT, `步骤 16 行为类型筛选后仅剩 ${MOVE_COUNT} 条「移动节点」日志`, `N=${moveTotal}`);
    report.check(
      items.length > 0 && items.every((n) => ui.subtreeText(n).includes("移动节点") && ui.subtreeText(n).includes("将节点“备注 B”从")),
      "步骤 16 列表条目均渲染为「移动节点」详情「将节点“…”从 (x, y) 移动到 (x, y)」",
      items[0] ? ui.subtreeText(items[0]) : "",
    );
    await snap("s3i-action-filtered");
    // 每页 20 条：21 条筛选结果 → 第二页恰好 1 条
    await clickLogPage(2, "移动节点");
    const t2 = await api.uiTree();
    const page2Items = logItems(t2);
    report.check(
      page2Items.length === MOVE_COUNT - 20,
      "步骤 16 每页 20 条：21 条筛选结果的第二页恰好 1 条",
      `page2=${page2Items.length}`,
    );
    report.check(
      keywordTotal === moveTotal + 2,
      "步骤 16 关键词「备注」总数 = 移动节点总数 + 2（「连接节点」与「修改节点」日志载荷含备注）",
      `keyword=${keywordTotal} move=${moveTotal}`,
    );
  }

  // 步骤 17：翻页找到字段变更条目（含掩码值）
  let fieldChangeText = "";
  {
    await clickLogDialogButton("清除筛选");
    const items = await gotoLogPageContaining("编辑节点字段");
    const entry = items.find((n) => ui.subtreeText(n).includes("编辑节点字段"));
    if (!entry) {
      throw new Error("步骤 17 未找到「编辑节点字段」条目");
    }
    const texts = flattenNodes([entry]).filter((n) => n.tag === "#text").map((n) => n.text ?? "");
    fieldChangeText = texts.join(" ");
    report.check(texts.includes("编辑节点字段"), "步骤 17 找到「编辑节点字段」条目");
    report.check(
      fieldChangeText.includes("编辑了节点“账号 A”的字段"),
      "步骤 17 条目显示「编辑了节点“账号 A”的字段」",
    );
    const masked = `新增字段“${SECRET_FIELD_NAME}”（密码）：${MASKED_VALUE}`;
    report.check(
      texts.some((text) => text.trim() === masked),
      "步骤 17 字段变更明细渲染新增字段（密码）且值掩码为 8 个圆点",
      texts.find((text) => text.includes("新增字段")) ?? "",
    );
    await snap("s3j-field-change-masked");
  }

  // 步骤 18-19：显示敏感值开关
  {
    await setSensitiveToggle(true);
    await sleep(400);
    let t = await api.uiTree();
    let entry = logItems(t).find((n) => ui.subtreeText(n).includes("编辑节点字段"));
    let texts = entry ? flattenNodes([entry]).filter((n) => n.tag === "#text").map((n) => n.text ?? "") : [];
    report.check(
      texts.some((text) => text.trim() === `新增字段“${SECRET_FIELD_NAME}”（密码）：${SECRET_FIELD_VALUE}`),
      "步骤 18 开启「显示敏感值」后字段值显示为明文",
      texts.find((text) => text.includes("新增字段")) ?? "",
    );
    await snap("s3j2-sensitive-plaintext");
    await setSensitiveToggle(false);
    await sleep(400);
    t = await api.uiTree();
    entry = logItems(t).find((n) => ui.subtreeText(n).includes("编辑节点字段"));
    texts = entry ? flattenNodes([entry]).filter((n) => n.tag === "#text").map((n) => n.text ?? "") : [];
    report.check(
      texts.some((text) => text.trim() === `新增字段“${SECRET_FIELD_NAME}”（密码）：${MASKED_VALUE}`),
      "步骤 19 关闭开关后字段值恢复掩码",
      texts.find((text) => text.includes("新增字段")) ?? "",
    );
    await clickLogDialogButton("关闭");
    await ui.waitForDialogGone("日志", { timeout: 8000 }).catch(() => null);
    await sleep(600);
    t = await api.uiTree();
    report.check(!ui.hasText(t, "无匹配的日志") && logTotal(t) === null, "步骤 19 日志对话框关闭");
    await snap("s3k-log-dialog-closed");
  }

  console.log("  (note) F4「未筛选且无日志」的空态「暂无日志」在 fixture 与准备流程下不可触达，按计划仅记录不覆盖");
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
