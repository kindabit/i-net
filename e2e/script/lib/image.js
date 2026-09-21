// 截图像素比对（基于 pngjs + pixelmatch）。

import fs from "node:fs";
import path from "node:path";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";

/**
 * 读取并解码 PNG 文件。
 * @param {string} filePath PNG 文件路径。
 * @returns {PNG} 解码后的 PNG 对象。
 */
export function readPng(filePath) {
  return PNG.sync.read(fs.readFileSync(filePath));
}

/**
 * 解码 PNG 数据。
 * @param {Buffer} buffer PNG 数据。
 * @returns {PNG} 解码后的 PNG 对象。
 */
export function decodePng(buffer) {
  return PNG.sync.read(buffer);
}

/**
 * 写入 PNG 文件。
 * @param {string} filePath 目标文件路径。
 * @param {PNG} png PNG 对象。
 * @returns {string} 写入的文件路径。
 */
export function writePng(filePath, png) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, PNG.sync.write(png));
  return filePath;
}

/**
 * 裁剪 PNG 的指定区域（自动按图像边界收敛）。
 * @param {PNG} png 源 PNG 对象。
 * @param {{left: number, top: number, width: number, height: number}} region 裁剪区域。
 * @returns {PNG} 裁剪后的 PNG 对象。
 */
export function cropPng(png, { left, top, width, height }) {
  const x = Math.max(0, Math.min(left, png.width));
  const y = Math.max(0, Math.min(top, png.height));
  const w = Math.min(width, png.width - x);
  const h = Math.min(height, png.height - y);
  if (w <= 0 || h <= 0) {
    throw new Error(`cropPng: empty region ${JSON.stringify({ left, top, width, height })}`);
  }
  const out = new PNG({ width: w, height: h });
  PNG.bitblt(png, out, x, y, w, h, 0, 0);
  return out;
}

/**
 * 比对两张 PNG，按差异像素占比判定是否通过。
 * @param {PNG} pngA 基准 PNG。
 * @param {PNG} pngB 待比对 PNG。
 * @param {object} [options] 可选参数。
 * @param {number} [options.threshold=0.1] pixelmatch 每像素颜色阈值（0..1）。
 * @param {number} [options.maxDiffRatio=0.01] 允许的差异像素占比上限。
 * @param {object|null} [options.region=null] 可选比对区域（缺省整图）。
 * @param {string|null} [options.diffFile=null] 可选差异图输出路径。
 * @returns {{pass: boolean, reason: string, width: number, height: number, diffCount: number, diffRatio: number, diffFile: string|null}} 比对结果。
 */
export function comparePng(pngA, pngB, { threshold = 0.1, maxDiffRatio = 0.01, region = null, diffFile = null } = {}) {
  const imgA = region ? cropPng(pngA, region) : pngA;
  const imgB = region ? cropPng(pngB, region) : pngB;
  if (imgA.width !== imgB.width || imgA.height !== imgB.height) {
    return {
      pass: false,
      reason: "size-mismatch",
      width: imgA.width,
      height: imgA.height,
      diffCount: -1,
      diffRatio: 1,
      diffFile: null,
    };
  }
  const diff = new PNG({ width: imgA.width, height: imgA.height });
  const diffCount = pixelmatch(imgA.data, imgB.data, diff.data, imgA.width, imgA.height, {
    threshold,
  });
  const total = imgA.width * imgA.height;
  const diffRatio = total > 0 ? diffCount / total : 0;
  const pass = diffRatio <= maxDiffRatio;
  const written = diffFile ? writePng(diffFile, diff) : null;
  return {
    pass,
    reason: pass ? "within-threshold" : "diff-ratio-exceeded",
    width: imgA.width,
    height: imgA.height,
    diffCount,
    diffRatio,
    diffFile: written,
  };
}

/**
 * 把运行时截图与黄金截图文件比对。
 * @param {Buffer} buffer 运行时截图 PNG 数据。
 * @param {string} captureFile 黄金截图文件路径（case 目录下的 compare_capture_*.png）。
 * @param {object} [options] 可选参数（见 comparePng）。
 * @returns {ReturnType<typeof comparePng>} 比对结果。
 */
export function compareWithCapture(buffer, captureFile, options = {}) {
  return comparePng(readPng(captureFile), decodePng(buffer), options);
}
