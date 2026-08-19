<!--
  删除用户数据库确认对话框。

  要求输入数据库名称与密码以确认永久删除，防止误操作。
-->
<script setup lang="ts">
import { computed, ref } from "vue";
import { t } from "@/i18n";
import PasswordField from "@/components/PasswordField.vue";

const emit = defineEmits<{
  /** 确认删除 */
  confirm: [id: string, name: string, password: string];
}>();

/** 对话框显示状态 */
const dialog = ref(false);
/** 用户输入的数据库名称 */
const nameInput = ref("");
/** 用户输入的密码 */
const passwordInput = ref("");
/** 要删除的数据库 id */
const targetId = ref("");
/** 要删除的数据库名称 */
const targetName = ref("");

/** 是否可以确认删除 */
const canConfirm = computed(
  () =>
    nameInput.value.length > 0 &&
    nameInput.value === targetName.value &&
    passwordInput.value.length > 0,
);

/**
 * 打开删除确认对话框。
 * @param id 数据库 id
 * @param name 数据库名称
 */
function open(id: string, name: string) {
  targetId.value = id;
  targetName.value = name;
  nameInput.value = "";
  passwordInput.value = "";
  dialog.value = true;
}

/** 关闭对话框 */
function close() {
  dialog.value = false;
}

/** 触发确认删除事件 */
function onConfirm() {
  emit("confirm", targetId.value, targetName.value, passwordInput.value);
  close();
}

defineExpose({ open, close });
</script>

<template>
  <VDialog v-model="dialog" max-width="500" persistent>
    <VCard>
      <VCardTitle>{{ t("home.delete-confirm.title") }}</VCardTitle>
      <VCardText>
        <p class="body-text">
          {{ t("home.delete-confirm.body", { name: targetName }) }}
        </p>
        <VTextField
          v-model="nameInput"
          :label="t('home.delete-confirm.name-label')"
          :placeholder="targetName"
          variant="outlined"
          hide-details="auto"
        />
        <PasswordField
          v-model="passwordInput"
          :label="t('home.delete-confirm.password-label')"
          class="password-field"
          @keydown.enter.prevent="canConfirm && onConfirm()"
        />
      </VCardText>
      <VCardActions>
        <VSpacer />
        <VBtn variant="text" @click="close">
          {{ t("home.delete-confirm.cancel") }}
        </VBtn>
        <VBtn
          color="error"
          variant="flat"
          :disabled="!canConfirm"
          @click="onConfirm"
        >
          {{ t("home.delete-confirm.confirm") }}
        </VBtn>
      </VCardActions>
    </VCard>
  </VDialog>
</template>

<style lang="scss" scoped>
.body-text {
  margin-bottom: 1rem;
}

.password-field {
  margin-top: 1rem;
}
</style>
