// case_011 导出与 KeePass 导入。
//
// 流程：复制 base fixture → 启动应用 → 解锁（lastScene 恢复根画布）
// → 准备数据：为「账号 A」添加字段「口令」= secret-export-123，
//   为「备注 B」添加字段「备注字段」= note-export-value
// → 导出三种模式（不包含字段 / 包含字段但不包含字段值 / 包含字段且包含字段值）：
//   工具菜单 → 导出数据库 → 对话框（明文警告 / 三个单选与说明 / 默认打码模式）
//   → 确认 → save 文件选择器（默认文件名「用户数据库.md」）→ 输出到 output\
//   → 等 snackbar「数据库已导出」→ 脚本读取三个 md 文件做含/不含明文的字段级断言
// → KeePass 导入（步骤 8-13 + F1-F4）：
//   对话框结构（只读路径框 / Master Password / 两个导入方式单选）→ 未选文件提交（F1 内联提示）
//   → 选择文件（open 模式、kdbx 扩展名过滤；F4 变体：开启「显示全部文件」后 .txt 行置灰
//   且不可选中，计划预期的「文件扩展名无效」提示在 UI 上不可达，按计划记录偏差）
//   → 选中 e2e-test.kdbx（路径经剪贴板验证）→ 未输密码提交（F2 内联提示）
//   → 错误密码（F3 snackbar「Master Password 不正确…」、对话框保持打开）
//   → 伪 .kdbx（无效文件 snackbar「无效的 KeePass 2.0 数据库文件」）
//   → 正确密码单画布导入 → 跳转新画布 e2e-test → 断言节点与字段
//   （密码字段值经复制按钮 + 剪贴板验证为 kp-secret）
// → 多画布导入（按分组生成多个画布）：顶层画布「KeePass 根分组」含数据节点「KeePass 条目」
//   与两个画布数据节点「子分组 A」「子分组 B」→ 画布宇宙出现 4 个新画布
//   → 进入「子分组 A」画布验证其 entry 节点「子条目 A」
// → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 工具菜单项为 VListItem（role=listitem，菜单项标题为 #text），点击标题文本即可；
// - 导出/导入对话框单选的选中态映射为 states.checked；导出默认模式为 mask-values；
// - save 文件选择器默认文件名、导入对话框只读路径框的值都经 Ctrl+A/Ctrl+C 读剪贴板验证；
// - 文件选择器为虚拟滚动、open 模式双击文件即确认；被扩展名过滤的文件在开启
//   「显示全部文件」后以置灰（disabled）行展示且不可选中；
// - KeePass 导入的解密解析在前端完成：密码错误与文件无效是不同文案的 snackbar；
// - 成功 snackbar「数据库已导出」「KeePass2 数据导入完成」各出现多次，第二次等待前
//   先等旧消息消失，避免旧提示被误判为新提示；
// - F5（不含分组的文件在多画布模式返回 EmptyImportedCanvasList）按计划不构造；
// - 「导入中…」按钮态由 loading 驱动，持续时间取决于 Argon2 解密耗时，脚本不做瞬时断言；
// - 全部等待使用 lib 等待函数；应用生命周期由 withApp 包装，保证无论成败都经 /shutdown 收尾。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_011\case_011.js`

import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as api from "../lib/api.js";
import { withApp, prepareCaseOutput } from "../lib/app.js";
import { prepareDataDir } from "../lib/fixtures.js";
import { Report, finalizeReport } from "../lib/report.js";
import * as ui from "../lib/ui.js";
import { sleep, waitForCondition } from "../lib/util.js";

const CASE_DIR = path.dirname(fileURLToPath(import.meta.url));
const OUTPUT_DIR = path.join(CASE_DIR, "output");
const DATA_DIR = path.join(OUTPUT_DIR, "data");

/** fixture 数据库名称与密码（见 _fixtures\README.md） */
const DB_NAME = "zz-e2e-base";
const DB_PASSWORD = "e2e-password";

/** KeePass 测试资产的 Master Password */
const KDBX_PASSWORD = "e2e-kdbx-pass";
/** KeePass 测试资产文件名（单画布导入后新画布名取其 stem「e2e-test」） */
const KDBX_NAME = "e2e-test.kdbx";
/** 伪 .kdbx 文件名（内容为普通文本，用于「无效的 KeePass 2.0 数据库文件」路径） */
const INVALID_KDBX_NAME = "invalid-file.kdbx";
/** 普通文本文件名（用于 F4 扩展名过滤变体） */
const TXT_NAME = "not-a-database.txt";

/** 准备阶段：两个节点新增的字段名与值 */
const FIELD_A_NAME = "口令";
const FIELD_A_VALUE = "secret-export-123";
const FIELD_B_NAME = "备注字段";
const FIELD_B_VALUE = "note-export-value";

/** 三个导出产物文件名 */
const EXPORT_EXCLUDE = "export-exclude.md";
const EXPORT_MASK = "export-mask.md";
const EXPORT_INCLUDE = "export-include.md";

/** 后端导出的固定文案（frontend 传 locale=zh-CN） */
const EXPORT_CANVAS_LINE = "## 画布：root";
const EXPORT_NODE_A_LINE = "### 节点：账号 A";
const EXPORT_NODE_B_LINE = "### 节点：备注 B";
const EXPORT_RELATION_SECTION = "#### 关系";
const EXPORT_RELATION_LINE = "- 账号 A --[]--> 备注 B";

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

const report = new Report("case_011 导出与 KeePass 导入");

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
 * 在 UI 树中查找 role=radio 且名称/子树文本匹配的选项。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} label 单选标签文本。
 * @returns {object|null} 命中的单选节点；未命中时为 null。
 */
function findRadio(tree, label) {
  const radios = flattenNodes(tree).filter((node) => node.role === "radio");
  return (
    radios.find((node) => (node.name ?? "").trim() === label) ??
    radios.find((node) => ui.subtreeText(node).trim() === label) ??
    radios.find((node) => ui.subtreeText(node).includes(label)) ??
    null
  );
}

/**
 * 在 UI 树中查找包含指定标题的导入对话框。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} KeePass 导入对话框；未命中时为 null。
 */
function importDialog(tree) {
  return ui.findDialog(tree, "KeePass2数据导入");
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
 * 返回导出对话框内的按钮（对话框子树内查找以避免歧义）。
 * @param {string} buttonText 按钮文本。
 * @returns {Promise<object>} 命中的按钮节点。
 */
async function findExportDialogButton(buttonText) {
  return waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = ui.findDialog(tree, "导出数据库");
      return dialog ? ui.findButton(tree, buttonText, { root: dialog, exact: true }) : null;
    },
    { timeout: 8000, interval: 250, label: `导出对话框按钮「${buttonText}」` },
  );
}

/**
 * 返回导入对话框内的按钮（对话框子树内查找以避免歧义）。
 * @param {string} buttonText 按钮文本。
 * @returns {Promise<object>} 命中的按钮节点。
 */
async function findImportDialogButton(buttonText) {
  return waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = importDialog(tree);
      return dialog ? ui.findButton(tree, buttonText, { root: dialog, exact: true }) : null;
    },
    { timeout: 8000, interval: 250, label: `导入对话框按钮「${buttonText}」` },
  );
}

/**
 * 点击导入对话框内的输入框并输入文本（先全选覆盖已有内容）。
 * @param {string} nameFragment 输入框名称片段（如「Master Password」）。
 * @param {string} text 要输入的文本。
 * @returns {Promise<void>} 无返回值。
 */
async function typeIntoImportInput(nameFragment, text) {
  const input = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = importDialog(tree);
      return dialog ? ui.findEditable([dialog], nameFragment) : null;
    },
    { timeout: 6000, interval: 200, label: `导入对话框输入框「${nameFragment}」` },
  );
  await ui.clickNode(input);
  await api.postInput([api.keyClick(["Control", "a"]), api.typeText(text)]);
  await sleep(400);
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
 * 点击右下角「工具」按钮打开扳手菜单。
 * @returns {Promise<object>} 被点击的「工具」按钮。
 */
async function openToolMenu() {
  const button = await clickFinder(
    async () => {
      const tree = await api.uiTree();
      return (
        flattenNodes(tree)
          .filter((n) => n.role === "button" && ui.subtreeText(n).includes("工具"))
          .sort((a, b) => b.bounds.top - a.bounds.top)[0] ?? null
      );
    },
    { label: "「工具」按钮" },
  );
  await sleep(500);
  return button;
}

/**
 * 点击工具菜单中的菜单项（菜单项标题为 #text）。
 * @param {string} text 菜单项文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickToolMenuItem(text) {
  await clickFinder(async () => ui.findByText(await api.uiTree(), text, { exact: true }), {
    label: `工具菜单项「${text}」`,
  });
  await sleep(700);
}

/**
 * 打开「导出数据库」对话框并等其渲染完成。
 * @returns {Promise<void>} 无返回值。
 */
async function openExportDialog() {
  await openToolMenu();
  await clickToolMenuItem("导出数据库");
  await ui.waitForDialog("导出数据库", { timeout: 8000 });
  await sleep(800);
}

/**
 * 在导出对话框中选择导出模式并校验选中态。
 * @param {string} modeLabel 模式标签文本。
 * @returns {Promise<void>} 无返回值。
 */
async function selectExportMode(modeLabel) {
  const radio = await waitForCondition(
    async () => findRadio(await api.uiTree(), modeLabel),
    { timeout: 6000, interval: 200, label: `导出模式单选「${modeLabel}」` },
  );
  await ui.clickNode(radio);
  await waitForCondition(
    async () => {
      const node = findRadio(await api.uiTree(), modeLabel);
      return node?.states?.checked === true ? node : null;
    },
    { timeout: 6000, interval: 200, label: `导出模式「${modeLabel}」选中` },
  );
  await sleep(300);
}

/**
 * 等 save 文件选择器出现并返回其对话框节点。
 * @returns {Promise<object>} 文件选择器对话框节点。
 */
async function waitForPicker() {
  const picker = await waitForCondition(async () => dialogWithPathInput(await api.uiTree()), {
    timeout: 10000,
    interval: 250,
    label: "文件选择器",
  });
  await sleep(800);
  return picker;
}

/**
 * 在文件选择器路径框输入目录并回车导航。
 * @param {string} dir 目录绝对路径。
 * @returns {Promise<void>} 无返回值。
 */
async function navigatePickerTo(dir) {
  const picker = await waitForCondition(async () => dialogWithPathInput(await api.uiTree()), {
    timeout: 8000,
    interval: 250,
    label: "文件选择器（导航）",
  });
  await ui.clickNode(ui.findEditable([picker], "路径"));
  await api.postInput([api.keyClick(["Control", "a"]), api.typeText(dir), api.keyClick(["Enter"])]);
  await sleep(1200);
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
    const pathInput = ui.findEditable([picker], "路径");
    const rows = flattenNodes([picker])
      .filter(
        (n) =>
          n.tag === "#text" &&
          n.bounds.width > 0 &&
          n.bounds.top > pathInput.bounds.top + pathInput.bounds.height + 8,
      )
      .sort((a, b) => a.bounds.top - b.bounds.top);
    const last = rows[rows.length - 1];
    if (last) {
      await api.postInput([
        api.mouseMove(picker.bounds.left + picker.bounds.width / 2, last.bounds.top + last.bounds.height / 2),
        api.mouseScroll("down", 5),
      ]);
    }
    await sleep(350);
    if (Date.now() > deadline) {
      throw new Error(`scrollPickerToFile("${name}") timed out after ${timeout}ms`);
    }
  }
}

/**
 * 在 save 模式文件选择器中确认导出：可选先读取默认文件名，再填写路径与文件名并确认。
 * @param {string} fileName 目标文件名。
 * @param {object} [options] 可选参数。
 * @param {string} [options.expectDefaultName=null] 若提供，则在覆盖前用剪贴板校验默认文件名。
 * @returns {Promise<string|null>} 读取到的默认文件名（未读取时为 null）。
 */
async function confirmExportSave(fileName, { expectDefaultName = null } = {}) {
  const picker = await waitForPicker();
  let defaultName = null;
  if (expectDefaultName !== null) {
    report.check(ui.hasText([picker], "导出数据库"), "步骤 3 文件选择器标题为「导出数据库」");
    const nameInput = ui.findEditable([picker], "文件名");
    defaultName = nameInput ? await readEditableValue(nameInput) : null;
    await sleep(300);
  }
  await navigatePickerTo(OUTPUT_DIR);
  {
    const fresh = await waitForCondition(async () => dialogWithPathInput(await api.uiTree()), {
      timeout: 6000,
      interval: 250,
      label: "文件选择器（填文件名）",
    });
    await ui.clickNode(ui.findEditable([fresh], "文件名"));
    await api.postInput([api.keyClick(["Control", "a"]), api.typeText(fileName)]);
    await sleep(400);
  }
  {
    const tree = await api.uiTree();
    const fresh = dialogWithPathInput(tree);
    const confirm = ui.findButton(tree, "确认", { root: fresh, exact: true });
    await ui.clickNode(confirm);
  }
  await sleep(600);
  return defaultName;
}

/**
 * 执行一次导出：工具菜单 → 导出数据库 → 选模式 → 确认 → 文件选择器输出到 output。
 * @param {string} modeLabel 导出模式标签。
 * @param {string} fileName 输出文件名。
 * @param {object} [options] 可选参数。
 * @param {string|null} [options.expectDefaultName=null] 首次导出时用于校验默认文件名。
 * @returns {Promise<void>} 无返回值。
 */
async function exportOnce(modeLabel, fileName, { expectDefaultName = null } = {}) {
  await openExportDialog();
  await selectExportMode(modeLabel);
  await findExportDialogButton("确认").then((button) => ui.clickNode(button));
  const defaultName = await confirmExportSave(fileName, { expectDefaultName });
  if (expectDefaultName !== null) {
    report.check(
      defaultName === expectDefaultName,
      `步骤 3 导出文件选择器默认文件名为「${expectDefaultName}」（剪贴板验证）`,
      `clip=${defaultName}`,
    );
  }
  const snack = await ui.waitForTextStable("数据库已导出", { timeout: 12000 }).catch(() => null);
  report.check(snack !== null, `导出模式「${modeLabel}」提示「数据库已导出」`);
  const file = path.join(OUTPUT_DIR, fileName);
  const exists = fs.existsSync(file);
  report.check(exists, `导出模式「${modeLabel}」产物文件存在`, file);
  if (exists) {
    report.check(fs.readFileSync(file, "utf8").length > 0, `导出模式「${modeLabel}」产物文件非空`);
  }
  await snap(`export-${fileName.replace(/\.md$/, "")}`);
  await ui.waitForGone("数据库已导出", { timeout: 15000 }).catch(() => null);
}

/**
 * 点击导入对话框「选择文件」并在 open 模式文件选择器中选择指定 .kdbx 文件。
 * @param {string} fileName 目标文件名（位于 case 目录）。
 * @returns {Promise<void>} 无返回值。
 */
async function pickKeepassFile(fileName) {
  const button = await findImportDialogButton("选择文件");
  await ui.clickNode(button);
  await waitForPicker();
  await navigatePickerTo(CASE_DIR);
  {
    const hit = await scrollPickerToFile(fileName);
    await sleep(300);
    await ui.clickNode(hit, { count: 2 });
    await sleep(1000);
  }
  await waitForCondition(async () => dialogWithPathInput(await api.uiTree()) === null, {
    timeout: 8000,
    interval: 250,
    label: "文件选择器关闭",
  });
  const tree = await api.uiTree();
  const dialog = importDialog(tree);
  const pathInput = dialog ? ui.findEditable([dialog], "KeePass 2.0 数据库文件") : null;
  const value = pathInput ? await readEditableValue(pathInput) : null;
  report.check(
    value === path.join(CASE_DIR, fileName),
    `选择文件后路径框回填 ${fileName} 的绝对路径（剪贴板验证）`,
    `clip=${value}`,
  );
}

/**
 * 在导入对话框中选择导入方式并校验选中态。
 * @param {string} modeLabel 导入方式标签文本。
 * @returns {Promise<void>} 无返回值。
 */
async function selectImportMode(modeLabel) {
  const radio = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = importDialog(tree);
      return dialog ? findRadio([dialog], modeLabel) : null;
    },
    { timeout: 6000, interval: 200, label: `导入方式单选「${modeLabel}」` },
  );
  await ui.clickNode(radio);
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = importDialog(tree);
      const node = dialog ? findRadio([dialog], modeLabel) : null;
      return node?.states?.checked === true ? node : null;
    },
    { timeout: 6000, interval: 200, label: `导入方式「${modeLabel}」选中` },
  );
  await sleep(300);
}

/**
 * 返回字段名文本对应的字段行元素集合（字段名 + 类型下拉 + 值编辑器 + 复制按钮 + 图标）。
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
  await sleep(300);
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

  report.section("S1 准备字段（前置条件）");
  await openEditDialogByTitle("账号 A");
  await addNodeField(FIELD_A_NAME);
  {
    const tree = await api.uiTree();
    report.check(textNodes(tree, FIELD_A_NAME).length === 1, "准备 1 「账号 A」编辑对话框出现字段行「口令」");
    const row = fieldRow(tree, textNodes(tree, FIELD_A_NAME)[0]);
    report.check((row?.typeCombo?.text ?? "").trim() === "单行文本", "准备 1 字段类型为默认「单行文本」", row?.typeCombo?.text);
    report.check(row?.valueEditor != null, "准备 1 字段出现值编辑器");
    await ui.clickNode(row.valueEditor);
    await api.postInput([api.typeText(FIELD_A_VALUE)]);
    await sleep(500);
  }
  {
    const tree = await api.uiTree();
    const row = fieldRow(tree, textNodes(tree, FIELD_A_NAME)[0]);
    report.check(row?.copyButton?.states?.disabled === false, "准备 1 输入值后复制按钮可用（值已写入）");
  }
  await snap("s1a-field-a-filled");
  await clickEditDialogButton("确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(800);

  await openEditDialogByTitle("备注 B");
  await addNodeField(FIELD_B_NAME);
  {
    const tree = await api.uiTree();
    report.check(textNodes(tree, FIELD_B_NAME).length === 1, "准备 2 「备注 B」编辑对话框出现字段行「备注字段」");
    const row = fieldRow(tree, textNodes(tree, FIELD_B_NAME)[0]);
    report.check((row?.typeCombo?.text ?? "").trim() === "单行文本", "准备 2 字段类型为默认「单行文本」", row?.typeCombo?.text);
    report.check(row?.valueEditor != null, "准备 2 字段出现值编辑器");
    await ui.clickNode(row.valueEditor);
    await api.postInput([api.typeText(FIELD_B_VALUE)]);
    await sleep(500);
  }
  {
    const tree = await api.uiTree();
    const row = fieldRow(tree, textNodes(tree, FIELD_B_NAME)[0]);
    report.check(row?.copyButton?.states?.disabled === false, "准备 2 输入值后复制按钮可用（值已写入）");
  }
  await snap("s1b-field-b-filled");
  await clickEditDialogButton("确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 });
  await sleep(800);
  {
    const tree = await api.uiTree();
    report.check(ui.findDialog(tree, "编辑节点") === null, "准备 2 保存后编辑节点对话框关闭");
  }

  report.section("S2 工具菜单（步骤 1）");
  {
    await openToolMenu();
    const tree = await api.uiTree();
    report.check(
      textNodes(tree, "KeePass 2.0数据导入").length === 1,
      "步骤 1 工具菜单展开并含菜单项「KeePass 2.0数据导入」",
    );
    report.check(textNodes(tree, "导出数据库").length === 1, "步骤 1 工具菜单含菜单项「导出数据库」");
    await snap("s2-tool-menu");
  }
  await api.postInput([api.keyClick(["Escape"])]);
  await sleep(600);

  report.section("S3 导出三种模式（步骤 2-7）");
  await exportOnce("不包含字段", EXPORT_EXCLUDE, { expectDefaultName: "用户数据库.md" });
  await exportOnce("包含字段但不包含字段值", EXPORT_MASK);
  await exportOnce("包含字段且包含字段值", EXPORT_INCLUDE);

  // 步骤 2：对话框结构断言（重开一次导出对话框专门校验后取消）
  {
    await openExportDialog();
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, "导出数据库");
    report.check(dialog !== null, "步骤 2 打开对话框「导出数据库」");
    report.check(
      ui.hasText(
        [dialog],
        "导出的文件为明文 Markdown 文件，任何获得该文件的人都可以直接阅读其中的内容，请妥善保管。",
      ),
      "步骤 2 显示明文警告文案",
    );
    report.check(ui.hasText([dialog], "导出模式"), "步骤 2 存在「导出模式」分组");
    report.check(textNodes([dialog], "不包含字段").length === 1, "步骤 2 存在单选「不包含字段」");
    report.check(
      ui.hasText([dialog], "只导出画布、节点和关系，不导出任何字段信息"),
      "步骤 2 「不包含字段」说明文案正确",
    );
    report.check(textNodes([dialog], "包含字段但不包含字段值").length === 1, "步骤 2 存在单选「包含字段但不包含字段值」");
    report.check(
      ui.hasText([dialog], "导出字段名和字段类型，字段值将被打码"),
      "步骤 2 「包含字段但不包含字段值」说明文案正确",
    );
    report.check(textNodes([dialog], "包含字段且包含字段值").length === 1, "步骤 2 存在单选「包含字段且包含字段值」");
    report.check(ui.hasText([dialog], "完整导出所有字段，字段值为明文"), "步骤 2 「包含字段且包含字段值」说明文案正确");
    const defaultRadio = findRadio([dialog], "包含字段但不包含字段值");
    report.check(defaultRadio?.states?.checked === true, "步骤 2 默认选中「包含字段但不包含字段值」");
    await snap("s3-export-dialog-detail");
    await findExportDialogButton("取消").then((button) => ui.clickNode(button));
    await ui.waitForDialogGone("导出数据库", { timeout: 8000 });
    await sleep(600);
  }

  // 步骤 7：读取三个导出文件做字段级断言
  {
    const read = (name) => fs.readFileSync(path.join(OUTPUT_DIR, name), "utf8");
    const exclude = read(EXPORT_EXCLUDE);
    const mask = read(EXPORT_MASK);
    const include = read(EXPORT_INCLUDE);

    for (const [name, content] of [
      [EXPORT_EXCLUDE, exclude],
      [EXPORT_MASK, mask],
      [EXPORT_INCLUDE, include],
    ]) {
      report.check(
        content.includes(EXPORT_CANVAS_LINE) &&
          content.includes(EXPORT_NODE_A_LINE) &&
          content.includes(EXPORT_NODE_B_LINE),
        `步骤 7 ${name} 含画布「root」与节点「账号 A」「备注 B」`,
      );
      report.check(
        content.includes(EXPORT_RELATION_SECTION) && content.includes(EXPORT_RELATION_LINE),
        `步骤 7 ${name} 含关系小节与边「账号 A --[]--> 备注 B」`,
      );
    }

    report.check(
      !exclude.includes("| 字段名 |") && !exclude.includes(FIELD_A_NAME),
      "步骤 7 export-exclude.md 不含字段表格与字段名「口令」",
    );
    report.check(
      mask.includes("| 字段名 | 值 |") && mask.includes(FIELD_A_NAME) && mask.includes(FIELD_B_NAME),
      "步骤 7 export-mask.md 含字段名「口令」「备注字段」",
    );
    report.check(
      !mask.includes(FIELD_A_VALUE) && !mask.includes(FIELD_B_VALUE),
      "步骤 7 export-mask.md 不含字段明文值",
    );
    report.check(
      /sec\*+123/.test(mask) && /not\*+lue/.test(mask),
      "步骤 7 export-mask.md 字段值以打码形式出现（保留首尾字符的掩码）",
    );
    report.check(
      include.includes(FIELD_A_VALUE) && include.includes(FIELD_B_VALUE),
      "步骤 7 export-include.md 含字段明文值 secret-export-123 与 note-export-value",
    );
    await snap("s3b-export-files-inspected");
  }

  report.section("S4 KeePass 导入对话框与失败路径（步骤 8-13、F1-F4）");
  await openToolMenu();
  await clickToolMenuItem("KeePass 2.0数据导入");
  await ui.waitForDialog("KeePass2数据导入", { timeout: 8000 });
  await sleep(800);
  {
    const tree = await api.uiTree();
    const dialog = importDialog(tree);
    report.check(dialog !== null, "步骤 8 打开对话框「KeePass2数据导入」");
    report.check(
      dialog !== null && ui.findEditable([dialog], "KeePass 2.0 数据库文件") !== null,
      "步骤 8 文件路径框 label 为「KeePass 2.0 数据库文件」",
    );
    report.check(
      dialog !== null && ui.findButton(tree, "选择文件", { root: dialog, exact: true }) !== null,
      "步骤 8 存在「选择文件」按钮",
    );
    report.check(
      dialog !== null && ui.findEditable([dialog], "Master Password") !== null,
      "步骤 8 存在 Master Password 输入框",
    );
    report.check(dialog !== null && ui.hasText([dialog], "导入方式"), "步骤 8 存在「导入方式」分组");
    report.check(
      dialog !== null &&
        textNodes([dialog], "导入单个画布").length === 1 &&
        textNodes([dialog], "按分组生成多个画布").length === 1,
      "步骤 8 两个导入方式单选文案正确",
    );
    report.check(
      dialog !== null && findRadio([dialog], "导入单个画布")?.states?.checked === true,
      "步骤 8 默认选中「导入单个画布」",
    );
    await snap("s4-keepass-dialog");
  }

  // F1：不选文件直接提交
  {
    const button = await findImportDialogButton("确认");
    await ui.clickNode(button);
    await waitForCondition(async () => ui.hasText(await api.uiTree(), "请选择 KeePass 2.0 数据库文件"), {
      timeout: 6000,
      interval: 200,
      label: "F1 路径内联提示",
    });
    const tree = await api.uiTree();
    report.check(ui.hasText(tree, "请选择 KeePass 2.0 数据库文件"), "F1 未选文件提交显示内联提示「请选择 KeePass 2.0 数据库文件」");
    report.check(importDialog(tree) !== null, "F1 校验失败后对话框保持打开，未发起导入");
    await snap("s4b-f1-file-required");
  }

  // 步骤 10 + F4：文件选择器（扩展名过滤与置灰行）
  {
    await findImportDialogButton("选择文件").then((button) => ui.clickNode(button));
    await waitForPicker();
    const tree = await api.uiTree();
    report.check(dialogWithPathInput(tree) !== null, "步骤 10 打开文件选择器（open 模式）");
    report.check(ui.hasText(tree, "选择文件"), "步骤 10 文件选择器标题为「选择文件」");
    report.check(
      flattenNodes(tree).some((n) => n.role === "checkbox" && n.name === "显示全部文件"),
      "步骤 10 文件选择器带扩展名过滤开关「显示全部文件」",
    );
    await snap("s4c-f4-picker");

    await navigatePickerTo(CASE_DIR);
    {
      const toggle = await waitForCondition(
        async () => {
          const t = await api.uiTree();
          const picker = dialogWithPathInput(t);
          return picker
            ? flattenNodes([picker]).find((n) => n.role === "checkbox" && n.name === "显示全部文件") ?? null
            : null;
        },
        { timeout: 6000, interval: 200, label: "「显示全部文件」开关" },
      );
      await ui.clickNode(toggle);
      await sleep(600);
    }
    const txtRow = await waitForCondition(
      async () => {
        const t = await api.uiTree();
        const picker = dialogWithPathInput(t);
        return picker ? ui.findByText([picker], TXT_NAME, { exact: true }) : null;
      },
      { timeout: 6000, interval: 250, label: `文件行「${TXT_NAME}」` },
    );
    report.check(txtRow !== null, `F4 开启「显示全部文件」后列出被过滤的 ${TXT_NAME}`);
    {
      const tree = await api.uiTree();
      const picker = dialogWithPathInput(tree);
      const confirm = ui.findButton(tree, "确认", { root: picker, exact: true });
      report.check(confirm?.states?.disabled === true, "F4 未选中任何文件时「确认」按钮禁用");
      await ui.clickNode(txtRow);
      await sleep(500);
      const t2 = await api.uiTree();
      const picker2 = dialogWithPathInput(t2);
      const confirm2 = ui.findButton(t2, "确认", { root: picker2, exact: true });
      report.check(confirm2?.states?.disabled === true, "F4 单击扩展名被过滤的文件行不产生选中（确认键仍禁用）");
      await ui.clickNode(txtRow, { count: 2 });
      await sleep(700);
      const t3 = await api.uiTree();
      report.check(dialogWithPathInput(t3) !== null, "F4 双击扩展名被过滤的文件行不关闭文件选择器（不可选中）");
      console.log(
        "  (note) F4 偏差：文件选择器将非 kdbx 文件置灰且不可选中，UI 上无法构造「文件扩展名无效」请求，按计划记录偏差并跳过",
      );
      await snap("s4d-f4-txt-disabled");
    }
    // 关闭文件选择器，回到导入对话框
    {
      const tree = await api.uiTree();
      const picker = dialogWithPathInput(tree);
      await ui.clickNode(ui.findButton(tree, "取消", { root: picker, exact: true }));
      await waitForCondition(async () => dialogWithPathInput(await api.uiTree()) === null, {
        timeout: 8000,
        interval: 250,
        label: "文件选择器关闭",
      });
      await sleep(700);
      report.check(importDialog(await api.uiTree()) !== null, "F4 关闭文件选择器后导入对话框仍在");
    }
  }

  // 步骤 11：选择 e2e-test.kdbx 并回填路径
  await pickKeepassFile(KDBX_NAME);
  await snap("s4e-kdbx-picked");

  // F2：未输密码提交
  {
    const button = await findImportDialogButton("确认");
    await ui.clickNode(button);
    await waitForCondition(async () => ui.hasText(await api.uiTree(), "请输入 Master Password"), {
      timeout: 6000,
      interval: 200,
      label: "F2 密码内联提示",
    });
    const tree = await api.uiTree();
    report.check(ui.hasText(tree, "请输入 Master Password"), "F2 未输密码提交显示内联提示「请输入 Master Password」");
    report.check(importDialog(tree) !== null, "F2 校验失败后对话框保持打开，未发起导入");
    await snap("s4f-f2-password-required");
  }

  // F3：错误密码
  {
    await typeIntoImportInput("Master Password", "wrong-password");
    const button = await findImportDialogButton("确认");
    await ui.clickNode(button);
    const snack = await ui.waitForTextStable("Master Password 不正确，无法解密所选数据库", { timeout: 15000 }).catch(() => null);
    report.check(snack !== null, "F3 错误密码提示「Master Password 不正确，无法解密所选数据库」");
    await sleep(500);
    const tree = await api.uiTree();
    report.check(importDialog(tree) !== null, "F3 解密失败后对话框保持打开可重试");
    await snap("s4g-f3-wrong-password");
    await ui.waitForGone("Master Password 不正确，无法解密所选数据库", { timeout: 15000 }).catch(() => null);
  }

  // 附加失败路径：伪 .kdbx（文件无效）
  {
    await pickKeepassFile(INVALID_KDBX_NAME);
    await typeIntoImportInput("Master Password", KDBX_PASSWORD);
    const button = await findImportDialogButton("确认");
    await ui.clickNode(button);
    const snack = await ui.waitForTextStable("无效的 KeePass 2.0 数据库文件", { timeout: 15000 }).catch(() => null);
    report.check(snack !== null, "附加 F6 伪 .kdbx 提示「无效的 KeePass 2.0 数据库文件」");
    await sleep(500);
    const tree = await api.uiTree();
    report.check(importDialog(tree) !== null, "附加 F6 文件无效时对话框保持打开");
    await snap("s4h-f6-invalid-file");
    await ui.waitForGone("无效的 KeePass 2.0 数据库文件", { timeout: 15000 }).catch(() => null);
  }

  report.section("S5 单画布导入与结果断言（步骤 14-15）");
  await pickKeepassFile(KDBX_NAME);
  await typeIntoImportInput("Master Password", KDBX_PASSWORD);
  {
    const button = await findImportDialogButton("确认");
    report.check(button.states?.disabled !== true, "步骤 14 导入前「确认」按钮可用");
    await ui.clickNode(button);
  }
  const importSnack = await ui.waitForTextStable("KeePass2 数据导入完成", { timeout: 20000 }).catch(() => null);
  report.check(importSnack !== null, "步骤 14 单画布导入提示「KeePass2 数据导入完成」");
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return importDialog(tree) === null && textNodes(tree, "KeePass 条目").length > 0 ? tree : null;
    },
    { timeout: 15000, interval: 300, label: "对话框关闭并跳转新画布" },
  );
  await sleep(1200);
  {
    const tree = await api.uiTree();
    report.check(importDialog(tree) === null, "步骤 14 导入成功后对话框关闭");
    report.check(textNodes(tree, "e2e-test").length === 1, "步骤 14 路由跳转新画布（面包屑出现画布名「e2e-test」）");
    report.check(textNodes(tree, "KeePass 根分组").length === 1, "步骤 15 新画布出现表示数据库本身的根节点「KeePass 根分组」");
    report.check(textNodes(tree, "KeePass 条目").length === 1, "步骤 15 新画布出现根 group 的 entry 节点「KeePass 条目」");
    report.check(textNodes(tree, "子分组 A").length === 1, "步骤 15 新画布出现子分组节点「子分组 A」");
    report.check(textNodes(tree, "子条目 A").length === 1, "步骤 15 新画布出现子分组下的 entry 节点「子条目 A」");
    await snap("s5-single-canvas-imported");
  }

  // 步骤 15：双击打开导入节点，断言字段区与密码值
  await openEditDialogByTitle("KeePass 条目");
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, "编辑节点");
    const passwordText = dialog ? textNodes([dialog], "密码")[0] : undefined;
    const passwordRow = fieldRow(tree, passwordText);
    report.check(passwordRow?.typeCombo != null, "步骤 15 编辑对话框含密码字段行（字段名「密码」）");
    report.check((passwordRow?.typeCombo?.text ?? "").trim() === "密码", "步骤 15 密码字段类型为「密码」", passwordRow?.typeCombo?.text);
    report.check(passwordRow?.copyButton?.states?.disabled === false, "步骤 15 密码字段有值（复制按钮可用）");
    report.check(passwordRow?.icons?.length === 2, "步骤 15 密码字段带显示/隐藏与生成器图标", `icons=${passwordRow?.icons?.length}`);
    const urlRow = fieldRow(tree, dialog ? textNodes([dialog], "访问链接")[0] : undefined);
    const notesRow = fieldRow(tree, dialog ? textNodes([dialog], "备注")[0] : undefined);
    report.check((urlRow?.typeCombo?.text ?? "").trim() === "网址", "步骤 15 访问链接字段类型为「网址」", urlRow?.typeCombo?.text);
    report.check((notesRow?.typeCombo?.text ?? "").trim() === "多行文本", "步骤 15 备注字段类型为「多行文本」", notesRow?.typeCombo?.text);
    if (passwordRow?.copyButton) {
      await ui.clickNode(passwordRow.copyButton);
      await sleep(500);
      const copied = readClipboard();
      report.check(copied === "kp-secret", "步骤 15 复制密码字段值经剪贴板验证为 kp-secret", `clip=${copied}`);
    } else {
      report.check(false, "步骤 15 复制密码字段值经剪贴板验证为 kp-secret", "copy button missing");
    }
    await snap("s5b-imported-node-fields");
  }
  await clickEditDialogButton("取消");
  await ui.waitForDialogGone("编辑节点", { timeout: 8000 }).catch(() => null);
  await sleep(700);

  report.section("S6 多画布导入与结果断言（步骤 16-17）");
  await openToolMenu();
  await clickToolMenuItem("KeePass 2.0数据导入");
  await ui.waitForDialog("KeePass2数据导入", { timeout: 8000 });
  await sleep(700);
  await pickKeepassFile(KDBX_NAME);
  await selectImportMode("按分组生成多个画布");
  await typeIntoImportInput("Master Password", KDBX_PASSWORD);
  await snap("s6-import-multi-ready");
  {
    const button = await findImportDialogButton("确认");
    await ui.clickNode(button);
  }
  const multiSnack = await ui.waitForTextStable("KeePass2 数据导入完成", { timeout: 20000 }).catch(() => null);
  report.check(multiSnack !== null, "步骤 16 多画布导入提示「KeePass2 数据导入完成」");
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      return importDialog(tree) === null && textNodes(tree, "KeePass 根分组").length === 1 ? tree : null;
    },
    { timeout: 15000, interval: 300, label: "跳转顶层新画布「KeePass 根分组」" },
  );
  await sleep(1200);
  {
    const tree = await api.uiTree();
    report.check(importDialog(tree) === null, "步骤 16 多画布导入成功后对话框关闭");
    report.check(textNodes(tree, "KeePass 根分组").length === 1, "步骤 16 跳转顶层新画布（面包屑出现「KeePass 根分组」）");
    report.check(textNodes(tree, "KeePass 条目").length === 1, "步骤 17 顶层画布含根 group 的数据节点「KeePass 条目」");
    report.check(textNodes(tree, "子分组 A").length === 1, "步骤 17 顶层画布出现画布数据节点「子分组 A」");
    report.check(textNodes(tree, "子分组 B").length === 1, "步骤 17 顶层画布出现画布数据节点「子分组 B」");
    report.check(textNodes(tree, "子条目 A").length === 0, "步骤 17 子分组内容不在顶层画布（子条目 A 未出现）");
    await snap("s6b-multi-canvas-top");
  }

  // 步骤 17：画布宇宙与子画布
  await clickFinder(
    async () =>
      flattenNodes(await api.uiTree()).find(
        (n) => n.role === "link" && (n.text ?? "").trim() === "画布宇宙",
      ) ?? null,
    { label: "面包屑链接「画布宇宙」" },
  );
  await waitForCondition(async () => ui.hasText(await api.uiTree(), "根画布"), {
    timeout: 10000,
    interval: 250,
    label: "画布宇宙页面",
  });
  await sleep(1200);
  {
    const tree = await api.uiTree();
    report.check(textNodes(tree, "根画布").length === 1, "步骤 17 画布宇宙显示内置根画布");
    report.check(textNodes(tree, "e2e-test").length === 1, "步骤 17 画布宇宙出现单画布导入的新画布「e2e-test」");
    report.check(textNodes(tree, "KeePass 根分组").length === 1, "步骤 17 画布宇宙出现多画布导入的顶层画布「KeePass 根分组」");
    report.check(textNodes(tree, "子分组 A").length === 1, "步骤 17 画布宇宙出现子画布「子分组 A」");
    report.check(textNodes(tree, "子分组 B").length === 1, "步骤 17 画布宇宙出现子画布「子分组 B」");
    await snap("s6c-canvas-universe");
  }
  {
    const subCanvasText = await waitForCondition(
      async () => textNodes(await api.uiTree(), "子分组 A")[0] ?? null,
      { timeout: 6000, interval: 200, label: "画布项「子分组 A」" },
    );
    await ui.clickNode(subCanvasText, { count: 2 });
  }
  await waitForCondition(async () => ui.hasText(await api.uiTree(), "子条目 A"), {
    timeout: 10000,
    interval: 250,
    label: "子画布内容「子条目 A」",
  });
  await sleep(1000);
  {
    const tree = await api.uiTree();
    report.check(textNodes(tree, "子条目 A").length === 1, "步骤 17 进入「子分组 A」画布可见其 entry 节点「子条目 A」");
    report.check(textNodes(tree, "KeePass 条目").length === 0, "步骤 17 子画布不含顶层画布的数据节点「KeePass 条目」");
    report.check(textNodes(tree, "子分组 A").length >= 1, "步骤 17 面包屑显示当前画布「子分组 A」");
    await snap("s6d-sub-canvas");
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
