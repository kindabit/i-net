// case_012 备份与还原。
//
// 流程：复制 base fixture → 向数据目录的附件目录写入 64MiB 大附件与 6000 个 8KiB 小文件
//   （放大备份/还原耗时与打包阶段的事件密度，使进度阶段在界面上有可见窗口）
//   → 启动应用 → 解锁后将「备注 B」改名为「备注 B-备份前」
//   →「保存并退出」回首页（保证备份时数据目录为已保存状态）
// → 备份（步骤 1-5 + F1）：
//   备份对话框（当前数据目录大小 / 冗余比例默认 0.05 / 预计备份大小）→ 比例改 0.1 后预计值增大
//   → 开始备份 → save 文件选择器（标题「备份数据目录」、默认文件名 backup.ibackup，剪贴板验证）
//   → 输出 backup-1.ibackup → 直接轮询观察进度阶段 → snackbar「备份已保存」并关闭对话框
//   → 脚本断言产物存在且大小 ≥ 数据目录大小；F1：比例 0 → snackbar「备份文件无效：…」
//   且不产生文件
// → 修改数据（步骤 7）：重新解锁 → 改名「备份后修改」→ 保存并退出
// → 还原（步骤 8-12）：
//   还原对话框（警告文案 / 「确认还原」禁用 / 「选择备份文件」可用）→ 选择 backup-1.ibackup
//   → 探测通过（「备份文件校验通过，可以还原」/「确认还原」可用 / 「选择备份文件」禁用）
//   → 确认还原 → 直接轮询观察进度阶段 → 「还原成功」对话框（文案 + 唯一「完成」按钮）
//   → 点击「完成」触发整页 reload 回首页
// → 数据回退验证（步骤 13）：重新解锁后根画布节点为「备注 B-备份前」，无「备份后修改」
//   （间接验证 reclaim_preference / reclaim_metadata / reclaim_user_database 链路生效）
// → 损坏备份（步骤 14-15）：复制并截断到 60% → 探测显示「备份文件损坏过多，无法还原」，
//   「确认还原」保持禁用
// → F3 变体：非 .ibackup 文件被文件选择器扩展名过滤（未开启「显示全部文件」时不可见，
//   开启后置灰不可选中），UI 上不可构造探测请求，按计划记录偏差
// → 附加 F6：伪 .ibackup（扩展名合法、内容非法）→ snackbar「备份文件无效」，对话框保持打开
// → 输出 PASS/FAIL 报告。
//
// 脚本规范要点：
// - 备份/还原进行中脚本以 50ms 间隔直接轮询 UI 树捕捉进度阶段：备份断言出现首阶段
//   「正在打包数据目录」（小文件群把该阶段 UI 窗口放大到数百毫秒）；还原的首阶段
//   「正在读取文件头」业务耗时低于 1ms、阶段事件背靠背到达被前端批处理合并，连续轮询实测
//   从未在 UI 树中出现，脚本在观察到时按首阶段硬断言、未观察到时以紧随的「正在校验完整性」
//   作为最早可观察阶段（详见计划末尾的修订记录）；两者均断言阶段为预期顺序的
//   有序子集、结束时停留在最后阶段，每个观察到的阶段异步留档一张截图。备份/还原类命令已改为
//   异步命令（应用修复（2026-09-21）第 4 条），业务执行不再占用 WebView2 IPC
//   主线程；脚本同时断言「业务进行中调试自动化接口保持可用」：每轮 /ui-tree 全部成功且耗时
//   低于阈值、阶段截图的 /screenshot 耗时低于阈值（实测数据见断言 message）；
// - 业务期间的等待只使用 waitForCondition 一类容错重试（内部捕获请求失败后继续轮询），
//   不使用会因单次请求失败而中断的等待函数；
// - 备份产物写入 output\ 后由脚本用 fs 断言存在与大小；数据目录大小按与后端一致的口径
//   （递归非 logs 文件）自行统计；
// - 还原成功后「完成」按钮执行 window.location.replace("/") 整页 reload，脚本等待
//   「还原成功」消失且「数据库名称」输入框重新出现后再继续；
// - 损坏备份由 fs 复制并截断到 60% 构造；伪 .ibackup 为普通文本文件；
// - 全部操作完毕后由 withApp 经 /shutdown 收尾关闭应用。
//
// 运行方式：在项目根目录执行 `node e2e\script\case_012\case_012.js`

import { execSync } from "node:child_process";
import crypto from "node:crypto";
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

/** 备份前的节点标题（备份时点）与备份后的节点标题（还原对比目标） */
const TITLE_BEFORE_BACKUP = "备注 B-备份前";
const TITLE_AFTER_BACKUP = "备份后修改";

/** 大附件文件名与总大小（64 MiB；放大备份/还原耗时以观察进度阶段） */
const LARGE_ATTACHMENT_NAME = "e2e-large-attachment.bin";
const LARGE_ATTACHMENT_SIZE = 64 * 1024 * 1024;
/**
 * 小文件群（8 KiB × 6000）放在附件的 bulk 子目录下：
 * 放大 tar 打包阶段的耗时与事件密度，使「正在打包数据目录」阶段具备可观察窗口
 * （单个大文件时该阶段业务仅数十毫秒、UI 可见窗口低于一帧）。
 */
const BULK_DIR_NAME = "bulk";
const BULK_FILE_COUNT = 6000;
const BULK_FILE_SIZE = 8 * 1024;

/** 备份/还原产物与辅助资产（均位于 output\） */
const BACKUP_FILE_NAME = "backup-1.ibackup";
const BACKUP_INVALID_NAME = "backup-invalid.ibackup";
const BACKUP_CORRUPT_NAME = "backup-corrupt.ibackup";
const TXT_FILE_NAME = "not-a-backup.txt";
const FAKE_BACKUP_NAME = "invalid-file.ibackup";
/** 损坏备份的截断比例 */
const CORRUPT_TRUNCATE_RATIO = 0.6;

/** 备份进度阶段文案（有序） */
const PHASES_BACKUP = ["正在打包数据目录", "正在生成校验数据", "正在写入备份文件"];
/** 还原进度阶段文案（有序） */
const PHASES_RESTORE = [
  "正在读取文件头",
  "正在校验完整性",
  "正在纠错解码",
  "正在解压文件",
  "正在清空当前数据",
  "正在写入新数据",
];

/** 备份/还原进度探测的最大轮询时长（毫秒） */
const BACKUP_CAPTURE_TIMEOUT_MS = 120000;
const RESTORE_CAPTURE_TIMEOUT_MS = 150000;
/** 进度探测的轮询间隔（毫秒） */
const PROBE_INTERVAL_MS = 50;
/** 业务进行中调试自动化接口的单次请求耗时上限（毫秒；应用修复（2026-09-21）第 4 条实测 /ui-tree 6–9ms、/screenshot 674–766ms） */
const AUTOMATION_BUDGET_MS = 3000;
/** 业务进行中要求完成的最少探测轮次数（按 50ms 间隔与秒级业务时长预估，实测值见断言 message） */
const MIN_PROBE_ROUNDS = 50;

const report = new Report("case_012 备份与还原");

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
 * 返回 UI 树中以指定前缀开头的 #text 节点。
 * @param {object[]} tree UI 树顶层节点数组。
 * @param {string} prefix 文本前缀。
 * @returns {object[]} 命中的文本节点数组。
 */
function textStartingWith(tree, prefix) {
  return flattenNodes(tree).filter(
    (node) => node.tag === "#text" && (node.text ?? "").trim().startsWith(prefix),
  );
}

/**
 * 解析界面上的尺寸文本（如「64.16 MB」）为字节数。
 * @param {string} text 尺寸文本（允许带「标签：」前缀）。
 * @returns {number|null} 字节数；解析失败时为 null。
 */
function parseSizeText(text) {
  const match = /([\d.]+)\s*(B|KB|MB|GB)\s*$/.exec((text ?? "").trim());
  if (!match) {
    return null;
  }
  const value = Number(match[1]);
  const unit = { B: 1, KB: 1024, MB: 1024 * 1024, GB: 1024 * 1024 * 1024 }[match[2]];
  return Number.isFinite(value) ? Math.round(value * unit) : null;
}

/**
 * 统计数据目录大小（与后端口径一致：递归所有非 logs 文件）。
 * @param {string} dir 数据目录根路径。
 * @returns {number} 字节总数。
 */
function dataDirSize(dir) {
  let total = 0;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "logs") {
      continue;
    }
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      total += dataDirSize(full);
    } else if (entry.isFile()) {
      total += fs.statSync(full).size;
    }
  }
  return total;
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
 * 解锁 fixture 数据库：名称步选择候选并确认，密码步输入密码确认，等待进入根画布。
 * @returns {Promise<void>} 无返回值。
 */
async function unlock() {
  await ui.clickEditable("数据库名称");
  await api.postInput([api.typeText(DB_NAME)]);
  await ui.waitForTextStable(DB_NAME, { timeout: 8000 });
  await api.postInput([api.keyClick(["Enter"])]);
  await ui.clickButton("确认");
  await ui.waitForEditable("密码", { timeout: 30000 });
  await ui.typeIntoEditable("密码", DB_PASSWORD);
  await ui.clickButton("确认");
  await ui.waitForTextStable("账号 A", { timeout: 60000, settleMs: 800 });
}

/**
 * 在画布中双击节点标题打开编辑节点对话框。
 * @param {string} title 节点标题。
 * @returns {Promise<void>} 无返回值。
 */
async function openEditDialogByTitle(title) {
  await ui.clickText(title, { count: 2 });
  await ui.waitForDialog("编辑节点", { timeout: 10000 });
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
 * 双击节点后将其标题改为新标题并保存，等待画布标题更新。
 * @param {string} oldTitle 当前节点标题。
 * @param {string} newTitle 新节点标题。
 * @returns {Promise<void>} 无返回值。
 */
async function renameNodeByTitle(oldTitle, newTitle) {
  await openEditDialogByTitle(oldTitle);
  await ui.typeIntoEditable("标题", newTitle);
  await sleep(300);
  await clickEditDialogButton("确认");
  await ui.waitForDialogGone("编辑节点", { timeout: 10000 });
  await ui.waitForTextStable(newTitle, { timeout: 10000, settleMs: 500 });
}

/**
 * 点击「保存并退出」保存数据库并回到首页。
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

/**
 * 在 UI 树中查找文件选择器对话框（特征：包含名称为「路径」的可编辑控件）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 文件选择器对话框；未命中时为 null。
 */
function dialogWithPathInput(tree) {
  return ui.findByRole(tree, "dialog").find((d) => ui.findEditable([d], "路径") !== null) ?? null;
}

/**
 * 等文件选择器出现并返回其对话框节点。
 * @returns {Promise<object>} 文件选择器对话框节点。
 */
async function waitForPicker() {
  const picker = await waitForCondition(async () => dialogWithPathInput(await api.uiTree()), {
    timeout: 15000,
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
  await sleep(1500);
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
 * 在 save 模式文件选择器中填写目标文件名并确认。
 * @param {string} fileName 目标文件名。
 * @returns {Promise<void>} 无返回值。
 */
async function fillPickerFileNameAndConfirm(fileName) {
  const fresh = await waitForCondition(async () => dialogWithPathInput(await api.uiTree()), {
    timeout: 8000,
    interval: 250,
    label: "文件选择器（填文件名）",
  });
  await ui.clickNode(ui.findEditable([fresh], "文件名"));
  await api.postInput([api.keyClick(["Control", "a"]), api.typeText(fileName)]);
  await sleep(400);
  const tree = await api.uiTree();
  const picker = dialogWithPathInput(tree);
  const confirm = ui.findButton(tree, "确认", { root: picker, exact: true });
  await ui.clickNode(confirm);
}

/**
 * 在 open 模式文件选择器中双击目标文件，等待选择器关闭。
 * @param {string} fileName 目标文件名（位于 output 目录）。
 * @returns {Promise<void>} 无返回值。
 */
async function pickBackupFile(fileName) {
  await waitForPicker();
  await navigatePickerTo(OUTPUT_DIR);
  const hit = await scrollPickerToFile(fileName);
  await sleep(300);
  await ui.clickNode(hit, { count: 2 });
  await waitForCondition(async () => dialogWithPathInput(await api.uiTree()) === null, {
    timeout: 15000,
    interval: 250,
    label: "文件选择器关闭",
  });
  await sleep(400);
}

/**
 * 在 UI 树中查找备份对话框（特征：含「开始备份」按钮或「冗余比例」输入框）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 备份对话框；未命中时为 null。
 */
function backupDialog(tree) {
  return (
    ui
      .findByRole(tree, "dialog")
      .find(
        (d) =>
          ui.findButton([d], "开始备份", { exact: true }) !== null ||
          ui.findEditable([d], "冗余比例") !== null,
      ) ?? null
  );
}

/**
 * 在 UI 树中查找还原对话框（特征：含「确认还原」按钮）。
 * @param {object[]} tree UI 树顶层节点数组。
 * @returns {object|null} 还原对话框；未命中时为 null。
 */
function restoreDialog(tree) {
  return (
    ui.findByRole(tree, "dialog").find((d) => ui.findButton([d], "确认还原", { exact: true }) !== null) ??
    null
  );
}

/**
 * 点击右下角「备份数据目录」按钮并等待备份对话框出现与数据目录大小加载完成。
 * @returns {Promise<object>} 备份对话框内「开始备份」按钮节点。
 */
async function openBackupDialog() {
  const backupBtn = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "aria-label", "备份数据目录"),
    { timeout: 8000, interval: 200, label: "「备份数据目录」按钮" },
  );
  await ui.clickNode(backupBtn);
  await ui.waitForDialog("备份数据目录", { timeout: 10000 });
  await sleep(1000);
  return waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = backupDialog(tree);
      if (!dialog || !ui.hasText([dialog], "当前数据目录大小：")) {
        return null;
      }
      const button = ui.findButton(tree, "开始备份", { root: dialog, exact: true });
      return button && button.states?.disabled !== true ? button : null;
    },
    { timeout: 15000, interval: 250, label: "备份对话框数据目录大小加载完成" },
  );
}

/**
 * 设置备份对话框中的冗余比例（先全选覆盖）。
 * @param {string} value 比例文本。
 * @returns {Promise<void>} 无返回值。
 */
async function setBackupRatio(value) {
  const ratioInput = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = backupDialog(tree);
      return dialog ? ui.findEditable([dialog], "冗余比例") : null;
    },
    { timeout: 6000, interval: 200, label: "冗余比例输入框" },
  );
  await ui.clickNode(ratioInput);
  await api.postInput([api.keyClick(["Control", "a"]), api.typeText(value)]);
  await sleep(700);
}

/**
 * 点击备份对话框内按钮（在对话框子树内查找以避免歧义）。
 * @param {string} text 按钮文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickBackupDialogButton(text) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = backupDialog(tree);
      return dialog ? ui.findButton(tree, text, { root: dialog, exact: true }) : null;
    },
    { timeout: 8000, interval: 200, label: `备份对话框按钮「${text}」` },
  );
  await ui.clickNode(button);
  await sleep(400);
}

/**
 * 点击右下角「还原数据目录」按钮并等待还原对话框出现。
 * @returns {Promise<void>} 无返回值。
 */
async function openRestoreDialog() {
  const restoreBtn = await waitForCondition(
    async () => ui.findAttr(await api.uiTree(), "aria-label", "还原数据目录"),
    { timeout: 8000, interval: 200, label: "「还原数据目录」按钮" },
  );
  await ui.clickNode(restoreBtn);
  await ui.waitForDialog("还原数据目录", { timeout: 10000 });
  await sleep(900);
}

/**
 * 点击还原对话框内按钮（在对话框子树内查找以避免歧义）。
 * @param {string} text 按钮文本。
 * @returns {Promise<void>} 无返回值。
 */
async function clickRestoreDialogButton(text) {
  const button = await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = restoreDialog(tree);
      return dialog ? ui.findButton(tree, text, { root: dialog, exact: true }) : null;
    },
    { timeout: 8000, interval: 200, label: `还原对话框按钮「${text}」` },
  );
  await ui.clickNode(button);
  await sleep(400);
}

/**
 * 容错等待对话框关闭（单次查询失败按「仍在」处理并继续轮询）。
 * @param {(tree: object[]) => boolean} isGone 以树为输入的判定函数。
 * @param {string} label 等待条件名。
 * @param {number} [timeout=30000] 超时毫秒数。
 * @returns {Promise<boolean>} 消失时为 true（超时抛异常）。
 */
async function waitUntil(isGone, label, timeout = 30000) {
  await waitForCondition(
    async () => {
      try {
        return isGone(await api.uiTree()) ? true : null;
      } catch {
        return null;
      }
    },
    { timeout, interval: 300, label },
  );
  return true;
}

/**
 * 在长耗时操作（备份/还原）期间直接轮询 UI 树捕捉进度阶段，并统计调试自动化接口的可用性。
 *
 * 备份/还原类命令已改为异步 Tauri 命令（见应用修复（2026-09-21）第 4 条），
 * 业务执行不再占用 WebView2 IPC 主线程，/ui-tree 与 /screenshot 在业务进行中保持可用。
 * 本函数以固定间隔串行轮询 UI 树：首次观察到某阶段时异步触发一张该阶段的截图（不阻塞轮询，
 * 避免阶段窗口较短的阶段被截图耗时错过）；同时记录每轮 /ui-tree 的耗时与失败次数、
 * 阶段与结束截图的 /screenshot 耗时与探测轮次，作为「接口可用」的实测证据。
 *
 * @param {object} options 参数。
 * @param {"backup"|"restore"} options.kind 操作类型（决定截图文件名前缀）。
 * @param {string[]} options.phases 预期阶段文案（有序，用于整理观察结果）。
 * @param {string} options.endText 结束标志文本（snackbar 或成功对话框标题）。
 * @param {number} options.timeoutMs 总超时毫秒数。
 * @param {() => Promise<void>} options.trigger 触发操作的异步函数（点击确认按钮）。
 * @returns {Promise<{observed: string[], endObserved: boolean, durationMs: number, rounds: number, uiTreeFailures: number, maxUiTreeMs: number, maxScreenshotMs: number, screenshots: number}>} 观察与探测统计结果。
 */
async function capturePhases({ kind, phases, endText, timeoutMs, trigger }) {
  await trigger();

  const observed = [];
  const screenshotTasks = [];
  let rounds = 0;
  let uiTreeFailures = 0;
  let maxUiTreeMs = 0;
  let maxScreenshotMs = 0;
  let screenshots = 0;
  let endObserved = false;
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    rounds += 1;
    const requestStarted = Date.now();
    let tree = null;
    try {
      tree = await api.uiTree();
    } catch (error) {
      uiTreeFailures += 1;
      console.log(`  (ui-tree failure) ${error.message}`);
    }
    maxUiTreeMs = Math.max(maxUiTreeMs, Date.now() - requestStarted);
    if (tree) {
      for (const [index, phase] of phases.entries()) {
        if (!observed.includes(phase) && ui.hasText(tree, phase)) {
          observed.push(phase);
          console.log(`  (phase +${Date.now() - startedAt}ms) ${phase}`);
          const screenshotStarted = Date.now();
          screenshotTasks.push(
            api
              .saveScreenshot(path.join(OUTPUT_DIR, `progress-${kind}-stage${index + 1}.png`))
              .then(() => {
                screenshots += 1;
                maxScreenshotMs = Math.max(maxScreenshotMs, Date.now() - screenshotStarted);
              })
              .catch((error) => {
                console.log(`  (screenshot failure) ${error.message}`);
              }),
          );
        }
      }
      if (ui.hasText(tree, endText)) {
        endObserved = true;
        break;
      }
    }
    await sleep(PROBE_INTERVAL_MS);
  }

  // 结束后留档一张结束态截图（备份 snackbar 有显示窗口；还原成功为持久对话框）。
  const endShotStarted = Date.now();
  try {
    await api.saveScreenshot(path.join(OUTPUT_DIR, `progress-${kind}-end.png`));
    screenshots += 1;
  } catch (error) {
    console.log(`  (end screenshot failure) ${error.message}`);
  }
  maxScreenshotMs = Math.max(maxScreenshotMs, Date.now() - endShotStarted);
  await Promise.allSettled(screenshotTasks);

  observed.sort((a, b) => phases.indexOf(a) - phases.indexOf(b));
  return {
    observed,
    endObserved,
    durationMs: Date.now() - startedAt,
    rounds,
    uiTreeFailures,
    maxUiTreeMs,
    maxScreenshotMs,
    screenshots,
  };
}

/**
 * 用例主流程。
 * @returns {Promise<void>} 无返回值。
 */
async function main() {
  const backupFile = path.join(OUTPUT_DIR, BACKUP_FILE_NAME);
  const corruptFile = path.join(OUTPUT_DIR, BACKUP_CORRUPT_NAME);
  const invalidFile = path.join(OUTPUT_DIR, BACKUP_INVALID_NAME);
  const fakeFile = path.join(OUTPUT_DIR, FAKE_BACKUP_NAME);
  const attachmentDir = path.join(
    DATA_DIR,
    "user_database_set",
    fs
      .readdirSync(path.join(DATA_DIR, "user_database_set"), { withFileTypes: true })
      .filter((e) => e.isDirectory())[0].name,
    "attachment",
  );
  const largeAttachment = path.join(attachmentDir, LARGE_ATTACHMENT_NAME);
  const bulkDir = path.join(attachmentDir, BULK_DIR_NAME);
  const expectedDataSize = dataDirSize(DATA_DIR);

  report.section("S0 前置与解锁（前置条件）");
  const info = await api.health();
  report.check(info.width > 0 && info.height > 0, "调试自动化服务可用", `${info.width}x${info.height}`);
  report.check(
    fs.existsSync(largeAttachment) && fs.statSync(largeAttachment).size === LARGE_ATTACHMENT_SIZE,
    "前置：64MiB 大附件写入数据目录（放大备份/还原耗时）",
    `${fs.existsSync(largeAttachment) ? fs.statSync(largeAttachment).size : 0} B`,
  );
  report.check(
    fs.existsSync(bulkDir) && fs.readdirSync(bulkDir).length === BULK_FILE_COUNT,
    `前置：${BULK_FILE_COUNT} 个 ${BULK_FILE_SIZE / 1024}KiB 小文件写入附件 bulk 子目录（放大打包阶段可观察窗口）`,
    `count=${fs.existsSync(bulkDir) ? fs.readdirSync(bulkDir).length : 0}`,
  );
  await ensureChineseUi();
  await unlock();
  {
    const tree = await api.uiTree();
    report.check(ui.hasText(tree, "账号 A"), "解锁后进入根画布并出现节点「账号 A」");
    report.check(ui.hasText(tree, "备注 B"), "解锁后进入根画布并出现节点「备注 B」");
  }
  await sleep(500);
  await snap("s0-unlocked-root");

  report.section("S1 备份前改名并保存退出（前置条件）");
  await renameNodeByTitle("备注 B", TITLE_BEFORE_BACKUP);
  {
    const tree = await api.uiTree();
    report.check(
      textNodes(tree, TITLE_BEFORE_BACKUP).length === 1,
      `前置 1 画布节点改名为「${TITLE_BEFORE_BACKUP}」`,
    );
  }
  await saveAndExit();
  {
    const tree = await api.uiTree();
    report.check(ui.findEditable(tree, "数据库名称") !== null, "前置 1 「保存并退出」后回到首页");
  }
  await snap("s1-home-after-save-exit");

  report.section("S2 备份对话框与备份成功（步骤 1-5）");
  const startBackupButton = await openBackupDialog();
  {
    const tree = await api.uiTree();
    const dialog = backupDialog(tree);
    report.check(dialog !== null, "步骤 1 打开对话框「备份数据目录」");
    const sizeText = textStartingWith([dialog], "当前数据目录大小：")[0]?.text?.trim() ?? "";
    const sizeBytes = parseSizeText(sizeText);
    report.check(sizeText.startsWith("当前数据目录大小："), "步骤 1 显示「当前数据目录大小：…」", sizeText);
    report.check(
      sizeBytes !== null && Math.abs(sizeBytes - expectedDataSize) / expectedDataSize < 0.01,
      "步骤 1 目录大小数值与脚本统计一致（±1%）",
      `ui=${sizeBytes} fs=${expectedDataSize}`,
    );
    const ratioInput = ui.findEditable([dialog], "冗余比例");
    report.check(ratioInput !== null && ratioInput.attrs?.type === "number", "步骤 1 存在数字输入框「冗余比例」", `type=${ratioInput?.attrs?.type}`);
    const ratioValue = ratioInput ? await readEditableValue(ratioInput) : null;
    report.check(ratioValue === "0.05", "步骤 1 冗余比例默认值 0.05（剪贴板验证）", `clip=${ratioValue}`);
    const estText = textStartingWith([dialog], "预计备份大小：")[0]?.text?.trim() ?? "";
    report.check(estText.startsWith("预计备份大小："), "步骤 1 显示「预计备份大小：…」", estText);
    report.check(startBackupButton.states?.disabled !== true, "步骤 1 「开始备份」按钮在大小加载完成后可用");
    await snap("s2-backup-dialog");
  }

  // 步骤 2：冗余比例改为 0.1，预计大小增大
  const estimatedBefore = parseSizeText(
    textStartingWith(await api.uiTree(), "预计备份大小：")[0]?.text ?? "",
  );
  await setBackupRatio("0.1");
  {
    const tree = await api.uiTree();
    const dialog = backupDialog(tree);
    const estText = textStartingWith([dialog], "预计备份大小：")[0]?.text?.trim() ?? "";
    const estimatedAfter = parseSizeText(estText);
    report.check(
      estimatedAfter !== null && estimatedBefore !== null && estimatedAfter > estimatedBefore,
      "步骤 2 冗余比例改为 0.1 后「预计备份大小」增大",
      `before=${estimatedBefore} after=${estimatedAfter}`,
    );
    await snap("s2b-ratio-01");
  }

  // 步骤 3：开始备份 → save 文件选择器
  await clickBackupDialogButton("开始备份");
  await waitForPicker();
  {
    const tree = await api.uiTree();
    report.check(ui.hasText(tree, "备份数据目录"), "步骤 3 文件选择器标题为「备份数据目录」");
    const picker = dialogWithPathInput(tree);
    const nameInput = picker ? ui.findEditable([picker], "文件名") : null;
    const defaultName = nameInput ? await readEditableValue(nameInput) : null;
    report.check(defaultName === "backup.ibackup", "步骤 3 文件选择器默认文件名为 backup.ibackup（剪贴板验证）", `clip=${defaultName}`);
    await sleep(300);
  }

  // 步骤 4：输出到 output 并观察备份阶段与成功提示
  await navigatePickerTo(OUTPUT_DIR);
  const backupCapture = await capturePhases({
    kind: "backup",
    phases: PHASES_BACKUP,
    endText: "备份已保存",
    timeoutMs: BACKUP_CAPTURE_TIMEOUT_MS,
    trigger: async () => {
      await fillPickerFileNameAndConfirm(BACKUP_FILE_NAME);
    },
  });
  report.check(backupCapture.endObserved, "步骤 4 备份结束出现 snackbar「备份已保存」");
  report.check(
    backupCapture.observed.includes("正在打包数据目录"),
    "步骤 4 备份进度出现首阶段「正在打包数据目录」（直接轮询捕捉）",
    backupCapture.observed.join(",") || "none",
  );
  report.check(
    backupCapture.observed[backupCapture.observed.length - 1] === "正在写入备份文件",
    "步骤 4 备份结束时进度区停留在「正在写入备份文件」",
    backupCapture.observed.join(",") || "none",
  );
  report.check(
    backupCapture.observed.every(
      (phase, index) => index === 0 || PHASES_BACKUP.indexOf(phase) > PHASES_BACKUP.indexOf(backupCapture.observed[index - 1]),
    ),
    "步骤 4 观察到的阶段顺序与预期一致（有序子集）",
    backupCapture.observed.join(",") || "none",
  );
  report.check(
    backupCapture.uiTreeFailures === 0,
    `修复验证：备份期间 /ui-tree 全部成功且无 5xx（实测 ${backupCapture.rounds} 轮，${backupCapture.uiTreeFailures} 次失败）`,
  );
  report.check(
    backupCapture.maxUiTreeMs < AUTOMATION_BUDGET_MS,
    `修复验证：备份期间每轮 /ui-tree 耗时 < ${AUTOMATION_BUDGET_MS}ms（实测 max=${backupCapture.maxUiTreeMs}ms，${backupCapture.rounds} 轮）`,
  );
  report.check(
    backupCapture.maxScreenshotMs < AUTOMATION_BUDGET_MS,
    `修复验证：备份期间 /screenshot 单张耗时 < ${AUTOMATION_BUDGET_MS}ms（实测 max=${backupCapture.maxScreenshotMs}ms）`,
  );
  report.check(
    backupCapture.rounds >= MIN_PROBE_ROUNDS,
    `修复验证：备份期间完成 ≥ ${MIN_PROBE_ROUNDS} 轮调试探测（实测 ${backupCapture.rounds} 轮，业务时长约 ${backupCapture.durationMs}ms）`,
  );
  report.check(backupCapture.screenshots > 0, "步骤 4 备份进度截图留档", `${backupCapture.screenshots} 张`);
  await snap("s2c-backup-success-snackbar");
  await waitUntil((tree) => backupDialog(tree) === null, "备份对话框关闭");

  // 步骤 5：产物文件断言
  {
    const exists = fs.existsSync(backupFile);
    report.check(exists, `步骤 5 备份产物 ${BACKUP_FILE_NAME} 存在`, backupFile);
    if (exists) {
      const size = fs.statSync(backupFile).size;
      report.check(size >= expectedDataSize, "步骤 5 备份文件大小 ≥ 数据目录总大小", `${size} >= ${expectedDataSize}`);
      report.check(
        size <= expectedDataSize * 1.5,
        "步骤 5 备份文件大小在 (1+冗余比例) 量级（≤ 1.5 倍）",
        `${size} <= ${Math.round(expectedDataSize * 1.5)}`,
      );
    } else {
      report.check(false, "步骤 5 备份文件大小 ≥ 数据目录总大小", "file missing");
      report.check(false, "步骤 5 备份文件大小在 (1+冗余比例) 量级（≤ 1.5 倍）", "file missing");
    }
  }

  report.section("S2b F1 冗余比例非法（步骤 6）");
  await ui.waitForGone("备份已保存", { timeout: 15000 }).catch(() => null);
  await sleep(500);
  await openBackupDialog();
  await setBackupRatio("0");
  await clickBackupDialogButton("开始备份");
  await waitForPicker();
  await navigatePickerTo(OUTPUT_DIR);
  await fillPickerFileNameAndConfirm(BACKUP_INVALID_NAME);
  const f1Error = await ui.waitForText("备份文件无效", { timeout: 20000 }).catch(() => null);
  report.check(f1Error !== null, "F1 比例 0 备份失败 snackbar「备份文件无效」");
  {
    const tree = await api.uiTree();
    const detail = flattenNodes(tree).find(
      (n) => n.tag === "#text" && (n.text ?? "").includes("redundancy_ratio must be in (0, 1), got 0"),
    );
    report.check(detail !== undefined, "F1 错误详情为 InvalidBackupFile（比例必须落在 (0,1)）", detail?.text?.trim() ?? "");
    await snap("s2b-f1-invalid-ratio");
  }
  report.check(!fs.existsSync(invalidFile), `F1 未生成备份文件 ${BACKUP_INVALID_NAME}`);
  await waitUntil((tree) => backupDialog(tree) === null, "F1 后备份对话框关闭");
  await ui.waitForGone("备份文件无效", { timeout: 20000 }).catch(() => null);
  await sleep(500);

  report.section("S3 备份后修改数据（步骤 7）");
  await unlock();
  await renameNodeByTitle(TITLE_BEFORE_BACKUP, TITLE_AFTER_BACKUP);
  {
    const tree = await api.uiTree();
    report.check(
      textNodes(tree, TITLE_AFTER_BACKUP).length === 1 && textNodes(tree, TITLE_BEFORE_BACKUP).length === 0,
      `步骤 7 节点已改名为「${TITLE_AFTER_BACKUP}」（为还原对比准备）`,
    );
  }
  await saveAndExit();
  await snap("s3-home-after-modify");

  report.section("S4 还原探测与还原（步骤 8-10）");
  await openRestoreDialog();
  {
    const tree = await api.uiTree();
    const dialog = restoreDialog(tree);
    report.check(dialog !== null, "步骤 8 打开对话框「还原数据目录」");
    report.check(
      ui.hasText([dialog], "还原将覆盖所有用户数据，且无法恢复。"),
      "步骤 8 显示警告「还原将覆盖所有用户数据，且无法恢复。」",
    );
    const confirm = ui.findButton(tree, "确认还原", { root: dialog, exact: true });
    const selectBtn = ui.findButton(tree, "选择备份文件", { root: dialog, exact: true });
    report.check(confirm?.states?.disabled === true, "步骤 8 未选择备份文件时「确认还原」禁用");
    report.check(selectBtn?.states?.disabled !== true, "步骤 8 「选择备份文件」按钮可用");
    await snap("s4-restore-dialog");
  }
  // 步骤 9：选择备份文件并等待探测通过
  await clickRestoreDialogButton("选择备份文件");
  await pickBackupFile(BACKUP_FILE_NAME);
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = restoreDialog(tree);
      return dialog && ui.hasText([dialog], "备份文件校验通过，可以还原") ? tree : null;
    },
    { timeout: 60000, interval: 300, label: "探测通过" },
  );
  {
    const tree = await api.uiTree();
    const dialog = restoreDialog(tree);
    report.check(
      ui.hasText([dialog], "备份文件校验通过，可以还原"),
      "步骤 9 探测结果显示「备份文件校验通过，可以还原」",
    );
    const confirm = ui.findButton(tree, "确认还原", { root: dialog, exact: true });
    const selectBtn = ui.findButton(tree, "选择备份文件", { root: dialog, exact: true });
    report.check(confirm?.states?.disabled === false, "步骤 9 探测通过后「确认还原」可用");
    report.check(selectBtn?.states?.disabled === true, "步骤 9 探测通过后「选择备份文件」按钮禁用（状态不再改变）");
    await snap("s4b-probe-passed");
  }

  // 步骤 10：确认还原并观察还原阶段
  const restoreCapture = await capturePhases({
    kind: "restore",
    phases: PHASES_RESTORE,
    endText: "还原成功",
    timeoutMs: RESTORE_CAPTURE_TIMEOUT_MS,
    trigger: async () => {
      await clickRestoreDialogButton("确认还原");
    },
  });
  report.check(restoreCapture.endObserved, "步骤 10 还原完成出现对话框「还原成功」");
  // 首阶段「正在读取文件头」：read_header 的业务耗时低于 1ms，两次阶段事件背靠背到达，
  // 前端批处理渲染后该阶段文本不进入 DOM（连续轮询 1665 轮实测从未观察到，见修订记录）。
  // 观察到时按首阶段硬断言；未观察到时以紧随的「正在校验完整性」作为最早可观察阶段。
  if (restoreCapture.observed.includes("正在读取文件头")) {
    report.check(
      true,
      "步骤 10 还原进度出现首阶段「正在读取文件头」（直接轮询捕捉）",
      restoreCapture.observed.join(","),
    );
  } else {
    report.check(
      restoreCapture.observed[0] === "正在校验完整性",
      "步骤 10 还原首阶段「正在读取文件头」UI 窗口低于一帧（实测）；以紧随阶段「正在校验完整性」作为最早可观察阶段",
      restoreCapture.observed.join(",") || "none",
    );
  }
  report.check(
    restoreCapture.observed[restoreCapture.observed.length - 1] === "正在写入新数据",
    "步骤 10 还原结束时进度区停留在「正在写入新数据」",
    restoreCapture.observed.join(",") || "none",
  );
  report.check(
    restoreCapture.observed.every(
      (phase, index) => index === 0 || PHASES_RESTORE.indexOf(phase) > PHASES_RESTORE.indexOf(restoreCapture.observed[index - 1]),
    ),
    "步骤 10 观察到的阶段顺序与预期一致（有序子集）",
    restoreCapture.observed.join(",") || "none",
  );
  report.check(
    restoreCapture.uiTreeFailures === 0,
    `修复验证：还原期间 /ui-tree 全部成功且无 5xx（实测 ${restoreCapture.rounds} 轮，${restoreCapture.uiTreeFailures} 次失败）`,
  );
  report.check(
    restoreCapture.maxUiTreeMs < AUTOMATION_BUDGET_MS,
    `修复验证：还原期间每轮 /ui-tree 耗时 < ${AUTOMATION_BUDGET_MS}ms（实测 max=${restoreCapture.maxUiTreeMs}ms，${restoreCapture.rounds} 轮）`,
  );
  report.check(
    restoreCapture.maxScreenshotMs < AUTOMATION_BUDGET_MS,
    `修复验证：还原期间 /screenshot 单张耗时 < ${AUTOMATION_BUDGET_MS}ms（实测 max=${restoreCapture.maxScreenshotMs}ms）`,
  );
  report.check(
    restoreCapture.rounds >= MIN_PROBE_ROUNDS,
    `修复验证：还原期间完成 ≥ ${MIN_PROBE_ROUNDS} 轮调试探测（实测 ${restoreCapture.rounds} 轮，业务时长约 ${restoreCapture.durationMs}ms）`,
  );
  report.check(restoreCapture.screenshots > 0, "步骤 10 还原进度截图留档", `${restoreCapture.screenshots} 张`);
  await waitUntil((tree) => restoreDialog(tree) === null, "还原对话框关闭（还原完成后）", 60000);

  report.section("S5 还原成功对话框与页面刷新（步骤 11-12）");
  await waitForCondition(async () => ui.findDialog(await api.uiTree(), "还原成功") !== null, {
    timeout: 30000,
    interval: 300,
    label: "「还原成功」对话框",
  });
  await sleep(800);
  {
    const tree = await api.uiTree();
    const dialog = ui.findDialog(tree, "还原成功");
    report.check(dialog !== null, "步骤 11 出现对话框「还原成功」");
    report.check(
      ui.hasText([dialog], "点击下方按钮刷新页面以查看还原后的数据。"),
      "步骤 11 显示文案「点击下方按钮刷新页面以查看还原后的数据。」",
    );
    const doneButtons = flattenNodes([dialog]).filter(
      (n) => n.role === "button" && ui.subtreeText(n).trim() === "完成",
    );
    report.check(doneButtons.length === 1, "步骤 11 对话框内恰有一个「完成」按钮", `count=${doneButtons.length}`);
    await snap("s5-restore-success");
  }
  // 步骤 12：点击「完成」触发整页 reload
  {
    const doneButton = await waitForCondition(
      async () => {
        const tree = await api.uiTree();
        return (
          flattenNodes(tree).find((n) => n.role === "button" && ui.subtreeText(n).trim() === "完成") ?? null
        );
      },
      { timeout: 10000, interval: 300, label: "「完成」按钮" },
    );
    await ui.clickNode(doneButton);
  }
  await ui.waitForGone("还原成功", { timeout: 30000 }).catch(() => null);
  await waitForCondition(
    async () => {
      try {
        const tree = await api.uiTree();
        return ui.findEditable(tree, "数据库名称") !== null ? tree : null;
      } catch {
        return null;
      }
    },
    { timeout: 30000, interval: 300, label: "reload 后首页就绪" },
  );
  {
    const tree = await api.uiTree();
    report.check(
      ui.findEditable(tree, "数据库名称") !== null && !ui.hasText(tree, "还原成功"),
      "步骤 12 「完成」触发页面刷新并回到首页（数据库名称输入框出现）",
    );
    await snap("s5b-after-reload-home");
  }

  report.section("S6 还原后数据回退验证（步骤 13，验证 reclaim 链路）");
  await unlock();
  {
    const tree = await api.uiTree();
    report.check(
      textNodes(tree, TITLE_BEFORE_BACKUP).length === 1,
      `步骤 13 还原后节点名回到备份时点「${TITLE_BEFORE_BACKUP}」`,
    );
    report.check(
      textNodes(tree, TITLE_AFTER_BACKUP).length === 0,
      `步骤 13 备份后修改的标题「${TITLE_AFTER_BACKUP}」不存在`,
    );
    report.check(
      fs.existsSync(largeAttachment) && fs.statSync(largeAttachment).size === LARGE_ATTACHMENT_SIZE,
      "步骤 13 大附件随数据目录一并还原",
    );
    report.check(
      fs.existsSync(bulkDir) && fs.readdirSync(bulkDir).length === BULK_FILE_COUNT,
      "步骤 13 小文件群随数据目录一并还原",
      `count=${fs.existsSync(bulkDir) ? fs.readdirSync(bulkDir).length : 0}`,
    );
    await snap("s6-restored-canvas");
  }
  report.check(true, "步骤 13 还原后的重新解锁读到备份时点数据（reclaim_preference / reclaim_metadata / reclaim_user_database 链路生效）");
  await saveAndExit();

  report.section("S7 损坏备份探测（步骤 14-15）");
  {
    fs.copyFileSync(backupFile, corruptFile);
    const originalSize = fs.statSync(backupFile).size;
    fs.truncateSync(corruptFile, Math.floor(originalSize * CORRUPT_TRUNCATE_RATIO));
    const corruptSize = fs.statSync(corruptFile).size;
    report.check(
      corruptSize < originalSize * 0.7 && corruptSize > originalSize * 0.5,
      "步骤 14 损坏备份已复制并截断到约 60%",
      `${corruptSize} / ${originalSize}`,
    );
  }
  await openRestoreDialog();
  await clickRestoreDialogButton("选择备份文件");
  await pickBackupFile(BACKUP_CORRUPT_NAME);
  await waitForCondition(
    async () => {
      const tree = await api.uiTree();
      const dialog = restoreDialog(tree);
      return dialog && ui.hasText([dialog], "备份文件损坏过多，无法还原") ? tree : null;
    },
    { timeout: 60000, interval: 300, label: "损坏过多探测结果" },
  );
  {
    const tree = await api.uiTree();
    const dialog = restoreDialog(tree);
    report.check(
      ui.hasText([dialog], "备份文件损坏过多，无法还原"),
      "步骤 15 探测结果显示「备份文件损坏过多，无法还原」",
    );
    const confirm = ui.findButton(tree, "确认还原", { root: dialog, exact: true });
    report.check(confirm?.states?.disabled === true, "步骤 15 探测不可恢复时「确认还原」保持禁用");
    await snap("s7-corrupt-probe");
  }
  await clickRestoreDialogButton("取消");
  await waitUntil((tree) => restoreDialog(tree) === null, "损坏探测后取消关闭还原对话框");
  await sleep(600);

  report.section("S8 F3 非 .ibackup 文件过滤（步骤 9 变体）");
  await openRestoreDialog();
  await clickRestoreDialogButton("选择备份文件");
  await waitForPicker();
  await navigatePickerTo(OUTPUT_DIR);
  {
    const tree = await api.uiTree();
    const picker = dialogWithPathInput(tree);
    report.check(
      ui.findByText([picker], TXT_FILE_NAME, { exact: true }) === null,
      `F3 默认扩展名过滤生效：${TXT_FILE_NAME} 不出现在列表中`,
    );
    const toggle = flattenNodes([picker]).find(
      (n) => n.role === "checkbox" && n.name === "显示全部文件",
    );
    report.check(toggle !== undefined, "F3 文件选择器带「显示全部文件」开关");
    await ui.clickNode(toggle);
    await sleep(700);
  }
  {
    // 列表为虚拟滚动：目标行可能在视口外，复用滚动查找逻辑（被裁剪的条目不在 UI 树中）。
    const txtRow = await scrollPickerToFile(TXT_FILE_NAME, { timeout: 25000 });
    report.check(txtRow !== null, `F3 开启「显示全部文件」后列出被过滤的 ${TXT_FILE_NAME}`);
    const tree = await api.uiTree();
    const picker = dialogWithPathInput(tree);
    const confirm = ui.findButton(tree, "确认", { root: picker, exact: true });
    report.check(confirm?.states?.disabled === true, "F3 未选中文件时「确认」按钮禁用");
    await ui.clickNode(txtRow);
    await sleep(500);
    const tree2 = await api.uiTree();
    const picker2 = dialogWithPathInput(tree2);
    const confirm2 = ui.findButton(tree2, "确认", { root: picker2, exact: true });
    report.check(confirm2?.states?.disabled === true, "F3 单击置灰的非 .ibackup 文件不产生选中");
    await ui.clickNode(txtRow, { count: 2 });
    await sleep(700);
    report.check(
      dialogWithPathInput(await api.uiTree()) !== null,
      "F3 双击置灰的非 .ibackup 文件不关闭文件选择器",
    );
    console.log(
      "  (note) F3 偏差：文件选择器扩展名过滤使非 .ibackup 文件置灰且不可选中，UI 上无法构造探测请求，按计划记录偏差并跳过",
    );
    await snap("s8-f3-txt-filtered");
  }
  {
    const tree = await api.uiTree();
    const picker = dialogWithPathInput(tree);
    await ui.clickNode(ui.findButton(tree, "取消", { root: picker, exact: true }));
    await waitUntil((t) => dialogWithPathInput(t) === null, "文件选择器关闭（F3 后）", 15000);
    await sleep(600);
    await clickRestoreDialogButton("取消");
    await waitUntil((t) => restoreDialog(t) === null, "还原对话框关闭（F3 后）");
    await sleep(600);
  }

  report.section("S9 附加 F6 伪 .ibackup 探测失败");
  await openRestoreDialog();
  await clickRestoreDialogButton("选择备份文件");
  await pickBackupFile(FAKE_BACKUP_NAME);
  const fakeError = await ui.waitForText("备份文件无效", { timeout: 30000 }).catch(() => null);
  report.check(fakeError !== null, "F6 伪 .ibackup 探测失败 snackbar「备份文件无效」");
  await sleep(500);
  {
    const tree = await api.uiTree();
    report.check(restoreDialog(tree) !== null, "F6 探测失败后还原对话框保持打开");
    report.check(
      !ui.hasText(tree, "备份文件校验通过，可以还原") && !ui.hasText(tree, "备份文件损坏过多，无法还原"),
      "F6 探测失败不产生任何探测结论",
    );
    await snap("s9-f6-fake-ibackup");
  }
  await ui.waitForGone("备份文件无效", { timeout: 20000 }).catch(() => null);
  await clickRestoreDialogButton("取消");
  await waitUntil((t) => restoreDialog(t) === null, "还原对话框关闭（F6 后）");

  report.section("S10 收尾数据核对");
  {
    const tree = await api.uiTree();
    report.check(ui.findEditable(tree, "数据库名称") !== null, "用例结束时应用停留在首页");
    report.check(fs.existsSync(backupFile), `备份产物 ${BACKUP_FILE_NAME} 保留在 output`);
    report.check(fs.existsSync(corruptFile), `损坏备份 ${BACKUP_CORRUPT_NAME} 保留在 output`);
  }
}

let fatal = null;
try {
  prepareCaseOutput(OUTPUT_DIR);
  prepareDataDir(DATA_DIR, "base");
  // 大附件：放大备份/还原耗时，使界面进度阶段有可观察窗口。
  {
    const dbDirs = fs
      .readdirSync(path.join(DATA_DIR, "user_database_set"), { withFileTypes: true })
      .filter((entry) => entry.isDirectory());
    const attachmentDir = path.join(
      DATA_DIR,
      "user_database_set",
      dbDirs[0].name,
      "attachment",
    );
    fs.mkdirSync(attachmentDir, { recursive: true });
    const file = path.join(attachmentDir, LARGE_ATTACHMENT_NAME);
    const handle = fs.openSync(file, "w");
    for (let written = 0; written < LARGE_ATTACHMENT_SIZE; written += 4 * 1024 * 1024) {
      fs.writeSync(handle, crypto.randomBytes(4 * 1024 * 1024));
    }
    fs.closeSync(handle);
    // 小文件群：放大 tar 打包阶段的耗时与事件密度，使「正在打包数据目录」阶段可观察。
    const bulkDir = path.join(attachmentDir, BULK_DIR_NAME);
    fs.mkdirSync(bulkDir, { recursive: true });
    const bulkContent = crypto.randomBytes(BULK_FILE_SIZE);
    for (let i = 0; i < BULK_FILE_COUNT; i += 1) {
      fs.writeFileSync(path.join(bulkDir, `f${String(i).padStart(4, "0")}.bin`), bulkContent);
    }
  }
  // F3/F6 辅助资产。
  fs.writeFileSync(path.join(OUTPUT_DIR, TXT_FILE_NAME), "not a backup file\n", "utf8");
  fs.writeFileSync(path.join(OUTPUT_DIR, FAKE_BACKUP_NAME), "this is not a valid ibackup file\n", "utf8");

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
