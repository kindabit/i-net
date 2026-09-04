<!--
  编辑节点对话框。

  通过 defineExpose 的 open() 以 Promise 形式获取编辑结果：
  确认返回 { title, subTitle }（trim 后的新值），取消返回 null
  （对话框为 persistent，Esc 与点击遮罩均不关闭，Esc 语义保留给对话框内部控件，如字段名编辑的取消）。
  支持字段编辑与保存为模板。
  支持以只读形式查看节点（open 的 options.readonly）：只读模式下隐藏编辑类控件，
  只保留关闭按钮，因此永远不会 resolve 出非 null 值。
-->
<script setup lang="ts">
import { reactive, ref, watch } from "vue";
import { t } from "@/i18n";
import {
  userDatabaseNodeModify,
  userDatabaseNodeFieldGet,
  userDatabaseNodeFieldSet,
  userDatabaseTemplateCreateFromNode,
} from "@/api";
import { snackbarErrorCode, snackbarText } from "@/composables/use-snackbar";
import { useNodeFieldList } from "@/composables/use-node-field-list";
import NodeField from "@/components/NodeField.vue";
import NameInputDialog from "@/components/NameInputDialog.vue";

/** 对话框显示状态 */
const dialog = ref(false);
/** 当前编辑的节点 id */
const nodeId = ref("");
/** 编辑草稿 */
const draft = reactive({
  title: "",
  subTitle: "",
});
/** 标题错误提示 */
const titleError = ref("");
/** 只读模式（用于影子节点只读查看） */
const readonly = ref(false);
/** 提交中 */
const submitting = ref(false);
/** 初始标题（用于比较是否有变化） */
const originalTitle = ref("");
/** 初始副标题（用于比较是否有变化） */
const originalSubTitle = ref("");
/** 数据加载中 */
const loading = ref(false);
/** 保存为模板进行中 */
const savingTemplate = ref(false);
/** 拖拽中记录的起点 uid */
const draggingUid = ref<number | null>(null);
/** 等待 Promise 结算的 resolve */
let resolveOpen: ((value: { title: string; subTitle: string } | null) => void) | null = null;

const fieldList = useNodeFieldList();
const nameInputDialogRef = ref<InstanceType<typeof NameInputDialog>>();

/**
 * 打开对话框编辑或查看节点。
 * @param node 节点信息（含 id、标题、副标题）
 * @param options 可选配置；readonly 为 true 时以只读模式展示（用于影子节点查看），只读模式不会 resolve 出非 null 值
 * @returns 编辑模式确认返回 trim 后的新值，取消/关闭返回 null；只读模式永远返回 null
 */
function open(
  node: { id: string; title: string; subTitle: string },
  options?: { readonly?: boolean },
): Promise<{ title: string; subTitle: string } | null> {
  settle(null);
  readonly.value = options?.readonly ?? false;
  nodeId.value = node.id;
  draft.title = node.title;
  draft.subTitle = node.subTitle;
  originalTitle.value = node.title;
  originalSubTitle.value = node.subTitle;
  titleError.value = "";
  submitting.value = false;
  draggingUid.value = null;
  loading.value = true;
  dialog.value = true;
  loadData();
  return new Promise((resolve) => {
    resolveOpen = resolve;
  });
}

async function loadData() {
  try {
    const fields = await userDatabaseNodeFieldGet(nodeId.value);
    fieldList.loadFromNodeFields(fields);
  } catch (e) {
    snackbarErrorCode(e);
    settle(null);
    dialog.value = false;
  } finally {
    loading.value = false;
  }
}

/**
 * 结算等待中的 Promise。
 * @param value 编辑结果
 */
function settle(value: { title: string; subTitle: string } | null) {
  resolveOpen?.(value);
  resolveOpen = null;
}

function onDragStart(uid: number) {
  draggingUid.value = uid;
}

function onDropOn(uid: number) {
  if (draggingUid.value !== null) {
    fieldList.moveRow(draggingUid.value, uid);
    draggingUid.value = null;
  }
}

/**
 * 校验标题与字段列表，通过后将标题/副标题与字段变更持久化到后端，并同步原始标题快照。
 * 校验失败时写入对应错误信息。
 * @returns 保存成功返回 trim 后的标题与副标题，校验失败返回 null
 */
async function saveNodeChanges(): Promise<{ title: string; subTitle: string } | null> {
  titleError.value = "";
  const title = draft.title.trim();
  const subTitle = draft.subTitle.trim();
  if (title === "") {
    titleError.value = t("database.canvas.edit-node-title-required");
    return null;
  }
  if (!fieldList.validate()) return null;
  if (title !== originalTitle.value || subTitle !== originalSubTitle.value) {
    await userDatabaseNodeModify(nodeId.value, title, subTitle);
    originalTitle.value = title;
    originalSubTitle.value = subTitle;
  }
  if (fieldList.isDirty()) {
    await userDatabaseNodeFieldSet(nodeId.value, fieldList.toNodeFieldVOs());
  }
  return { title, subTitle };
}

/** 确认编辑并提交 */
async function onConfirm() {
  submitting.value = true;
  try {
    const result = await saveNodeChanges();
    if (result === null) return;
    settle(result);
    dialog.value = false;
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    submitting.value = false;
  }
}

/** 保存为模板 */
async function saveAsTemplate() {
  savingTemplate.value = true;
  try {
    const result = await saveNodeChanges();
    if (result === null) return;
    const templateName = await nameInputDialogRef.value?.open({
      title: t("database.canvas.save-as-template-title"),
      label: t("database.canvas.template-name-label"),
      confirmText: t("database.canvas.edit-node-save-as-template"),
    });
    if (!templateName) return;
    await userDatabaseTemplateCreateFromNode(nodeId.value, templateName);
    snackbarText(t("database.canvas.template-created"), "success");
  } catch (e) {
    snackbarErrorCode(e);
  } finally {
    savingTemplate.value = false;
  }
}

// 任何途径的关闭（取消按钮、提交完成）都按取消/成功结算（对话框为 persistent，Esc 与遮罩不会触发关闭）
watch(dialog, (value) => {
  if (!value) settle(null);
});

defineExpose({ open });
</script>

<template>
  <VDialog v-model="dialog" max-width="54rem" persistent>
    <!-- persistent：Esc 与点击遮罩均不关闭本对话框，只能通过对话框内的按钮关闭，
         避免与对话框内部控件的 Esc 语义（如字段名编辑的 Esc 取消）冲突，也防止误触遮罩丢失编辑内容。 -->
    <VCard>
      <VCardTitle>{{ readonly ? t("database.canvas.view-node-readonly") : t("database.canvas.edit-node") }}</VCardTitle>
      <VCardText :class="{ 'fields-scroll': !loading }">
        <div v-if="loading" class="d-flex justify-center py-8">
          <VProgressCircular indeterminate color="primary" />
        </div>
        <template v-else>
          <v-row>
            <v-col cols="6">
              <VTextField
                v-model="draft.title"
                :label="t('database.canvas.edit-node-title-label')"
                :error-messages="titleError"
                :readonly="readonly"
                variant="outlined"
                density="comfortable"
                class="mb-4"
              />
            </v-col>
            <v-col cols="6">
              <VTextField
                v-model="draft.subTitle"
                :label="t('database.canvas.edit-node-subtitle-label')"
                :readonly="readonly"
                variant="outlined"
                density="comfortable"
              />
            </v-col>
          </v-row>
          <VDivider class="my-4" />
          <div v-for="row in fieldList.rows.value" :key="row.uid" class="mb-4">
            <NodeField
              :row="row"
              :readonly="readonly"
              :errors="fieldList.errors.value"
              @remove="fieldList.removeRow"
              @drag-start="onDragStart"
              @drop-on="onDropOn"
            />
          </div>
          <VBtn
            v-if="!readonly"
            variant="text"
            prepend-icon="mdi-plus"
            @click="fieldList.addRow()"
          >
            {{ t("database.field.add-field") }}
          </VBtn>
        </template>
      </VCardText>
      <VCardActions class="justify-end ga-2">
        <template v-if="!readonly">
          <VBtn
            v-if="!loading"
            class="mr-auto"
            :loading="savingTemplate"
            :disabled="submitting"
            variant="text"
            @click="saveAsTemplate"
          >
            {{ t("database.canvas.edit-node-save-as-template") }}
          </VBtn>
          <VBtn variant="text" :disabled="submitting || savingTemplate" @click="dialog = false">
            {{ t("common.cancel") }}
          </VBtn>
          <VBtn color="primary" variant="flat" :loading="submitting" :disabled="savingTemplate" @click="onConfirm">
            {{ t("common.confirm") }}
          </VBtn>
        </template>
        <VBtn v-else variant="text" @click="dialog = false">
          {{ t("common.cancel") }}
        </VBtn>
      </VCardActions>
    </VCard>
    <NameInputDialog ref="nameInputDialogRef" />
  </VDialog>
</template>

<style lang="scss" scoped>
.mb-4 {
  margin-bottom: 1rem;
}

.my-4 {
  margin-top: 1rem;
  margin-bottom: 1rem;
}

.py-8 {
  padding-top: 2rem;
  padding-bottom: 2rem;
}

.fields-scroll {
  max-height: 60vh;
  overflow-y: auto;
}
</style>
