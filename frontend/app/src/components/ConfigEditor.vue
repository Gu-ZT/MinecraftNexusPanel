<script setup lang="ts">
import { computed, ref, watch } from 'vue';

import type {
  ConfigDocument,
  ConfigDocumentSummary,
  ConfigValidationIssue,
  PanelApiClient,
} from '@mcnp/api-client';

import ConfigValueEditor from './ConfigValueEditor.vue';

const props = defineProps<{
  client: PanelApiClient;
  coreId: string;
  instanceId: string;
}>();

const documents = ref<ConfigDocumentSummary[]>([]);
const selectedDocument = ref<ConfigDocument | null>(null);
const draft = ref<Record<string, unknown>>({});
const issues = ref<ConfigValidationIssue[]>([]);
const loading = ref(false);
const saving = ref(false);
const allowLossy = ref(false);
const errorMessage = ref('');
const noticeMessage = ref('');
let workspaceGeneration = 0;
let documentGeneration = 0;

const draftEntries = computed(() => Object.entries(draft.value));
const requiresLossyConfirmation = computed(() => selectedDocument.value?.lossy === true);

function asRecord(value: unknown): Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

watch(
  () => `${props.coreId}:${props.instanceId}`,
  () => {
    void loadDocuments();
  },
  { immediate: true },
);

async function loadDocuments(): Promise<void> {
  // 实例切换或快速切换文档会产生并发请求，旧响应不能覆盖当前草稿。
  const generation = ++workspaceGeneration;
  const selection = ++documentGeneration;
  documents.value = [];
  selectedDocument.value = null;
  draft.value = {};
  issues.value = [];
  errorMessage.value = '';
  noticeMessage.value = '';
  if (!props.coreId || !props.instanceId) {
    return;
  }

  loading.value = true;
  try {
    const page = await props.client.listConfigDocuments(props.coreId, props.instanceId);
    if (generation !== workspaceGeneration || selection !== documentGeneration) {
      return;
    }
    documents.value = page.documents;
    const firstDocument = page.documents[0];
    if (firstDocument) {
      await loadDocument(firstDocument.documentId, generation);
    }
  } catch (error) {
    if (generation === workspaceGeneration && selection === documentGeneration) {
      errorMessage.value = describeError(error, '无法读取配置文档');
    }
  } finally {
    if (generation === workspaceGeneration && selection === documentGeneration) {
      loading.value = false;
    }
  }
}

async function scanDocuments(): Promise<void> {
  if (!props.coreId || !props.instanceId) {
    return;
  }
  const generation = workspaceGeneration;
  const selection = ++documentGeneration;
  loading.value = true;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    const page = await props.client.scanConfigDocuments(props.coreId, props.instanceId);
    if (generation !== workspaceGeneration || selection !== documentGeneration) {
      return;
    }
    documents.value = page.documents;
    const currentId = selectedDocument.value?.documentId;
    const nextId = page.documents.some((document) => document.documentId === currentId)
      ? currentId
      : page.documents[0]?.documentId;
    if (nextId) {
      await loadDocument(nextId, generation);
    } else {
      selectedDocument.value = null;
      draft.value = {};
      allowLossy.value = false;
      issues.value = [];
    }
    noticeMessage.value = '配置文档已重新扫描';
  } catch (error) {
    if (generation === workspaceGeneration && selection === documentGeneration) {
      errorMessage.value = describeError(error, '配置扫描失败');
    }
  } finally {
    if (generation === workspaceGeneration && selection === documentGeneration) {
      loading.value = false;
    }
  }
}

async function loadDocument(documentId: string, generation = workspaceGeneration): Promise<void> {
  const selection = ++documentGeneration;
  loading.value = true;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    const document = await props.client.getConfigDocument(props.coreId, props.instanceId, documentId);
    if (generation !== workspaceGeneration || selection !== documentGeneration) {
      return;
    }
    selectedDocument.value = document;
    draft.value = cloneValues(document.values);
    allowLossy.value = false;
    issues.value = [];
  } catch (error) {
    if (generation === workspaceGeneration && selection === documentGeneration) {
      errorMessage.value = describeError(error, '无法读取配置文档');
    }
  } finally {
    if (generation === workspaceGeneration && selection === documentGeneration) {
      loading.value = false;
    }
  }
}

async function validateDocuments(): Promise<void> {
  if (!props.coreId || !props.instanceId) {
    return;
  }
  const generation = workspaceGeneration;
  loading.value = true;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    const result = await props.client.validateConfigDocuments(props.coreId, props.instanceId);
    if (generation !== workspaceGeneration) {
      return;
    }
    issues.value = result.issues;
    noticeMessage.value = result.valid ? '配置校验通过' : '配置校验发现问题';
  } catch (error) {
    if (generation === workspaceGeneration) {
      errorMessage.value = describeError(error, '配置校验失败');
    }
  } finally {
    if (generation === workspaceGeneration) {
      loading.value = false;
    }
  }
}

async function saveDocument(): Promise<void> {
  const document = selectedDocument.value;
  if (!document || (requiresLossyConfirmation.value && !allowLossy.value)) {
    return;
  }
  saving.value = true;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    const updated = await props.client.patchConfigDocument(
      props.coreId,
      props.instanceId,
      document.documentId,
      document.revision,
      draft.value,
      allowLossy.value,
    );
    selectedDocument.value = updated;
    draft.value = cloneValues(updated.values);
    allowLossy.value = false;
    noticeMessage.value = '配置已保存';
  } catch (error) {
    errorMessage.value = describeError(error, '配置保存失败');
  } finally {
    saving.value = false;
  }
}

function updateDraft(key: string, value: unknown): void {
  draft.value = { ...draft.value, [key]: value };
}

function cloneValues(values: Record<string, unknown>): Record<string, unknown> {
  return JSON.parse(JSON.stringify(values)) as Record<string, unknown>;
}

function describeError(error: unknown, fallback: string): string {
  if (error instanceof Error) {
    return `${fallback}：${error.message}`;
  }
  return fallback;
}
</script>

<template>
  <section class="config-editor" aria-label="配置编辑器">
    <header class="config-editor-head">
      <div>
        <h2>结构化配置</h2>
        <p>{{ selectedDocument?.path ?? '选择配置文档' }}</p>
      </div>
      <div class="config-actions">
        <button type="button" :disabled="loading || saving" @click="scanDocuments">重新扫描</button>
        <button type="button" :disabled="loading || saving || documents.length === 0" @click="validateDocuments">校验</button>
      </div>
    </header>

    <div class="config-editor-body">
      <aside class="config-documents" aria-label="配置文档列表">
        <button
          v-for="document in documents"
          :key="document.documentId"
          class="config-document"
          :class="{ selected: document.documentId === selectedDocument?.documentId }"
          type="button"
          :disabled="loading || saving"
          @click="loadDocument(document.documentId)"
        >
          <strong>{{ document.path }}</strong>
          <small>{{ document.format }} · revision {{ document.revision.slice(0, 8) }}</small>
        </button>
        <p v-if="!loading && documents.length === 0" class="config-muted">暂无结构化配置</p>
      </aside>

      <div class="config-form">
        <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>
        <p v-else-if="noticeMessage" class="notice">{{ noticeMessage }}</p>
        <p v-if="selectedDocument?.unmapped.length" class="config-warning">
          未映射字段：{{ selectedDocument.unmapped.join('、') }}
        </p>

        <div v-if="selectedDocument" class="config-fields">
          <ConfigValueEditor
            v-for="[key, value] in draftEntries"
            :key="key"
            :label="key"
            :schema="asRecord(asRecord(selectedDocument.schema.properties)[key])"
            :ui-schema="asRecord(asRecord(asRecord(selectedDocument.uiSchema).properties)[key])"
            :value="value"
            @update:value="updateDraft(key, $event)"
          />
        </div>
        <p v-else-if="!loading" class="config-muted">选择一个配置文档开始编辑。</p>

        <label v-if="selectedDocument?.lossy" class="lossy-confirmation">
          <input v-model="allowLossy" type="checkbox" />
          <span>允许规范化写回</span>
        </label>
        <button
          v-if="selectedDocument"
          class="config-save"
          type="button"
          :disabled="saving || loading || (requiresLossyConfirmation && !allowLossy)"
          @click="saveDocument"
        >
          {{ saving ? '正在保存' : '保存配置' }}
        </button>

        <ul v-if="issues.length" class="config-issues">
          <li v-for="issue in issues" :key="`${issue.code}-${issue.path}-${issue.field}`">
            <strong>{{ issue.severity }}</strong>
            <span>{{ issue.path }}：{{ issue.message }}</span>
          </li>
        </ul>
      </div>
    </div>
  </section>
</template>

<style scoped>
.config-editor {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  background: #f7f8f7;
}

.config-editor-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.85rem 1rem;
  border-bottom: 1px solid #d7dcd8;
  background: #ffffff;
}

.config-editor-head h2 {
  margin: 0;
  color: #18201b;
  font-size: 0.95rem;
}

.config-editor-head p {
  margin: 0.3rem 0 0;
  overflow: hidden;
  color: #637068;
  font-size: 0.78rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.config-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.config-actions button,
.config-save {
  min-height: 2.15rem;
  border: 1px solid #c7d0ca;
  border-radius: 4px;
  padding: 0 0.7rem;
  background: #ffffff;
  color: #2f6f95;
  cursor: pointer;
  font-size: 0.78rem;
  font-weight: 650;
}

.config-actions button:hover:not(:disabled),
.config-save:hover:not(:disabled) {
  border-color: #2f7d4a;
  color: #1f6239;
}

.config-actions button:disabled,
.config-save:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.config-save {
  width: fit-content;
  border-color: #206b3a;
  background: #206b3a;
  color: #ffffff;
}

.config-editor-body {
  display: grid;
  min-height: 0;
  flex: 1;
  grid-template-columns: minmax(13rem, 18rem) minmax(0, 1fr);
}

.config-documents {
  display: grid;
  align-content: start;
  gap: 0.45rem;
  min-width: 0;
  overflow: auto;
  padding: 0.75rem;
  border-right: 1px solid #d7dcd8;
  background: #fbfcfb;
}

.config-document {
  display: grid;
  gap: 0.3rem;
  width: 100%;
  min-width: 0;
  border: 1px solid #c7d0ca;
  border-radius: 4px;
  padding: 0.65rem;
  background: #ffffff;
  color: #26352b;
  cursor: pointer;
  text-align: left;
}

.config-document.selected {
  border-color: #2f7d4a;
  background: #eef7f1;
}

.config-document strong,
.config-document small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.config-document strong {
  color: #18201b;
  font-size: 0.82rem;
}

.config-document small {
  color: #637068;
  font-size: 0.72rem;
}

.config-form {
  display: grid;
  align-content: start;
  gap: 0.8rem;
  min-width: 0;
  overflow: auto;
  padding: 1rem;
}

.config-fields {
  display: grid;
  gap: 1rem;
}

.form-error,
.notice,
.config-warning {
  margin: 0;
  border-radius: 4px;
  padding: 0.6rem 0.7rem;
  font-size: 0.8rem;
}

.form-error {
  background: #fde4e2;
  color: #8f2b25;
}

.notice {
  background: #e1f2ea;
  color: #1f6239;
}

.config-warning {
  background: #fff3cf;
  color: #8a5a00;
}

.lossy-confirmation {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: #8a5a00;
  font-size: 0.8rem;
}

.lossy-confirmation input {
  accent-color: #8a5a00;
}

.config-muted {
  margin: 0;
  color: #718078;
  font-size: 0.82rem;
}

.config-issues {
  display: grid;
  gap: 0.45rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.config-issues li {
  display: grid;
  gap: 0.25rem;
  border-left: 3px solid #c7d0ca;
  padding: 0.45rem 0.6rem;
  background: #ffffff;
  color: #526159;
  font-size: 0.78rem;
}

.config-issues li strong {
  color: #8a5a00;
  font-size: 0.7rem;
}

@media (max-width: 48rem) {
  .config-editor-head {
    align-items: stretch;
    flex-direction: column;
  }

  .config-editor-body {
    grid-template-columns: 1fr;
  }

  .config-documents {
    max-height: 12rem;
    border-right: 0;
    border-bottom: 1px solid #d7dcd8;
  }
}
</style>
