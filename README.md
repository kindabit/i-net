# i-net

> 一款可视化的个人敏感数据管理工具，以有向无环图（DAG）的方式组织数据节点之间的关系，数据在本地加密存储。

---

## 项目简介

**i-net** 是一款基于 [Tauri 2.x](https://tauri.app/) 构建的跨平台桌面应用。应用提供<span style="color: #297bda; font-weight: bold;">画布宇宙—画布</span>两级视图，画布宇宙中的每个节点都是一张子画布，子画布内的每个<span style="color: #297bda; font-weight: bold;">节点</span>都是一组个人数据，而<span style="color: #297bda; font-weight: bold;">边</span>就是数据之间的关系。用户通过交互式的有向无环图建立、浏览和管理复杂的数据关系。

---

## 功能特性

- 🧩 **有向无环图**：拖拽创建节点，自由连线，自定义节点颜色，支持自动布局。
- 🗂️ **嵌套画布**：画布宇宙中的节点即子画布，支持无限多层级的嵌套组织。
- 🔒 **本地加密存储**：数据库与附件均使用 AES256 加密，没有任何数据会不经用户允许以明文方式落盘。
- 📎 **多类型附件**：图片 / 音频 / 视频 / PDF / Office 文档（Word、Excel、PowerPoint）无需导出明文即可在应用内直接预览；文本类附件可在应用内直接编辑，内置语法高亮。
- 🧾 **模板与字典**：自定义模板（新建、从节点创建、导入导出）与树形字典，提升录入效率。
- 🌐 **全局搜索**：按关键词搜索节点并快速定位。
- 🗑️ **回收站**：节点、画布与附件均支持逻辑删除、恢复与物理删除；附件还支持孤儿文件检测与清理。
- 🔑 **密码生成器**：内置安全密码生成工具。
- 📋 **剪贴板安全**：敏感内容复制后按设定延迟自动清空，并以进度条提示。
- 🎨 **自定义主题**：内置亮色 / 暗色主题，支持新建、编辑、导入、导出自定义主题。
- 🧭 **多语言**：支持简体中文（zh-CN）与英语（en-US）。
- 📤 **数据库导出**：支持将数据库导出为**明文**可读格式。
- 📦 **数据目录级备份与还原**：以 Reed-Solomon 纠删码格式备份整个数据目录，可在传输过程中承受一定程度的损坏、污染或尾部数据缺失。
- 📜 **操作日志**：精确到字段新旧值的操作日志。

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 应用框架 | Tauri 2.x |
| 包管理器 | pnpm（前端）+ Cargo（后端） |
| 前端框架 | Vue 3.x + TypeScript |
| UI 组件库 | Vuetify 4.x |
| 图可视化 | Vue Flow 1.x |
| 附件预览 | @file-viewer/vue3-full |
| 文本编辑 | CodeMirror 6 |
| 前端测试 | Vitest |
| 后端 | Rust |
| 本地数据库 | SQLite（rusqlite） |
| 加密 | AES-GCM-SIV |
| 容错编码 | Reed-Solomon（reed-solomon-erasure）+ tar 流 |

---

## 项目结构

```text
.
├── src/                             # 前端源码
│   ├── api.ts                       # Tauri 后端接口封装
│   ├── api-types.ts                 # 后端类型定义
│   ├── error-code.ts                # 错误码
│   ├── preferences.ts               # 用户偏好读写
│   ├── vf-convert.ts                # Vue Flow 数据转换
│   ├── App.vue / main.ts            # 根组件与前端入口
│   ├── components/                  # 可复用 Vue 组件
│   │   └── field-editors/           # 按字段类型自动选择的值编辑器
│   ├── composables/                 # 组合式逻辑（自动布局、视口、回收站、剪贴板清除、备份进度等）
│   ├── dictionary/                  # 字典状态管理
│   ├── field-types/                 # 字段类型系统
│   ├── i18n/                        # 国际化（<模块>/<locale>.json）
│   ├── node-colors/                 # 节点色彩
│   ├── router/                      # 路由
│   ├── styles/ / themes/            # 样式与自定义主题系统
│   ├── utils/                       # 工具函数
│   └── views/                       # 页面与页面级组件
│       ├── Home.vue                 # 首页（注册 / 打开数据库）
│       ├── HomeComponents/          # 首页专用组件（归档管理、备份与还原对话框、删除数据库对话框）
│       ├── DatabaseView.vue         # 数据库页面基座
│       ├── CanvasUniverseView.vue   # 画布宇宙页面
│       ├── CanvasView.vue           # 画布页面
│       └── DatabaseComponents/      # 数据库页面组件库（节点、边、附件、模板、字典等）
├── schemas/                         # 字段类型 JSON Schema
├── src-tauri/                       # Tauri / Rust 后端
│   ├── src/
│   │   ├── main.rs / lib.rs         # 后端入口与 Tauri 运行时初始化
│   │   ├── argv.rs                  # 命令行参数解析
│   │   ├── state.rs                 # 全局路径状态
│   │   ├── error_code.rs            # 错误码定义
│   │   ├── common/                  # 通用业务模块（数据库连接、数据版本）
│   │   ├── business/                # 专项业务模块（模块间无源码间依赖）
│   │   │   ├── preference/          # 用户偏好
│   │   │   ├── metadata/            # 用户数据库元数据
│   │   │   ├── clipboard/           # 剪贴板
│   │   │   ├── backup/              # 数据目录级备份与还原（Reed-Solomon + tar 流）
│   │   │   ├── reclaim/             # 还原后刷新各业务模块的内存 connection（避免被旧数据覆盖还原结果）
│   │   │   └── user_database/       # 用户数据库（canvas / node / edge / node_field /
│   │   │                            # template / dictionary / attachment / viewport /
│   │   │                            # log / export / field_type / lifecycle / registry）
│   │   ├── security/                # 加密模块
│   │   ├── util/                    # 工具模块
│   │   └── test.rs                  # 测试辅助套件
│   └── Cargo.toml
├── index.html
├── package.json
├── vite.config.ts
└── tsconfig.json
```

后端各业务子模块内部统一按 `command`（Tauri 接口暴露与参数校验）/ `service`（业务逻辑）/ `dao`（SQL 封装）/ `entity` / `vo` 分层。

---

## 数据存储

数据目录用于存放应用程序配置、用户数据库元数据以及所有用户数据库。每个用户数据库拥有独立的子目录，其中 `user_database.sqlite` 以加密形式存储画布、节点、边、字段、模板、字典、日志等核心数据，`attachment/` 目录则存放加密的附件文件。

在 Windows 下，数据目录一般为 `%APPDATA%\saya\i-net\data`，内部结构如下：

```text
<data_dir>/
├── preference.sqlite                      # 用户偏好（语言、主题、剪贴板等）
├── metadata.sqlite                        # 用户数据库元数据（注册信息、归档状态等）
├── logs/                                  # 按日滚动的日志文件
└── user_database_set/<user_uuid>/
    ├── user_database.sqlite               # 加密的用户数据库
    └── attachment/<attachment_uuid>.bin   # 加密的附件文件
```

整个数据目录（除 `logs/` 外）可被打包为一个 `.ibackup` 文件用于备份与还原，详见下节。

---

## 数据备份与还原

### 备份文件格式

`.ibackup` 文件采用自定义二进制格式，由三段拼接而成：

```text
+--------------------+--------------------------+--------------------+
| Header (64B)       | Shard 校验和表（变长） | Shard 区（变长）   |
+--------------------+--------------------------+--------------------+
```

- **Header**：固定 64 字节，前 8 字节 magic `"IBACKUP\0"` 防误识别，再依次记录格式版本（当前 v1）、原始字节长度、shard 划分参数、冗余比例与原始字节流 SHA-256。
- **Shard 校验和表**：N+M 条 32 字节 SHA-256，用于还原时定位坏块并触发 RS 重建。校验和表前置使备份文件尾部仅含 shard 数据：写入中断造成的尾部缺失只损失 shard，校验和表始终完整可读。
- **Shard 区**：原始数据经 Reed-Solomon（GF(2^8)）编码后产生的 N 个数据 shard 与 M 个校验 shard，等长排列；任意至多 M 个 shard 损坏或缺失都能被恢复。

### 备份流程

1. 触发 `preference_save` 与 `metadata_save`，避免漏写最近修改。
2. 递归遍历数据目录（跳过 `logs/` 与符号链接），按 tar 格式生成字节流。
3. 按用户设定的冗余比例（默认 5%）自适应决定 (N, M, shard_size) 并完成 Reed-Solomon 编码。
4. 按 `Header | Shard 校验和表 | Shard 区` 组装写入用户选定的目标文件；后端会强制要求目标路径位于数据目录之外，避免覆盖自身数据。

### 还原流程

1. **校验探测（probe）**：仅校验 Header 与 shard SHA-256，返回是否可还原、损坏 shard 数等结构化结论，不修改任何数据；尾部截断的备份按缺失 shard 计入损坏数，缺失不超过冗余容量时仍判定为可还原。
2. **确认还原（restore）**：执行完整还原 —— 读 Header → 校验 shard（shard 区容错读取，尾部截断的 shard 按缺失处理，交给 RS 重建）→ 必要时 RS 重建 → 解压到系统 temp 目录下的 `inet-restore-<pid>-<ts>/` → 清空数据目录（保留 `logs/`）→ 移动临时目录到数据目录（跨设备时 fallback 到 copy+remove）。临时目录由 RAII 守卫清理，任何错误路径（含解压失败、panic）都不残留。
3. **内存连接刷新（reclaim）**：触发 `reclaim_preference` / `reclaim_metadata` / `reclaim_user_database` 让各业务模块重新持有磁盘文件所有权，避免关闭应用时旧内存覆盖还原结果。
4. 还原完成后用户在首页对话框中点击「完成」按钮即可刷新页面。

---

## 国际化

- 全局文案：位于 `src/i18n/<模块>/<locale>.json`，由 `@intlify/unplugin-vue-i18n` 自动收集并按 locale 合并；每个模块的文案挂在与模块同名的顶级键下。
- 组件级文案：通过 `<i18n>` 单文件标签管理，键名使用 kebab-case。
- 当前支持：`zh-CN`、`en-US`。

---

## 开发环境

- Node.js v24.13.1
- pnpm 11.9.0
- Rust 1.95.0

## 常用命令

```powershell
pnpm install        # 安装前端依赖
pnpm dev            # 启动前端开发服务器
pnpm build          # 前端构建（含 vue-tsc 类型检查）
pnpm test           # 前端单元测试（Vitest）
cargo check         # 后端编译检查（在 src-tauri 目录下执行）
cargo test          # 后端单元测试（在 src-tauri 目录下执行）
pnpm tauri dev      # 以 dev 模式启动应用
pnpm tauri build    # 以 release 模式构建发布包
```

---

## 贡献指南

欢迎提交 Issue 与 Pull Request。在编写代码前，请先阅读 `.agents` 目录下的规范文档与前后端源码地图。
