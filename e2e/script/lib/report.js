// 用例断言收集与报告落盘。

import fs from "node:fs";
import path from "node:path";

/** 用例断言与结果收集器 */
export class Report {
  /**
   * 创建报告实例。
   * @param {string} name 用例名称。
   */
  constructor(name) {
    this.name = name;
    this.checks = [];
    this.startTime = Date.now();
    this.currentSection = "";
  }

  /**
   * 输出小节标题，后续断言归入该小节（仅用于报告可读性）。
   * @param {string} title 小节标题。
   * @returns {void} 无返回值。
   */
  section(title) {
    this.currentSection = title;
    console.log(`\n== ${title} ==`);
  }

  /**
   * 记录一条断言结果并输出到控制台。
   * @param {boolean} ok 断言是否通过。
   * @param {string} message 断言描述。
   * @param {string} [detail=""] 附加信息（测量值、文件路径、错误消息等）。
   * @returns {boolean} 断言是否通过。
   */
  check(ok, message, detail = "") {
    const passed = !!ok;
    this.checks.push({ ok: passed, section: this.currentSection, message, detail: String(detail) });
    console.log(`  [${passed ? "PASS" : "FAIL"}] ${message}${detail ? ` -- ${detail}` : ""}`);
    return passed;
  }

  /**
   * 汇总断言结果。
   * @returns {{name: string, total: number, failed: number, durationMs: number, checks: object[]}} 汇总对象。
   */
  summary() {
    return {
      name: this.name,
      total: this.checks.length,
      failed: this.checks.filter((c) => !c.ok).length,
      durationMs: Date.now() - this.startTime,
      checks: this.checks,
    };
  }
}

/**
 * 把报告写入 output\report.json，打印总结并按结果设置进程退出码。
 * @param {Report} report 报告实例。
 * @param {string} outputDir 用例输出目录。
 * @param {Error|null} [fatal=null] 导致流程中断的异常（若有）。
 * @returns {object} 写入的汇总对象。
 */
export function finalizeReport(report, outputDir, fatal = null) {
  const summary = report.summary();
  if (fatal) {
    summary.fatal = String(fatal.stack ?? fatal);
  }
  fs.mkdirSync(outputDir, { recursive: true });
  fs.writeFileSync(path.join(outputDir, "report.json"), JSON.stringify(summary, null, 2), "utf8");
  const passed = !fatal && summary.failed === 0;
  console.log(
    `\n[${report.name}] ${passed ? "PASS" : "FAIL"} - ${summary.total} checks, ${summary.failed} failed, ${summary.durationMs}ms`,
  );
  if (fatal) {
    console.error(`fatal: ${summary.fatal}`);
  }
  process.exitCode = passed ? 0 : 1;
  return summary;
}
