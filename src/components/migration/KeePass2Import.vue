<!--
  KeePass 2.0 数据导入对话框。

  从 KeePass 2.0 数据库（.kdbx）导入数据的对话框：用户选择数据库文件、输入其
  Master Password 并选择导入方式，确认后由前端读取文件字节、解密解析并构造好
  待导入数据（字段名国际化随当前语言），再调用后端聚合导入接口写库。
  导入方式二选一：
  - 导入单个画布：全部 group 与数据条目组织为一棵节点树（含表示数据库本身的根节点、
    父子边与树形布局坐标），后端在根画布下创建一个画布数据节点，其引用的新画布
    容纳全部节点；
  - 按分组生成多个画布：每个 group 一张画布（含根 group，挂到根画布；子 group 画布
    挂到父 group 画布），group 的 entry 与引用子 group 画布的画布数据节点以矩阵布局
    平铺在画布内，整棵画布树在画布宇宙中按 group 层级以树形布局呈现。
  导入成功后 emit "success" 事件并携带顶层新画布 id，父组件借此跳转至新画布。
-->
<script setup lang="ts">
import { ref, watch } from "vue";
import { t } from "@/i18n";
import {
  userDatabaseCanvasList,
  userDatabaseMigrationImportKeepass2,
  userDatabaseMigrationImportKeepass2Canvases,
  userDatabaseMigrationPickFile,
  userDatabaseMigrationReadFile,
  userDatabaseNodeList,
} from "@/api";
import type { ImportedCanvasVO } from "@/api-types";
import type { ImportedTree } from "@/migration/keepass2";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import { triggerFatalError } from "@/composables/use-fatal-error";
import PasswordField from "@/components/PasswordField.vue";
import {
  layoutImportedCanvases,
  layoutImportedTree,
  parseKeepass2Database,
  parseKeepass2DatabaseCanvases,
  type Keepass2FieldText,
} from "@/migration/keepass2";

const emit = defineEmits<{
  /** 导入成功后通知父组件跳转至新画布，携带顶层新画布 id */
  success: [canvasId: string];
}>();

/** 对话框显示状态 */
const dialog = ref(false);
/** KeePass 2.0 数据库文件路径 */
const filePath = ref("");
/** KeePass 2.0 数据库的 Master Password */
const masterPassword = ref("");
/** 导入方式："single" 导入单个画布（树形布局）；"canvases" 按分组生成多个画布（画布内矩阵布局） */
const importMode = ref<"single" | "canvases">("single");
/** 文件路径校验错误文案 */
const fileError = ref("");
/** 密码校验错误文案 */
const passwordError = ref("");
/** 文件选择进行中（系统对话框打开期间） */
const picking = ref(false);
/** 导入进行中 */
const importing = ref(false);

/**
 * 打开对话框并重置表单状态（文件路径空、密码空、导入方式单画布、校验错误清空）。
 */
function open() {
  filePath.value = "";
  masterPassword.value = "";
  importMode.value = "single";
  fileError.value = "";
  passwordError.value = "";
  picking.value = false;
  importing.value = false;
  dialog.value = true;
}

// 密码输入变化时清除密码校验错误，让用户修正后重试
watch(masterPassword, () => {
  passwordError.value = "";
});

/**
 * 触发"选择文件"按钮：调后端弹出系统文件选择对话框；
 * 用户在系统对话框中取消时保持原状，选中时回填路径并清除路径校验错误。
 */
async function pickFile() {
  picking.value = true;
  try {
    const picked = await userDatabaseMigrationPickFile();
    if (picked === null) return;
    filePath.value = picked;
    fileError.value = "";
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    picking.value = false;
  }
}

/**
 * 取文件路径的文件名 stem（basename 去扩展名）：
 * Windows（\）与 POSIX（/）分隔符都处理；扩展名分隔点必须位于非首字符位置，
 * 避免把 ".kdbx" 这类以点开头的整名剥成空串。
 * @param path 文件路径
 * @returns 文件名 stem，路径不含文件名（如以分隔符结尾）时返回空串
 */
function fileNameStem(path: string): string {
  const segments = path.split(/[\\/]/);
  const basename = segments[segments.length - 1];
  const dotIndex = basename.lastIndexOf(".");
  return dotIndex > 0 ? basename.slice(0, dotIndex) : basename;
}

/**
 * 解析 KeePass 数据库失败的统一提示：失败值为 Keepass2ParseError
 * （"incorrect-password" 或 "invalid-file"），非 ErrorCode，用本地化文案直接提示。
 * @param error parseKeepass2Database / parseKeepass2DatabaseCanvases 抛出的值
 */
function snackbarParseError(error: unknown) {
  snackbarText(
    error === "incorrect-password"
      ? t("database.migration.incorrect-password")
      : t("database.migration.invalid-file"),
    "error",
  );
}

/**
 * 查询画布列表并找出根画布（单画布路线中画布数据节点的落点画布、多画布路线中
 * 整棵画布树的挂点）。根画布必然位于正常画布列表中；includeDeleted 为真时把
 * 已逻辑删除的画布并入返回列表，供多画布路线的空闲点搜索避让。找不到根画布说明
 * 数据结构损坏（已打开的用户数据库必有根画布），走受控崩溃而非静默容错
 * （见 use-fatal-error 与 FatalErrorDialog）。
 * @param includeDeleted 是否把已逻辑删除的画布并入返回的列表（多画布路线的空闲点搜索需要避让全部画布）
 * @returns 根画布与画布列表；根画布不存在时触发受控崩溃并返回 null
 */
async function queryRootWithCanvases(includeDeleted: boolean) {
  // 后端 canvas_list 的 deleted 参数是二选一语义（false 返回正常画布、true 返回已逻辑删除的画布），
  // 根画布必在正常画布列表中，已逻辑删除的列表仅在需要避让时另外并入。
  const canvases = await userDatabaseCanvasList(false);
  const root = canvases.find((canvas) => canvas.parent_id === null);
  if (root === undefined) {
    // 不变量破坏：已打开的用户数据库必有根画布。
    // data.id 与后端同场景返回的 NoCanvasWithSuchId { id: "root" } 保持一致，
    // 供 FatalErrorDialog 的错误文案插值。
    triggerFatalError({ variant: "NoCanvasWithSuchId", data: { id: "root" } });
    return null;
  }
  if (includeDeleted) {
    canvases.push(...(await userDatabaseCanvasList(true)));
  }
  return { root, canvases };
}

/**
 * 单画布路线导入：前端解密解析为树形节点数据（根节点表示数据库本身）→
 * 树形布局 → 后端聚合导入，在根画布下创建一个容纳全部节点的新画布。
 * @param bytes 数据库文件字节
 * @param fieldText 字段名文案集合
 * @returns 新画布 id；解析失败或文件路径无法推导画布名（均已提示用户）、
 * 根画布缺失（已触发受控崩溃）时返回 null
 */
async function importSingleCanvas(
  bytes: Uint8Array,
  fieldText: Keepass2FieldText,
): Promise<string | null> {
  // 前端解密解析为树形节点数据；字段名文案取当前语言，导入后即用户所见字段名
  let tree: ImportedTree;
  try {
    tree = await parseKeepass2Database(bytes, masterPassword.value, fieldText);
  } catch (error) {
    snackbarParseError(error);
    return null;
  }

  // 画布名取文件名 stem
  const canvasName = fileNameStem(filePath.value);
  if (canvasName === "") {
    // 防御分支：文件路径不含文件名（理论上后端文件选择对话框不会返回这种路径），
    // 无法推导画布名，按无效文件提示并保持对话框打开供用户重选。
    snackbarText(t("database.migration.invalid-file"), "error");
    return null;
  }
  // 根节点表示数据库本身：KeePass 根 group 名为空时以文件名兜底，
  // 保证根节点始终有可辨识的标题。
  if (tree.nodes[0].title === "") {
    tree.nodes[0].title = canvasName;
  }

  const query = await queryRootWithCanvases(false);
  if (query === null) return null;
  const { root } = query;

  // 画布数据节点 y = 根画布现有节点的最大 y（无节点取 -240）+ 240，x 恒 0
  const existingNodes = await userDatabaseNodeList(root.id, false);
  const canvasNodeY = existingNodes.reduce((max, node) => Math.max(max, node.y), -240) + 240;

  // 就地写入树形布局坐标
  layoutImportedTree(tree);

  // 后端聚合导入（失败抛 ErrorCode，由调用方统一反馈），返回新画布 id
  return await userDatabaseMigrationImportKeepass2(
    canvasName,
    0,
    canvasNodeY,
    tree.nodes,
    tree.edges,
  );
}

/**
 * 多画布路线导入：前端解密解析为画布列表（每个 group 一张画布）→ 两级布局
 * （画布宇宙中的树形布局 + 画布内的矩阵布局）→ 后端聚合导入，在根画布下
 * 创建整棵画布子树。
 * @param bytes 数据库文件字节
 * @param fieldText 字段名文案集合
 * @returns 根 group 画布的 id；解析失败或文件路径无法推导画布名（均已提示用户）、
 * 根画布缺失（已触发受控崩溃）时返回 null
 */
async function importCanvases(
  bytes: Uint8Array,
  fieldText: Keepass2FieldText,
): Promise<string | null> {
  // 前端解密解析为画布列表；字段名文案取当前语言，导入后即用户所见字段名
  let importedCanvases: ImportedCanvasVO[];
  try {
    importedCanvases = await parseKeepass2DatabaseCanvases(
      bytes,
      masterPassword.value,
      fieldText,
    );
  } catch (error) {
    snackbarParseError(error);
    return null;
  }

  // 顶层画布名取文件名 stem
  const canvasName = fileNameStem(filePath.value);
  if (canvasName === "") {
    // 防御分支：文件路径不含文件名（理论上后端文件选择对话框不会返回这种路径），
    // 无法推导画布名，按无效文件提示并保持对话框打开供用户重选。
    snackbarText(t("database.migration.invalid-file"), "error");
    return null;
  }
  // 名称兜底：KeePass 根 group 名为空时以文件名兜底（与单画布路线的根节点规则一致），
  // 子 group 名为空时以"未命名分组"兜底（画布名称不允许为空，重名由后端自动追加序号）。
  if (importedCanvases[0].name === "") {
    importedCanvases[0].name = canvasName;
  }
  for (let index = 1; index < importedCanvases.length; index++) {
    if (importedCanvases[index].name === "") {
      importedCanvases[index].name = t("database.migration.unnamed-group");
    }
  }

  // 空闲点搜索需要避让全部画布（含已逻辑删除的），与后端环形布局的避让口径一致
  const query = await queryRootWithCanvases(true);
  if (query === null) return null;
  const { root, canvases } = query;

  // 就地写入两级布局坐标（宇宙树形布局 + 画布内矩阵布局）
  layoutImportedCanvases(importedCanvases, root, canvases);

  // 后端聚合导入（失败抛 ErrorCode，由调用方统一反馈），返回根 group 画布的 id
  return await userDatabaseMigrationImportKeepass2Canvases(importedCanvases);
}

/**
 * 触发"确定"按钮：先做前端非空校验（路径、密码），校验通过后按导入方式执行导入：
 * 读文件字节 → 前端解密解析 → 布局 → 后端聚合导入。
 * 成功后关闭对话框并 emit "success"（携带顶层新画布 id），失败保持打开供用户重试。
 */
async function confirmImport() {
  fileError.value = "";
  passwordError.value = "";
  if (!filePath.value) {
    fileError.value = t("database.migration.file-required");
  }
  if (!masterPassword.value) {
    passwordError.value = t("database.migration.password-required");
  }
  if (fileError.value || passwordError.value) return;
  importing.value = true;
  try {
    // a) 读文件字节（后端 command 失败抛 ErrorCode，由末尾 catch 统一反馈）
    const bytes = await userDatabaseMigrationReadFile(filePath.value);

    // b) 字段名文案取当前语言，导入后即用户所见字段名
    const fieldText: Keepass2FieldText = {
      password: t("database.migration.field-password"),
      url: t("database.migration.field-url"),
      notes: t("database.migration.field-notes"),
    };

    // c) 按导入方式执行导入路线（解析失败等已提示用户的情形返回 null，保持对话框打开）
    const newCanvasId =
      importMode.value === "single"
        ? await importSingleCanvas(bytes, fieldText)
        : await importCanvases(bytes, fieldText);
    if (newCanvasId === null) return;

    dialog.value = false;
    snackbarText(t("database.migration.imported"), "success");
    emit("success", newCanvasId);
  } catch (error) {
    snackbarErrorCode(error);
  } finally {
    importing.value = false;
  }
}

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="30rem" :persistent="importing">
    <VCard>
      <VCardTitle>{{ t("database.migration.dialog-title") }}</VCardTitle>
      <VCardText>
        <div class="form-rows">
          <VTextField
            v-model="filePath"
            readonly
            variant="outlined"
            :label="t('database.migration.file-label')"
            :error-messages="fileError"
            hide-details="auto"
          >
            <template #append-inner>
              <VBtn
                variant="text"
                size="small"
                :loading="picking"
                @click="pickFile"
              >
                {{ t("database.migration.pick-file") }}
              </VBtn>
            </template>
          </VTextField>
          <PasswordField
            v-model="masterPassword"
            :label="t('database.migration.password-label')"
            :error-messages="passwordError"
          />
          <VRadioGroup
            v-model="importMode"
            :label="t('database.migration.mode-label')"
            :disabled="importing"
            hide-details="auto"
          >
            <VRadio
              value="single"
              :label="t('database.migration.mode-single')"
            />
            <VRadio
              value="canvases"
              :label="t('database.migration.mode-canvases')"
            />
          </VRadioGroup>
        </div>
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" :disabled="importing" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
        <VBtn
          color="primary"
          variant="flat"
          :loading="importing"
          @click="confirmImport"
        >
          {{ importing ? t("database.migration.importing") : t("common.confirm") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.form-rows {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding-top: 0.5rem;
}
</style>
