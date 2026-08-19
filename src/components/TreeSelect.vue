<!--
  通用树形单选下拉组件。

  将 VTreeview 置于 VMenu 中，通过只读 VTextField 激活下拉。
  通过 item-title / item-value / item-children 适配任意形态的树形数据，
  仅叶子节点可选中，选中值可通过清除按钮置空。
  readonly 时不允许打开下拉与清除，仅展示当前选中项文本。
-->
<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { t } from "@/i18n";
import { useMenuDismiss } from "@/composables/use-menu-dismiss";

/** item 属性取值键：属性路径（支持点号嵌套）或取值函数。 */
type ItemKey = string | ((item: unknown) => unknown);

const props = withDefaults(defineProps<{
  modelValue: string | null;
  /** 任意形态的树形条目列表。 */
  items: unknown[];
  /** 显示文本取值键，缺省为 "title"。 */
  itemTitle?: ItemKey;
  /** 唯一标识取值键，缺省为 "value"。 */
  itemValue?: ItemKey;
  /** 子节点列表取值键，缺省为 "children"。 */
  itemChildren?: ItemKey;
  label?: string;
  errorMessages?: string | string[];
  clearable?: boolean;
  /** 只读模式：仅展示选中项文本，不打开下拉、不可清除。 */
  readonly?: boolean;
}>(), {
  label: "",
  clearable: true,
  readonly: false,
});

const emit = defineEmits<{
  "update:modelValue": [value: string | null];
}>();

const menuOpen = ref(false);
useMenuDismiss(menuOpen, ".tree-select-popper");

const activatorEl = ref<HTMLElement>();
const popperMinWidth = ref("0px");

watch(menuOpen, (open) => {
  if (open && activatorEl.value) {
    popperMinWidth.value = `${activatorEl.value.getBoundingClientRect().width}px`;
  }
});

/**
 * 按取值键解析条目的属性值。
 * @param item 原始条目
 * @param key 属性路径或取值函数，缺省时使用 defaultKey
 * @param defaultKey 默认属性名
 * @returns 解析出的属性值，路径中断时返回 undefined
 */
function resolveItemProp(
  item: unknown,
  key: ItemKey | undefined,
  defaultKey: string,
): unknown {
  if (typeof key === "function") return key(item);
  const path = key ?? defaultKey;
  let cur: unknown = item;
  for (const segment of path.split(".")) {
    if (cur === null || typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[segment];
  }
  return cur;
}

/**
 * 取条目的显示文本。
 * @param item 原始条目
 * @returns 显示文本，缺失时返回空串
 */
function titleOf(item: unknown): string {
  const v = resolveItemProp(item, props.itemTitle, "title");
  return v == null ? "" : String(v);
}

/**
 * 取条目的唯一标识。
 * @param item 原始条目
 * @returns 唯一标识，缺失时返回空串
 */
function valueOf(item: unknown): string {
  const v = resolveItemProp(item, props.itemValue, "value");
  return v == null ? "" : String(v);
}

/**
 * 取条目的子节点列表。
 * @param item 原始条目
 * @returns 子节点列表，缺失或非数组时返回空数组
 */
function childrenOf(item: unknown): unknown[] {
  const v = resolveItemProp(item, props.itemChildren, "children");
  return Array.isArray(v) ? v : [];
}

/**
 * 判断条目是否可选中（仅叶子节点可选）。
 * @param item 原始条目
 * @returns 是否可选中
 */
function isSelectable(item: unknown): boolean {
  return childrenOf(item).length === 0;
}

/**
 * 在 items 中深度优先搜索指定 value 的节点。
 * @param items 树形条目列表
 * @param targetValue 目标 value
 * @returns 找到的节点，未找到返回 null
 */
function findNode(items: unknown[], targetValue: string): unknown | null {
  for (const item of items) {
    if (valueOf(item) === targetValue) return item;
    const found = findNode(childrenOf(item), targetValue);
    if (found) return found;
  }
  return null;
}

const selectedTitle = computed(() => {
  if (props.modelValue === null || props.modelValue === "") return "";
  const node = findNode(props.items, props.modelValue);
  return node === null ? "" : titleOf(node);
});

const activatedItems = ref<string[]>([]);

watch(() => props.modelValue, (val) => {
  activatedItems.value = val ? [val] : [];
}, { immediate: true });

/**
 * 处理 VTreeview 的 update:activated 事件。
 * 归一化激活值为数组，取最后一个元素，若对应节点可选则 emit 并关闭菜单，
 * 否则重置 activatedItems 忽略本次点击。
 * @param val VTreeview 激活回调参数
 */
function onActivated(val: unknown): void {
  const arr: string[] = Array.isArray(val) ? val : [val];
  const v = arr[arr.length - 1];
  if (v === undefined) return;
  const node = findNode(props.items, v);
  if (node !== null && isSelectable(node)) {
    emit("update:modelValue", v);
    menuOpen.value = false;
  } else {
    activatedItems.value = props.modelValue ? [props.modelValue] : [];
  }
}
</script>

<template>
  <VMenu v-model="menuOpen" :close-on-content-click="false">
    <template #activator="{ props: menuProps }">
      <div ref="activatorEl">
        <VTextField
          :model-value="selectedTitle"
          readonly
          variant="outlined"
          density="compact"
          hide-details="auto"
          :append-inner-icon="readonly ? undefined : 'mdi-menu-down'"
          :clearable="clearable && !readonly && modelValue !== null"
          :label="label"
          :error-messages="errorMessages"
          v-bind="readonly ? {} : menuProps"
          @click:clear="emit('update:modelValue', null)"
        />
      </div>
    </template>
    <VCard class="tree-select-popper" :style="{ minWidth: popperMinWidth }">
      <VTreeview
        v-if="items.length > 0"
        :items="items"
        :item-title="itemTitle"
        :item-value="itemValue"
        :item-children="itemChildren"
        activatable
        active-strategy="single-independent"
        :activated="activatedItems"
        density="compact"
        @update:activated="onActivated"
      >
        <template #title="{ title, internalItem }">
          <template v-if="isSelectable(internalItem.raw)">
            {{ title }}
          </template>
          <template v-else>
            <span class="text-disabled">{{ title }}</span>
          </template>
        </template>
      </VTreeview>
      <div v-else class="text-body-2 text-disabled pa-3 text-center">
        {{ t("common.tree-select.no-data") }}
      </div>
    </VCard>
  </VMenu>
</template>

<style lang="scss" scoped>
.tree-select-popper {
  display: inline-block;
  background: rgb(var(--v-theme-surface));
  border-radius: 0.5rem;
  box-shadow: 0 0.5rem 1.5rem rgba(0, 0, 0, 0.25);
  max-height: 15rem;
  overflow-y: auto;
}
</style>
