<script setup lang="ts">
/**
 * 两列键值编辑表格：用于端口映射、环境变量、路径映射等「成对条目」场景。
 * 支持添加、就地修改、删除条目；条目字段名由 columns 的 key 决定。
 */

import { computed, reactive, ref } from 'vue';
import { Button, Form, FormItem, Input, Modal, Table, TableColumn } from '@arco-design/web-vue';

export interface PairColumn {
  /** 条目对象上的字段名。 */
  key: string;
  title: string;
  placeholder?: string;
}

export type PairRow = Record<string, string>;

const props = withDefaults(
  defineProps<{
    columns: [PairColumn, PairColumn];
    rows: PairRow[];
    disabled?: boolean;
    /** 校验函数：返回错误文案，null 表示通过。 */
    validate?: (row: PairRow) => string | null;
  }>(),
  { disabled: false, validate: undefined },
);

const emit = defineEmits<{ change: [rows: PairRow[]] }>();

const editorVisible = ref(false);
const editingIndex = ref<number | null>(null);
const draft = reactive<PairRow>({});
const error = ref('');

const isEditing = computed(() => editingIndex.value !== null);

function openEditor(index: number | null): void {
  editingIndex.value = index;
  error.value = '';
  for (const col of props.columns) {
    draft[col.key] = index !== null ? (props.rows[index]?.[col.key] ?? '') : '';
  }
  editorVisible.value = true;
}

/** 保存校验；返回 false 阻止弹窗关闭（校验失败时错误提示保持可见）。 */
function save(): boolean {
  for (const col of props.columns) {
    if (!(draft[col.key] ?? '').trim()) {
      error.value = `请填写「${col.title}」`;
      return false;
    }
  }
  const row: PairRow = {};
  for (const col of props.columns) row[col.key] = (draft[col.key] ?? '').trim();
  const err = props.validate?.(row) ?? null;
  if (err) {
    error.value = err;
    return false;
  }
  const next = [...props.rows];
  if (editingIndex.value !== null) next[editingIndex.value] = row;
  else next.push(row);
  emit('change', next);
  error.value = '';
  return true;
}

function remove(index: number): void {
  emit(
    'change',
    props.rows.filter((_, i) => i !== index),
  );
}
</script>

<template>
  <div class="pair-table">
    <Table :data="rows" :pagination="false" size="small" :scroll="{ x: 520 }">
      <template #columns>
        <TableColumn v-for="col in columns" :key="col.key" :title="col.title">
          <template #cell="{ record }">
            <span class="mono">{{ record[col.key] }}</span>
          </template>
        </TableColumn>
        <TableColumn v-if="!disabled" title="操作" :width="110" fixed="right">
          <template #cell="{ rowIndex }">
            <Button size="mini" type="text" @click="openEditor(rowIndex)">编辑</Button>
            <Button size="mini" type="text" status="danger" @click="remove(rowIndex)">删除</Button>
          </template>
        </TableColumn>
      </template>
    </Table>
    <Button v-if="!disabled" size="small" long style="margin-top: 8px" @click="openEditor(null)">添加条目</Button>

    <!-- on-before-ok 返回 false 阻止关闭，校验失败时错误提示保持可见 -->
    <Modal v-model:visible="editorVisible" :title="isEditing ? '编辑条目' : '添加条目'" ok-text="保存" :on-before-ok="save">
      <Form :model="draft" layout="vertical">
        <FormItem v-for="col in columns" :key="col.key" :label="col.title" required>
          <Input v-model="draft[col.key]" :placeholder="col.placeholder" class="mono" />
        </FormItem>
        <p v-if="error" class="pair-table__error">{{ error }}</p>
      </Form>
    </Modal>
  </div>
</template>

<style scoped>
.pair-table__error {
  margin: 0;
  color: var(--mcnp-color-danger);
  font-size: 12px;
}
</style>
