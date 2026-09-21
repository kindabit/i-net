// 共享 fixture 的数据目录准备。

import fs from "node:fs";
import path from "node:path";
import { FIXTURES_DIR } from "./paths.js";

/**
 * 清空目标数据目录，并把指定 fixture 的数据复制进去。
 * @param {string} dataDir 目标用户数据目录（--data-dir 参数值）。
 * @param {string} fixtureName fixture 名称（对应 _fixtures 下的子目录名）。
 * @returns {void} 无返回值。
 */
export function prepareDataDir(dataDir, fixtureName) {
  const source = path.join(FIXTURES_DIR, fixtureName, "data");
  if (!fs.existsSync(source)) {
    throw new Error(`fixture not found: ${source}`);
  }
  fs.rmSync(dataDir, { recursive: true, force: true });
  fs.mkdirSync(path.dirname(dataDir), { recursive: true });
  fs.cpSync(source, dataDir, { recursive: true });
}
