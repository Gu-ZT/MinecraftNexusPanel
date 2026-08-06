<script setup lang="ts">
import {
  Button as AButton,
  Input as AInput,
  InputNumber as AInputNumber,
  InputPassword as AInputPassword,
  Option as AOption,
  Select as ASelect,
  Switch as ASwitch,
} from '@arco-design/web-vue';
import { IconDelete, IconPlus } from '@arco-design/web-vue/es/icon';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';

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

const { t } = useI18n();

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
    return t('config.tupleArrayUnsupported');
  }
  if (hasSchemaComposition(props.schema) || (itemSchema.value && hasSchemaComposition(itemSchema.value))) {
    return t('config.composedArrayUnsupported');
  }
  const schema = itemSchema.value;
  if (!schema || Object.keys(schema).length === 0) {
    return t('config.arraySchemaMissing');
  }
  if (arrayValue.value.some((item) => !schemaAcceptsValue(schema, item))) {
    return t('config.arrayItemMismatch');
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

function updateString(value: string): void {
  emit('update:value', value);
}

function updateNumber(value: number | undefined): void {
  emit('update:value', value ?? null);
}

function updateBoolean(value: string | number | boolean): void {
  emit('update:value', value === true);
}

function updateEnum(value: unknown): void {
  const encoded = String(value ?? '');
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
      <p v-if="properties.length === 0" class="config-muted">{{ t('config.emptyObject') }}</p>
    </div>

    <div v-else-if="fieldType === 'array' && arrayEditingSupported" class="config-array">
      <div v-for="(item, index) in arrayValue" :key="index" class="config-array-item">
        <ConfigValueEditor
          :depth="depth + 1"
          :label="t('config.itemLabel', { index: index + 1 })"
          :schema="itemSchema ?? {}"
          :ui-schema="itemUiSchema"
          :value="item"
          @update:value="updateArray(index, $event)"
        />
        <a-button class="config-remove" size="mini" status="danger" @click="removeArrayItem(index)">
          <template #icon><IconDelete /></template>
          {{ t('common.delete') }}
        </a-button>
      </div>
      <a-button class="config-add" size="mini" @click="addArrayItem">
        <template #icon><IconPlus /></template>
        {{ t('config.addItem') }}
      </a-button>
    </div>
    <output v-else-if="fieldType === 'array'" class="config-unsupported">
      {{ arrayUnsupportedReason }}<span v-if="arrayValue.length">{{ t('config.itemCount', { count: arrayValue.length }) }}</span>
    </output>

    <a-select v-else-if="enumValues.length > 0" :model-value="enumValue(value)" @change="updateEnum">
      <a-option v-for="option in enumValues" :key="enumValue(option)" :value="enumValue(option)">
        {{ option }}
      </a-option>
    </a-select>
    <div v-else-if="fieldType === 'boolean'" class="config-checkbox">
      <a-switch size="small" :model-value="value === true" @change="updateBoolean" />
      <span>{{ value === true ? t('config.enabled') : t('config.disabled') }}</span>
    </div>
    <a-input-number
      v-else-if="fieldType === 'integer' || fieldType === 'number'"
      :model-value="typeof value === 'number' ? value : 0"
      @change="updateNumber"
    />
    <a-input-password
      v-else-if="fieldType === 'string' && sensitive"
      :model-value="typeof value === 'string' ? value : ''"
      @input="updateString"
    />
    <a-input
      v-else-if="fieldType === 'string'"
      :model-value="typeof value === 'string' ? value : ''"
      @input="updateString"
    />
    <output v-else class="config-unsupported">{{ value === null ? 'null' : t('config.unsupported') }}</output>
  </section>
</template>

<style scoped>
.config-field {
  display: grid;
  gap: 0.45rem;
  min-width: 0;
}

.config-field-nested {
  border-left: 2px solid var(--mcnp-border);
  padding-left: 0.75rem;
}

.config-field-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.75rem;
}

.config-field-heading label {
  color: var(--mcnp-text-muted);
  font-size: 0.84rem;
  font-weight: 650;
}

code {
  color: var(--mcnp-text-faint);
  font-size: 0.72rem;
}

.config-field :deep(.arco-input-wrapper),
.config-field :deep(.arco-select-view) {
  width: 100%;
  min-height: 2.25rem;
  border-color: var(--mcnp-border);
  border-radius: 4px;
  background: var(--mcnp-surface);
  color: var(--mcnp-text);
}

.config-group,
.config-array {
  display: grid;
  gap: 0.75rem;
}

.config-array-item {
  display: grid;
  gap: 0.5rem;
  border: 1px solid var(--mcnp-border);
  border-radius: 4px;
  padding: 0.65rem;
  background: var(--mcnp-surface-raised);
}

.config-checkbox {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--mcnp-text-muted);
  font-size: 0.82rem;
}

.config-add,
.config-remove {
  width: fit-content;
}

.config-muted,
.config-unsupported {
  color: var(--mcnp-text-faint);
  font-size: 0.8rem;
}

.config-unsupported {
  display: block;
  line-height: 1.45;
}
</style>
