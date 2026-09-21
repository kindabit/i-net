# 共享最小 fixture（base）

各测试用例在启动应用前，把本目录下的 `data\` 复制到自己的 `output\data` 作为起始用户数据目录。

## 内容与凭据

- 数据库名称：`zz-e2e-base`
- 数据库密码：`e2e-password`
- 数据内容：根画布；根画布内的数据节点「账号 A」与「备注 B」；一条从「账号 A」指向「备注 B」的边
- registry `lastScene` 指向根画布（构建脚本最后停留在根画布并保存），因此解锁后**直接进入根画布**而非画布宇宙
- 界面语言：继承系统语言（测试机为中文；偏好在 fixture 中未固定，用例脚本按中文文案断言，必要时先通过「切换语言」菜单确保为中文）

## 使用方式

在用例脚本中使用 `prepareDataDir(dataDir, "base")`（见 `e2e\script\lib\fixtures.js`）：
清空目标数据目录后，将 `_fixtures\base\data` 整体复制过去。

## 重建方式

```powershell
node e2e\script\_fixtures\build.js
```

脚本会临时启动应用（运行数据目录位于 `.temp\e2e-fixture-build`），通过 UI 操作重新构造
数据库与内容，核对持久化后复制到本目录；重建过程会覆盖本目录下已有的 `data\`。
