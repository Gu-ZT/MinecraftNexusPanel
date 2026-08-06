<script setup lang="ts">
import { computed } from 'vue';

type JsonRecord = Record<string, unknown>;

const props = withDefaults(
  defineProps<{
    label: string;
    value: unknown;
    schema: JsonRecord;
    uiSchema?: JsonRecord;
    depth?: number;
  }>(),
  {
    uiSchema: () => ({}),
    depth: 0,
  },
);

const emit = defineEmits<{
  'update:value': [value: unknown];
}>();

defineOptions({ name: 'ConfigValueEditor' });

const fieldType = computed(() => schemaType(props.schema, props.value));
const properties = computed(() => Object.entries(asRecord(props.schema.properties)));
const objectValue = computed(() => asRecord(props.value));
const arrayValue = computed(() => (Array.isArray(props.value) ? props.value : []));
const enumValues = computed(() => (Array.isArray(props.schema.enum) ? props.schema.enum : []));
const itemSchema = computed(() => (isRecord(props.schema.items) ? props.schema.items : null));
const itemUiSchema = computed(() => asRecord(props.uiSchema.items));
const sensitive = computed(() => props.uiSchema.sensitive === true || props.schema.writeOnly === true);
// 数组增删会改变原文结构；没有统一项 Schema 时宁可保持只读，也不根据样例值猜测类型。
const arrayUnsupportedReason = computed(() => {
  if (fieldType.value !== 'array') {
    return '';
  }
  if (Array.isArray(props.schema.items)) {
    return '元组数组暂不支持编辑';
  }
  if (hasSchemaComposition(props.schema) || (itemSchema.value && hasSchemaComposition(itemSchema.value))) {
    return '包含多种候选结构的数组暂不支持编辑';
  }
  const schema = itemSchema.value;
  if (!schema || Object.keys(schema).length === 0) {
    return '数组项未声明统一 Schema，暂不支持编辑';
  }
  if (arrayValue.value.some((item) => !schemaAcceptsValue(schema, item))) {
    return '数组包含与统一 Schema 不匹配的项目，暂不支持编辑';
  }
  return '';
});
const arrayEditingSupported = computed(() => arrayUnsupportedReason.value === '');

function asRecord(value: unknown): JsonRecord {
  return isRecord(value) ? value : {};
}

function isRecord(value: unknown): value is JsonRecord {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function schemaType(schema: JsonRecord, value: unknown): string {
  const type = schema.type;
  if (Array.isArray(type)) {
    return String(type.find((candidate) => candidate !== 'null') ?? type[0] ?? inferType(value));
  }
  if (typeof type === 'string') {
    return type;
  }
  if (schema.properties) {
    return 'object';
  }
  return inferType(value);
}

function inferType(value: unknown): string {
  if (Array.isArray(value)) {
    return 'array';
  }
  if (value !== null && typeof value === 'object') {
    return 'object';
  }
  if (typeof value === 'number') {
    return Number.isInteger(value) ? 'integer' : 'number';
  }
  return typeof value;
}

function hasSchemaComposition(schema: JsonRecord): boolean {
  const type = schema.type;
  const declaredTypes = Array.isArray(type) ? type.filter((candidate) => candidate !== 'null') : [];
  return declaredTypes.length > 1 || Array.isArray(schema.oneOf) || Array.isArray(schema.anyOf);
}

function schemaAcceptsValue(schema: JsonRecord, value: unknown): boolean {
  const actualType = inferType(value);
  if (Array.isArray(schema.type)) {
    return schema.type.some(
      (candidate) =>
        candidate === actualType ||
        (candidate === 'number' && actualType === 'integer') ||
        (candidate === 'null' && value === null),
    );
  }
  const expectedType = schemaType(schema, value);
  return expectedType === actualType || (expectedType === 'number' && actualType === 'integer');
}

function uiProperty(key: string): JsonRecord {
  return asRecord(asRecord(props.uiSchema.properties)[key]);
}

function updateObject(key: string, value: unknown): void {
  emit('update:value', { ...objectValue.value, [key]: value });
}

function updateArray(index: number, value: unknown): void {
  const next = [...arrayValue.value];
  next[index] = value;
  emit('update:value', next);
}

function removeArrayItem(index: number): void {
  emit(
    'update:value',
    arrayValue.value.filter((_, currentIndex) => currentIndex !== index),
  );
}

function addArrayItem(): void {
  if (!itemSchema.value || !arrayEditingSupported.value) {
    return;
  }
  emit('update:value', [...arrayValue.value, defaultValue(itemSchema.value)]);
}

function defaultValue(schema: JsonRecord): unknown {
  switch (schemaType(schema, null)) {
    case 'object':
      return Object.fromEntries(
        Object.entries(asRecord(schema.properties)).map(([key, childSchema]) => [
          key,
          defaultValue(asRecord(childSchema)),
        ]),
      );
    case 'array':
      return [];
    case 'boolean':
      return false;
    case 'integer':
    case 'number':
      return 0;
    case 'null':
      return null;
    default:
      return Array.isArray(schema.enum) ? schema.enum[0] ?? '' : '';
  }
}

function updateString(event: Event): void {
  emit('update:value', (event.target as HTMLInputElement).value);
}

function updateNumber(event: Event): void {
  const input = event.target as HTMLInputElement;
  emit('update:value', input.value === '' || Number.isNaN(input.valueAsNumber) ? null : input.valueAsNumber);
}

function updateBoolean(event: Event): void {
  emit('update:value', (event.target as HTMLInputElement).checked);
}

function updateEnum(event: Event): void {
  const encoded = (event.target as HTMLSelectElement).value;
  try {
    emit('update:value', JSON.parse(encoded));
  } catch {
    emit('update:value', encoded);
  }
}

function enumValue(value: unknown): string {
  return JSON.stringify(value);
}
</script>

<template>
  <section class="config-field" :class="{ 'config-field-nested': depth > 0 }">
    <div class="config-field-heading">
      <label>{{ label }}</label>
      <code v-if="fieldType !== 'object'">{{ fieldType }}</code>
    </div>

    <div v-if="fieldType === 'object'" class="config-group">
      <ConfigValueEditor
        v-for="[key, childSchema] in properties"
        :key="key"
        :depth="depth + 1"
        :label="key"
        :schema="asRecord(childSchema)"
        :ui-schema="uiProperty(key)"
        :value="objectValue[key]"
        @update:value="updateObject(key, $event)"
      />
      <p v-if="properties.length === 0" class="config-muted">空对象</p>
    </div>

    <div v-else-if="fieldType === 'array' && arrayEditingSupported" class="config-array">
      <div v-for="(item, index) in arrayValue" :key="index" class="config-array-item">
        <ConfigValueEditor
          :depth="depth + 1"
          :label="`条目 ${index + 1}`"
          :schema="itemSchema ?? {}"
          :ui-schema="itemUiSchema"
          :value="item"
          @update:value="updateArray(index, $event)"
        />
        <button class="config-remove" type="button" @click="removeArrayItem(index)">删除</button>
      </div>
      <button class="config-add" type="button" @click="addArrayItem">添加条目</button>
    </div>
    <output v-else-if="fieldType === 'array'" class="config-unsupported">
      {{ arrayUnsupportedReason }}<span v-if="arrayValue.length">（当前 {{ arrayValue.length }} 项）</span>
    </output>

    <select v-else-if="enumValues.length > 0" :value="enumValue(value)" @change="updateEnum">
      <option v-for="option in enumValues" :key="enumValue(option)" :value="enumValue(option)">
        {{ option }}
      </option>
    </select>
    <label v-else-if="fieldType === 'boolean'" class="config-checkbox">
      <input :checked="value === true" type="checkbox" @change="updateBoolean" />
      <span>{{ value === true ? '已启用' : '已停用' }}</span>
    </label>
    <input
      v-else-if="fieldType === 'integer' || fieldType === 'number'"
      :value="typeof value === 'number' ? value : ''"
      type="number"
      @input="updateNumber"
    />
    <input v-else-if="fieldType === 'string'" :value="typeof value === 'string' ? value : ''" :type="sensitive ? 'password' : 'text'" @input="updateString" />
    <output v-else class="config-unsupported">{{ value === null ? 'null' : '暂不支持编辑' }}</output>
  </section>
</template>

<style scoped>
.config-field {
  display: grid;
  gap: 0.45rem;
  min-width: 0;
}

.config-field-nested {
  border-left: 2px solid #d8dfda;
  padding-left: 0.75rem;
}

.config-field-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.75rem;
}

.config-field-heading label {
  color: #36453c;
  font-size: 0.84rem;
  font-weight: 650;
}

code {
  color: #718078;
  font-size: 0.72rem;
}

input:not([type='checkbox']),
select {
  width: 100%;
  min-height: 2.25rem;
  border: 1px solid #c8d0cb;
  border-radius: 4px;
  padding: 0 0.65rem;
  background: #ffffff;
  color: #18201b;
}

.config-group,
.config-array {
  display: grid;
  gap: 0.75rem;
}

.config-array-item {
  display: grid;
  gap: 0.5rem;
  border: 1px solid #d8dfda;
  border-radius: 4px;
  padding: 0.65rem;
  background: #ffffff;
}

.config-checkbox {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  color: #526159;
  font-size: 0.82rem;
}

.config-checkbox input {
  width: 1rem;
  height: 1rem;
  accent-color: #206b3a;
}

.config-add,
.config-remove {
  width: fit-content;
  min-height: 2rem;
  border: 1px solid #c7d0ca;
  border-radius: 4px;
  padding: 0 0.65rem;
  background: #ffffff;
  color: #2f6f95;
  cursor: pointer;
  font-size: 0.78rem;
  font-weight: 650;
}

.config-remove {
  color: #9a2f29;
}

.config-add:hover,
.config-remove:hover {
  border-color: currentColor;
}

.config-muted,
.config-unsupported {
  color: #718078;
  font-size: 0.8rem;
}

.config-unsupported {
  display: block;
  line-height: 1.45;
}
</style>
