<script setup lang="ts">
import { Button as AButton, Checkbox as ACheckbox } from '@arco-design/web-vue';
import { IconCheck, IconRefresh, IconSave } from '@arco-design/web-vue/es/icon';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import type {
  ConfigDocument,
  ConfigDocumentSummary,
  ConfigValidationIssue,
  PanelApiClient,
} from '@mcnp/api-client';

import ConfigValueEditor from './ConfigValueEditor.vue';

const { t } = useI18n();

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
      errorMessage.value = describeError(error, t('error.readDocuments'));
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
    noticeMessage.value = t('notice.documentsScanned');
  } catch (error) {
    if (generation === workspaceGeneration && selection === documentGeneration) {
      errorMessage.value = describeError(error, t('error.scanDocuments'));
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
      errorMessage.value = describeError(error, t('error.readDocument'));
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
    noticeMessage.value = result.valid ? t('notice.validationPassed') : t('notice.validationFailed');
  } catch (error) {
    if (generation === workspaceGeneration) {
      errorMessage.value = describeError(error, t('error.validateDocuments'));
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
    noticeMessage.value = t('notice.configSaved');
  } catch (error) {
    errorMessage.value = describeError(error, t('error.saveDocument'));
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
    return `${fallback}: ${error.message}`;
  }
  return fallback;
}
</script>

<template>
  <section class="config-editor" :aria-label="t('config.editorAria')">
    <header class="config-editor-head">
      <div>
        <h2>{{ t('config.title') }}</h2>
        <p>{{ selectedDocument?.path ?? t('config.selectDocument') }}</p>
      </div>
      <div class="config-actions">
        <a-button size="small" :disabled="loading || saving" @click="scanDocuments">
          <template #icon><IconRefresh /></template>
          {{ t('config.rescan') }}
        </a-button>
        <a-button size="small" :disabled="loading || saving || documents.length === 0" @click="validateDocuments">
          <template #icon><IconCheck /></template>
          {{ t('config.validate') }}
        </a-button>
      </div>
    </header>

    <div class="config-editor-body">
      <aside class="config-documents" :aria-label="t('config.documentsAria')">
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
          <small>
            {{ t('config.formatRevision', { format: document.format, revision: document.revision.slice(0, 8) }) }}
          </small>
        </button>
        <p v-if="!loading && documents.length === 0" class="config-muted">{{ t('config.emptyDocuments') }}</p>
      </aside>

      <div class="config-form">
        <p v-if="errorMessage" class="form-error">{{ errorMessage }}</p>
        <p v-else-if="noticeMessage" class="notice">{{ noticeMessage }}</p>
        <p v-if="selectedDocument?.unmapped.length" class="config-warning">
          {{ t('config.unmappedFields', { fields: selectedDocument.unmapped.join(', ') }) }}
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
        <p v-else-if="!loading" class="config-muted">{{ t('config.selectPrompt') }}</p>

        <a-checkbox v-if="selectedDocument?.lossy" v-model="allowLossy" class="lossy-confirmation">
          {{ t('config.allowLossy') }}
        </a-checkbox>
        <a-button
          v-if="selectedDocument"
          class="config-save"
          type="primary"
          size="small"
          :loading="saving"
          :disabled="saving || loading || (requiresLossyConfirmation && !allowLossy)"
          @click="saveDocument"
        >
          <template #icon><IconSave /></template>
          {{ saving ? t('config.saving') : t('config.save') }}
        </a-button>

        <ul v-if="issues.length" class="config-issues">
          <li v-for="issue in issues" :key="`${issue.code}-${issue.path}-${issue.field}`">
            <strong>{{ issue.severity }}</strong>
            <span>{{ issue.path }}: {{ issue.message }}</span>
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
  background: var(--mcnp-surface);
}

.config-editor-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.85rem 1rem;
  border-bottom: 1px solid var(--mcnp-border);
  background: var(--mcnp-surface);
}

.config-editor-head h2 {
  margin: 0;
  color: var(--mcnp-text);
  font-size: 0.95rem;
}

.config-editor-head p {
  margin: 0.3rem 0 0;
  overflow: hidden;
  color: var(--mcnp-text-faint);
  font-size: 0.78rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.config-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.config-save {
  width: fit-content;
}

.config-actions :deep(.arco-btn) {
  border-color: var(--mcnp-border);
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-text-muted);
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
  border-right: 1px solid var(--mcnp-border);
  background: var(--mcnp-surface-raised);
}

.config-document {
  display: grid;
  gap: 0.3rem;
  width: 100%;
  min-width: 0;
  border: 1px solid transparent;
  border-radius: 4px;
  padding: 0.65rem;
  background: transparent;
  color: var(--mcnp-text);
  cursor: pointer;
  text-align: left;
}

.config-document.selected {
  border-color: var(--mcnp-primary);
  background: var(--mcnp-primary-soft);
}

.config-document:hover:not(:disabled) {
  background: var(--mcnp-surface-hover);
}

.config-document strong,
.config-document small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.config-document strong {
  color: var(--mcnp-text);
  font-size: 0.82rem;
}

.config-document small {
  color: var(--mcnp-text-faint);
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
  background: var(--mcnp-danger-soft);
  color: var(--mcnp-danger);
}

.notice {
  background: var(--mcnp-success-soft);
  color: var(--mcnp-success);
}

.config-warning {
  background: var(--mcnp-warning-soft);
  color: var(--mcnp-warning);
}

.lossy-confirmation {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  color: var(--mcnp-warning);
  font-size: 0.8rem;
}

.config-muted {
  margin: 0;
  color: var(--mcnp-text-faint);
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
  border-left: 3px solid var(--mcnp-border);
  padding: 0.45rem 0.6rem;
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-text-muted);
  font-size: 0.78rem;
}

.config-issues li strong {
  color: var(--mcnp-warning);
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
    border-bottom: 1px solid var(--mcnp-border);
  }
}
</style>
