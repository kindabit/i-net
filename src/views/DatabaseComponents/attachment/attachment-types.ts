/**
 * 附件模块的公共类型与纯函数工具。
 *
 * 提供附件预览类型路由（扩展名 → 查看器类型）、MIME 推断
 * 与文件大小格式化，供附件对话框与各查看器组件共用。
 *
 * 设计原则：支持的扩展名一定存在对应的 MIME 映射与预览组件路由；
 * 不支持的扩展名统一返回 null，由调用方提示"不支持预览"。
 */

/** 附件查看器类型；不支持的类型路由结果为 null（提示导出后自行打开） */
export type AttachmentViewerType = "omni" | "text";

/** 各查看器类型支持的扩展名（小写、不含点） */
const EXTENSIONS_BY_TYPE: Record<AttachmentViewerType, readonly string[]> = {
  omni: [
    // 图片
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico",
    // 音频
    "mp3", "wav", "ogg", "flac", "m4a", "aac", "opus",
    // 视频
    "mp4", "webm", "mov",
    // Office / PDF
    // Word
    "docx", "docm", "dotx", "dotm", "doc", "dot",
    // PowerPoint
    "pptx", "pptm", "potx", "potm", "ppsx", "ppsm", "ppt",
    // Excel / Spreadsheet
    "xlsx", "xltx", "xlsm", "xlsb", "xls", "xlt", "xltm", "ods", "fods",
    // PDF
    "pdf",
  ],
  text: [
    "txt", "text", "md", "markdown", "json", "jsonc", "yaml", "yml", "toml",
    "xml", "html", "htm", "js", "mjs", "cjs", "ts", "mts", "cts", "jsx",
    "tsx", "vue", "css", "scss", "less", "py", "rs", "java", "c", "h", "cpp",
    "cxx", "cc", "hpp", "cs", "go", "rb", "php", "swift", "kt", "kts", "sh",
    "bash", "zsh", "ps1", "bat", "cmd", "sql", "ini", "cfg", "conf",
    "properties", "log", "csv", "tsv", "env", "diff", "patch", "lua", "r",
    "dart", "scala", "pl", "gitignore", "dockerfile", "editorconfig",
  ],
};

/** 各扩展名对应的 MIME 类型（小写、不含点；仅覆盖受支持的扩展名） */
const MIME_BY_EXTENSION: Record<string, string> = {
  pdf: "application/pdf",
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  bmp: "image/bmp",
  svg: "image/svg+xml",
  ico: "image/x-icon",
  mp3: "audio/mpeg",
  wav: "audio/wav",
  ogg: "audio/ogg",
  flac: "audio/flac",
  m4a: "audio/mp4",
  aac: "audio/aac",
  opus: "audio/opus",
  mp4: "video/mp4",
  webm: "video/webm",
  mov: "video/quicktime",
  docx: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  docm: "application/vnd.ms-word.document.macroEnabled.12",
  dotx: "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
  dotm: "application/vnd.ms-word.template.macroEnabled.12",
  doc: "application/msword",
  dot: "application/msword",
  pptx: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  pptm: "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
  potx: "application/vnd.openxmlformats-officedocument.presentationml.template",
  potm: "application/vnd.ms-powerpoint.template.macroEnabled.12",
  ppsx: "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
  ppsm: "application/vnd.ms-powerpoint.slideshow.macroEnabled.12",
  ppt: "application/vnd.ms-powerpoint",
  xlsx: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  xltx: "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
  xlsm: "application/vnd.ms-excel.sheet.macroEnabled.12",
  xlsb: "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
  xls: "application/vnd.ms-excel",
  xlt: "application/vnd.ms-excel",
  xltm: "application/vnd.ms-excel.template.macroEnabled.12",
  ods: "application/vnd.oasis.opendocument.spreadsheet",
  fods: "application/vnd.oasis.opendocument.spreadsheet",
  txt: "text/plain",
  text: "text/plain",
  md: "text/markdown",
  markdown: "text/markdown",
  json: "application/json",
  jsonc: "application/json",
  yaml: "application/yaml",
  yml: "application/yaml",
  toml: "application/toml",
  xml: "application/xml",
  html: "text/html",
  htm: "text/html",
  js: "text/javascript",
  mjs: "text/javascript",
  cjs: "text/javascript",
  ts: "text/typescript",
  mts: "text/typescript",
  cts: "text/typescript",
  jsx: "text/jsx",
  tsx: "text/tsx",
  vue: "text/html",
  css: "text/css",
  scss: "text/css",
  less: "text/css",
  py: "text/x-python",
  rs: "text/x-rust",
  java: "text/x-java",
  c: "text/x-c",
  h: "text/x-c",
  cpp: "text/x-c++",
  cxx: "text/x-c++",
  cc: "text/x-c++",
  hpp: "text/x-c++",
  cs: "text/x-csharp",
  go: "text/x-go",
  rb: "text/x-ruby",
  php: "text/x-php",
  swift: "text/x-swift",
  kt: "text/x-kotlin",
  kts: "text/x-kotlin",
  sh: "application/x-sh",
  bash: "application/x-sh",
  zsh: "application/x-sh",
  ps1: "application/x-powershell",
  bat: "application/bat",
  cmd: "application/bat",
  sql: "application/sql",
  ini: "text/plain",
  cfg: "text/plain",
  conf: "text/plain",
  properties: "text/plain",
  log: "text/plain",
  csv: "text/csv",
  tsv: "text/tab-separated-values",
  env: "text/plain",
  diff: "text/x-diff",
  patch: "text/x-diff",
  lua: "text/x-lua",
  r: "text/x-r",
  dart: "text/x-dart",
  scala: "text/x-scala",
  pl: "text/x-perl",
  gitignore: "text/plain",
  dockerfile: "text/plain",
  editorconfig: "text/plain",
};

/**
 * 取文件名的小写扩展名。
 * @param fileName 文件名
 * @returns 小写扩展名（不含点），无扩展名时返回空字符串
 */
export function extensionOf(fileName: string): string {
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex < 0 || dotIndex === fileName.length - 1) return "";
  return fileName.slice(dotIndex + 1).toLowerCase();
}

/**
 * 按文件扩展名路由预览类型。
 * @param fileName 文件名
 * @returns 支持的查看器类型；不支持的类型返回 null
 */
export function viewerTypeOf(fileName: string): AttachmentViewerType | null {
  const ext = extensionOf(fileName);
  for (const type of Object.keys(EXTENSIONS_BY_TYPE) as AttachmentViewerType[]) {
    if (EXTENSIONS_BY_TYPE[type].includes(ext)) return type;
  }
  return null;
}

/**
 * 按文件扩展名推断 MIME 类型。
 * @param fileName 文件名
 * @returns MIME 类型；扩展名不受支持时返回 null
 */
export function mimeOf(fileName: string): string | null {
  return MIME_BY_EXTENSION[extensionOf(fileName)] ?? null;
}

/**
 * 各扩展名对应的 CodeMirror 语言标识（key 需与文本查看器组件的语言加载器对齐；
 * "text" 表示纯文本，不加载语法高亮）。
 */
const LANGUAGE_BY_EXTENSION: Record<string, string> = {
  txt: "text",
  text: "text",
  md: "markdown",
  markdown: "markdown",
  json: "json",
  jsonc: "json",
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  xml: "xml",
  html: "html",
  htm: "html",
  js: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  ts: "typescript",
  mts: "typescript",
  cts: "typescript",
  jsx: "jsx",
  tsx: "tsx",
  // legacy-modes 未内置 vue，退而使用 html 高亮
  vue: "html",
  css: "css",
  scss: "css",
  less: "css",
  py: "python",
  rs: "rust",
  java: "java",
  c: "c",
  h: "c",
  cpp: "cpp",
  cxx: "cpp",
  cc: "cpp",
  hpp: "cpp",
  cs: "csharp",
  go: "go",
  rb: "ruby",
  php: "php",
  swift: "swift",
  kt: "kotlin",
  kts: "kotlin",
  sh: "shell",
  bash: "shell",
  zsh: "shell",
  ps1: "powershell",
  bat: "text",
  cmd: "text",
  sql: "sql",
  ini: "ini",
  cfg: "ini",
  conf: "ini",
  properties: "ini",
  log: "text",
  csv: "csv",
  tsv: "csv",
  env: "shell",
  diff: "diff",
  patch: "diff",
  lua: "lua",
  r: "r",
  dart: "dart",
  scala: "scala",
  pl: "perl",
  gitignore: "text",
  dockerfile: "dockerfile",
  editorconfig: "ini",
};

/**
 * 按文件扩展名返回 CodeMirror 语言标识，供文本查看器加载对应语言的语法高亮。
 * @param fileName 文件名
 * @returns 语言标识；扩展名未收录时返回 "text"（纯文本，无语法高亮）
 */
export function textLanguageOf(fileName: string): string {
  return LANGUAGE_BY_EXTENSION[extensionOf(fileName)] ?? "text";
}

/**
 * 格式化附件大小。
 * @param size 字节数
 * @returns 人类可读的大小文本（B / KB / MB，非整除保留一位小数）
 */
export function formatSize(size: number): string {
  if (size < 1024) return `${size} B`;
  const kb = size / 1024;
  if (kb < 1024) return `${formatUnit(kb)} KB`;
  return `${formatUnit(kb / 1024)} MB`;
}

/**
 * 格式化数值单位：整数原样，小数保留一位。
 * @param value 待格式化的数值
 * @returns 数值文本
 */
function formatUnit(value: number): string {
  return Number.isInteger(value) ? `${value}` : value.toFixed(1);
}
