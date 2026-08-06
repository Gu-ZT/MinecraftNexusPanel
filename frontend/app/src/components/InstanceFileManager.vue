<script setup lang="ts">
import {
  Button as AButton,
  Empty as AEmpty,
  Input as AInput,
  Modal as AModal,
  Popconfirm as APopconfirm,
  Progress as AProgress,
  Spin as ASpin,
  Textarea as ATextarea,
  Tooltip as ATooltip,
} from '@arco-design/web-vue';
import {
  IconDelete,
  IconDownload,
  IconEdit,
  IconFile,
  IconFolder,
  IconFolderAdd,
  IconHome,
  IconPlus,
  IconRefresh,
  IconSearch,
  IconUpload,
} from '@arco-design/web-vue/es/icon';
import { computed, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import type { FileEntry, FileTask, PanelApiClient } from '@mcnp/api-client';

import { describeError, formatBytes, formatDate } from '../utils/presentation';
import { Sha256Digest } from '../utils/sha256-digest';

const props = defineProps<{
  client: PanelApiClient;
  coreId: string;
  instanceId: string;
}>();

type CreateMode = 'file' | 'directory' | 'rename';

const { locale, t } = useI18n();
const entries = ref<FileEntry[]>([]);
const currentPath = ref('');
const query = ref('');
const loading = ref(false);
const pending = ref('');
const progress = ref(0);
const errorMessage = ref('');
const noticeMessage = ref('');
const uploadInput = ref<HTMLInputElement | null>(null);
const createVisible = ref(false);
const createMode = ref<CreateMode>('file');
const createName = ref('');
const renameEntry = ref<FileEntry | null>(null);
const editorVisible = ref(false);
const editorEntry = ref<FileEntry | null>(null);
const editorContent = ref('');
const editorSha256 = ref('');
const editorLoading = ref(false);

const sortedEntries = computed(() =>
  [...entries.value]
    .filter((entry) => entry.name.toLocaleLowerCase().includes(query.value.trim().toLocaleLowerCase()))
    .sort((left, right) => {
      if (left.kind === 'DIRECTORY' && right.kind !== 'DIRECTORY') {
        return -1;
      }
      if (left.kind !== 'DIRECTORY' && right.kind === 'DIRECTORY') {
        return 1;
      }
      return left.name.localeCompare(right.name, locale.value);
    }),
);
const breadcrumbs = computed(() => {
  const result = [{ label: t('files.root'), path: '' }];
  const segments = currentPath.value.split('/').filter(Boolean);
  let path = '';
  for (const segment of segments) {
    path = path ? `${path}/${segment}` : segment;
    result.push({ label: segment, path });
  }
  return result;
});

onMounted(() => {
  void loadEntries();
});

watch(
  () => [props.coreId, props.instanceId],
  () => {
    currentPath.value = '';
    void loadEntries();
  },
);

async function loadEntries(): Promise<void> {
  loading.value = true;
  errorMessage.value = '';
  try {
    const loadedEntries: FileEntry[] = [];
    let cursor: string | undefined;
    do {
      const page = await props.client.listInstanceFiles(
        props.coreId,
        props.instanceId,
        currentPath.value,
        cursor,
        200,
      );
      loadedEntries.push(...page.items);
      cursor = page.nextCursor ?? undefined;
    } while (cursor !== undefined);
    entries.value = loadedEntries;
  } catch (error) {
    entries.value = [];
    errorMessage.value = describeError(error, t('error.files'));
  } finally {
    loading.value = false;
  }
}

async function openEntry(entry: FileEntry): Promise<void> {
  if (entry.kind === 'DIRECTORY') {
    currentPath.value = entry.path;
    query.value = '';
    await loadEntries();
    return;
  }
  if (entry.kind === 'FILE') {
    await openEditor(entry);
  }
}

async function goToPath(path: string): Promise<void> {
  currentPath.value = path;
  query.value = '';
  await loadEntries();
}

function showCreate(mode: CreateMode, entry?: FileEntry): void {
  createMode.value = mode;
  createName.value = entry?.name ?? '';
  renameEntry.value = entry ?? null;
  createVisible.value = true;
}

async function submitCreate(): Promise<void> {
  const name = createName.value.trim();
  if (!name || name.includes('/') || name.includes('\\')) {
    errorMessage.value = t('files.invalidName');
    return;
  }

  pending.value = createMode.value;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    if (createMode.value === 'directory') {
      await props.client.createInstanceDirectory(props.coreId, props.instanceId, joinPath(currentPath.value, name));
      noticeMessage.value = t('files.directoryCreated');
    } else if (createMode.value === 'file') {
      await props.client.writeInstanceFile(
        props.coreId,
        props.instanceId,
        joinPath(currentPath.value, name),
        new Uint8Array(),
      );
      noticeMessage.value = t('files.fileCreated');
    } else if (renameEntry.value) {
      await props.client.moveInstanceFile(
        props.coreId,
        props.instanceId,
        renameEntry.value.path,
        joinPath(currentPath.value, name),
      );
      noticeMessage.value = t('files.renamed');
    }
    createVisible.value = false;
    await loadEntries();
  } catch (error) {
    errorMessage.value = describeError(error, t('error.fileOperation'));
  } finally {
    pending.value = '';
  }
}

async function openEditor(entry: FileEntry): Promise<void> {
  if (entry.size > 1024 * 1024) {
    errorMessage.value = t('files.editorSizeLimit');
    return;
  }
  editorVisible.value = true;
  editorLoading.value = true;
  editorEntry.value = entry;
  editorContent.value = '';
  editorSha256.value = '';
  errorMessage.value = '';
  try {
    const chunks: Uint8Array[] = [];
    let offset = 0;
    let eof = false;
    while (!eof) {
      const result = await props.client.readInstanceFile(props.coreId, props.instanceId, entry.path, offset);
      const chunk = new Uint8Array(result.data);
      chunks.push(chunk);
      offset += chunk.byteLength;
      eof = result.eof;
      editorSha256.value = result.sha256;
      if (!eof && chunk.byteLength === 0) {
        throw new Error(t('files.emptyReadChunk'));
      }
    }
    const bytes = concatenateBytes(chunks, offset);
    editorContent.value = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch (error) {
    editorVisible.value = false;
    errorMessage.value = describeError(error, t('error.readFile'));
  } finally {
    editorLoading.value = false;
  }
}

async function saveEditor(): Promise<void> {
  if (!editorEntry.value) {
    return;
  }
  pending.value = 'save';
  errorMessage.value = '';
  try {
    const content = new TextEncoder().encode(editorContent.value);
    const result = await props.client.writeInstanceFile(
      props.coreId,
      props.instanceId,
      editorEntry.value.path,
      content,
      editorSha256.value,
    );
    editorSha256.value = result.sha256 ?? '';
    editorVisible.value = false;
    noticeMessage.value = t('files.saved');
    await loadEntries();
  } catch (error) {
    errorMessage.value = describeError(error, t('error.writeFile'));
  } finally {
    pending.value = '';
  }
}

function triggerUpload(): void {
  uploadInput.value?.click();
}

async function uploadFiles(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement;
  const files = Array.from(input.files ?? []);
  input.value = '';
  if (!files.length) {
    return;
  }

  pending.value = 'upload';
  progress.value = 0;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    for (let fileIndex = 0; fileIndex < files.length; fileIndex += 1) {
      const file = files[fileIndex];
      if (!file) {
        continue;
      }
      await uploadFile(file, fileIndex, files.length);
    }
    noticeMessage.value = t('files.uploaded', { count: files.length });
    await loadEntries();
  } catch (error) {
    errorMessage.value = describeError(error, t('error.uploadFile'));
  } finally {
    pending.value = '';
    progress.value = 0;
  }
}

async function uploadFile(file: File, fileIndex: number, fileCount: number): Promise<void> {
  const completeSha256 = await hashFile(file, (fraction) => {
    progress.value = Math.round(((fileIndex + fraction * 0.15) / fileCount) * 100);
  });
  const transfer = await props.client.beginFileUpload(
    props.coreId,
    props.instanceId,
    joinPath(currentPath.value, file.name),
    file.size,
    completeSha256,
  );
  try {
    const partCount = Math.ceil(file.size / transfer.chunkSize);
    for (let partNumber = 0; partNumber < partCount; partNumber += 1) {
      const offset = partNumber * transfer.chunkSize;
      const chunk = await file.slice(offset, Math.min(offset + transfer.chunkSize, file.size)).arrayBuffer();
      await props.client.uploadFilePart(
        props.coreId,
        transfer.transferId,
        partNumber,
        chunk,
        await sha256Hex(chunk),
      );
      const uploadFraction = (partNumber + 1) / Math.max(partCount, 1);
      progress.value = Math.round(((fileIndex + 0.15 + uploadFraction * 0.85) / fileCount) * 100);
    }
    await props.client.completeFileUpload(props.coreId, transfer.transferId);
  } catch (error) {
    await props.client.abortFileUpload(props.coreId, transfer.transferId).catch(() => undefined);
    throw error;
  }
}

async function downloadEntry(entry: FileEntry): Promise<void> {
  pending.value = `download:${entry.path}`;
  progress.value = 0;
  errorMessage.value = '';
  const transfer = await props.client.beginFileDownload(props.coreId, props.instanceId, entry.path);
  try {
    const chunks: Uint8Array[] = [];
    const partCount = Math.ceil(transfer.sizeBytes / transfer.chunkSize);
    let totalSize = 0;
    for (let partNumber = 0; partNumber < partCount; partNumber += 1) {
      const chunk = await props.client.downloadFilePart(props.coreId, transfer.transferId, partNumber);
      const bytes = new Uint8Array(chunk.data);
      chunks.push(bytes);
      totalSize += bytes.byteLength;
      progress.value = Math.round(((partNumber + 1) / Math.max(partCount, 1)) * 100);
      if (chunk.eof) {
        break;
      }
    }
    await props.client.completeFileDownload(props.coreId, transfer.transferId);
    const bytes = concatenateBytes(chunks, totalSize);
    const blob = new Blob([bytes.slice().buffer as ArrayBuffer]);
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = url;
    anchor.download = entry.name;
    anchor.click();
    URL.revokeObjectURL(url);
    noticeMessage.value = t('files.downloaded');
  } catch (error) {
    await props.client.abortFileDownload(props.coreId, transfer.transferId).catch(() => undefined);
    errorMessage.value = describeError(error, t('error.downloadFile'));
  } finally {
    pending.value = '';
    progress.value = 0;
  }
}

async function deleteEntry(entry: FileEntry): Promise<void> {
  pending.value = `delete:${entry.path}`;
  errorMessage.value = '';
  noticeMessage.value = '';
  try {
    const accepted = await props.client.deleteInstanceFile(
      props.coreId,
      props.instanceId,
      entry.path,
      entry.kind === 'DIRECTORY',
    );
    const task = await waitForTask(accepted.taskId);
    if (task.state === 'FAILED') {
      throw new Error(task.error ?? t('error.fileOperation'));
    }
    noticeMessage.value = t('files.deleted');
    await loadEntries();
  } catch (error) {
    errorMessage.value = describeError(error, t('error.deleteFile'));
  } finally {
    pending.value = '';
  }
}

async function waitForTask(taskId: string): Promise<FileTask> {
  for (let attempt = 0; attempt < 240; attempt += 1) {
    const task = await props.client.getFileTask(props.coreId, taskId);
    if (task.state !== 'RUNNING') {
      return task;
    }
    await new Promise<void>((resolve) => window.setTimeout(resolve, 500));
  }
  throw new Error(t('files.taskTimeout'));
}

function joinPath(parent: string, name: string): string {
  return parent ? `${parent}/${name}` : name;
}

function concatenateBytes(chunks: Uint8Array[], totalSize: number): Uint8Array {
  const result = new Uint8Array(totalSize);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return result;
}

async function sha256Hex(content: ArrayBuffer): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', content);
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('');
}

async function hashFile(file: File, reportProgress: (fraction: number) => void): Promise<string> {
  const digest = new Sha256Digest();
  const chunkSize = 4 * 1024 * 1024;
  for (let offset = 0; offset < file.size; offset += chunkSize) {
    const chunk = await file.slice(offset, Math.min(offset + chunkSize, file.size)).arrayBuffer();
    digest.update(new Uint8Array(chunk));
    reportProgress(Math.min((offset + chunk.byteLength) / Math.max(file.size, 1), 1));
  }
  if (file.size === 0) {
    reportProgress(1);
  }
  return digest.digestHex();
}
</script>

<template>
  <section class="file-manager">
    <header class="file-toolbar">
      <div class="file-actions">
        <a-tooltip :content="t('files.newFile')">
          <a-button size="small" :disabled="Boolean(pending)" :aria-label="t('files.newFile')" @click="showCreate('file')">
            <template #icon><IconPlus /></template>
          </a-button>
        </a-tooltip>
        <a-tooltip :content="t('files.newDirectory')">
          <a-button size="small" :disabled="Boolean(pending)" :aria-label="t('files.newDirectory')" @click="showCreate('directory')">
            <template #icon><IconFolderAdd /></template>
          </a-button>
        </a-tooltip>
        <a-button size="small" :loading="pending === 'upload'" :disabled="Boolean(pending)" @click="triggerUpload">
          <template #icon><IconUpload /></template>
          {{ t('files.upload') }}
        </a-button>
        <input ref="uploadInput" class="visually-hidden" type="file" multiple @change="uploadFiles" />
        <a-tooltip :content="t('common.refresh')">
          <a-button size="small" :loading="loading" :aria-label="t('common.refresh')" @click="loadEntries">
            <template #icon><IconRefresh /></template>
          </a-button>
        </a-tooltip>
      </div>
      <a-input v-model="query" class="file-search" allow-clear :placeholder="t('files.searchPlaceholder')">
        <template #prefix><IconSearch /></template>
      </a-input>
    </header>

    <nav class="file-breadcrumbs" :aria-label="t('files.breadcrumb')">
      <button v-for="(item, index) in breadcrumbs" :key="item.path" type="button" @click="goToPath(item.path)">
        <IconHome v-if="index === 0" />
        <span v-else>{{ item.label }}</span>
      </button>
      <span class="file-path">/{{ currentPath }}</span>
    </nav>

    <a-progress v-if="pending === 'upload' || pending.startsWith('download:')" :percent="progress / 100" :show-text="true" />
    <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
    <p v-else-if="noticeMessage" class="notice" role="status">{{ noticeMessage }}</p>

    <a-spin class="file-list-spinner" :loading="loading">
      <div v-if="sortedEntries.length" class="file-table-wrap">
        <table class="file-table">
          <thead>
            <tr>
              <th>{{ t('files.name') }}</th>
              <th>{{ t('files.kind') }}</th>
              <th>{{ t('files.size') }}</th>
              <th>{{ t('files.modifiedAt') }}</th>
              <th>{{ t('common.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="entry in sortedEntries" :key="entry.path">
              <td>
                <button class="file-name" type="button" @click="openEntry(entry)">
                  <IconFolder v-if="entry.kind === 'DIRECTORY'" />
                  <IconFile v-else />
                  <span>{{ entry.name }}</span>
                </button>
              </td>
              <td>{{ t(`files.kindLabel.${entry.kind}`) }}</td>
              <td>{{ entry.kind === 'DIRECTORY' ? t('common.none') : formatBytes(entry.size, locale) }}</td>
              <td>{{ formatDate(entry.modifiedAt, locale, t('common.notRecorded')) }}</td>
              <td>
                <div class="row-actions">
                  <a-tooltip v-if="entry.kind === 'FILE'" :content="t('files.edit')">
                    <a-button size="mini" :disabled="Boolean(pending)" :aria-label="t('files.edit')" @click="openEditor(entry)">
                      <template #icon><IconEdit /></template>
                    </a-button>
                  </a-tooltip>
                  <a-tooltip v-if="entry.kind === 'FILE'" :content="t('files.download')">
                    <a-button
                      size="mini"
                      :loading="pending === `download:${entry.path}`"
                      :disabled="Boolean(pending)"
                      :aria-label="t('files.download')"
                      @click="downloadEntry(entry)"
                    >
                      <template #icon><IconDownload /></template>
                    </a-button>
                  </a-tooltip>
                  <a-tooltip :content="t('files.rename')">
                    <a-button size="mini" :disabled="Boolean(pending)" :aria-label="t('files.rename')" @click="showCreate('rename', entry)">
                      <template #icon><IconEdit /></template>
                    </a-button>
                  </a-tooltip>
                  <a-popconfirm :content="t('files.deleteConfirm', { name: entry.name })" @ok="deleteEntry(entry)">
                    <a-tooltip :content="t('common.delete')">
                      <a-button
                        size="mini"
                        status="danger"
                        :loading="pending === `delete:${entry.path}`"
                        :disabled="Boolean(pending)"
                        :aria-label="t('common.delete')"
                      >
                        <template #icon><IconDelete /></template>
                      </a-button>
                    </a-tooltip>
                  </a-popconfirm>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <a-empty v-else :description="query ? t('files.noMatches') : t('files.empty')" />
    </a-spin>

    <a-modal v-model:visible="createVisible" :footer="false" unmount-on-close>
      <template #title>{{ t(`files.dialog.${createMode}`) }}</template>
      <form class="file-dialog" @submit.prevent="submitCreate">
        <a-input v-model="createName" autofocus :placeholder="t('files.namePlaceholder')" />
        <div>
          <a-button @click="createVisible = false">{{ t('common.cancel') }}</a-button>
          <a-button type="primary" html-type="submit" :loading="Boolean(pending)">{{ t('common.confirm') }}</a-button>
        </div>
      </form>
    </a-modal>

    <a-modal v-model:visible="editorVisible" :footer="false" :width="900" unmount-on-close>
      <template #title>{{ editorEntry?.path ?? t('files.edit') }}</template>
      <a-spin class="editor-spinner" :loading="editorLoading">
        <a-textarea v-model="editorContent" class="file-editor" :auto-size="false" />
        <div class="editor-actions">
          <a-button @click="editorVisible = false">{{ t('common.cancel') }}</a-button>
          <a-button type="primary" :loading="pending === 'save'" @click="saveEditor">{{ t('common.save') }}</a-button>
        </div>
      </a-spin>
    </a-modal>
  </section>
</template>

<style scoped>
.file-manager {
  display: flex;
  min-height: 0;
  flex: 1;
  flex-direction: column;
  gap: 0.65rem;
  padding: 0.75rem;
}

.file-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.file-actions,
.row-actions,
.editor-actions,
.file-dialog > div {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.file-search {
  width: min(100%, 20rem);
}

.file-breadcrumbs {
  display: flex;
  min-height: 2.4rem;
  align-items: center;
  gap: 0.25rem;
  overflow-x: auto;
  border: 1px solid var(--mcnp-border);
  border-radius: 4px;
  padding: 0.35rem 0.5rem;
  background: var(--mcnp-surface-raised);
}

.file-breadcrumbs button {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  border: 0;
  padding: 0.2rem 0.3rem;
  background: transparent;
  color: var(--mcnp-primary);
  cursor: pointer;
  font-size: 0.68rem;
  white-space: nowrap;
}

.file-breadcrumbs button + button::before {
  margin-right: 0.3rem;
  color: var(--mcnp-text-faint);
  content: '/';
}

.file-path {
  margin-left: auto;
  color: var(--mcnp-text-faint);
  font-family: "Cascadia Mono", Consolas, monospace;
  font-size: 0.62rem;
  white-space: nowrap;
}

.file-list-spinner {
  display: block;
  min-height: 15rem;
  flex: 1;
}

.file-table-wrap {
  height: 100%;
  overflow: auto;
  border: 1px solid var(--mcnp-border);
  border-radius: 4px;
}

.file-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.7rem;
}

.file-table th,
.file-table td {
  padding: 0.62rem 0.75rem;
  border-bottom: 1px solid var(--mcnp-border-subtle);
  color: var(--mcnp-text-muted);
  text-align: left;
}

.file-table th {
  position: sticky;
  z-index: 1;
  top: 0;
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-text-faint);
  font-size: 0.64rem;
}

.file-table tr:last-child td {
  border-bottom: 0;
}

.file-table th:first-child {
  width: 42%;
}

.file-table th:last-child {
  width: 9rem;
  text-align: right;
}

.file-table td:last-child .row-actions {
  justify-content: flex-end;
}

.file-name {
  display: inline-flex;
  max-width: 100%;
  align-items: center;
  gap: 0.5rem;
  border: 0;
  padding: 0;
  background: transparent;
  color: var(--mcnp-text);
  cursor: pointer;
  font-weight: 600;
}

.file-name svg {
  flex: 0 0 auto;
  color: var(--mcnp-primary);
}

.file-name span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-dialog {
  display: grid;
  gap: 1rem;
}

.file-dialog > div,
.editor-actions {
  justify-content: flex-end;
}

.editor-spinner {
  display: block;
}

.file-editor {
  height: min(60vh, 36rem);
}

.file-editor :deep(textarea) {
  height: 100%;
  font-family: "Cascadia Mono", Consolas, monospace;
  font-size: 0.72rem;
  line-height: 1.55;
}

.editor-actions {
  margin-top: 0.75rem;
}

@media (max-width: 44rem) {
  .file-toolbar {
    align-items: stretch;
    flex-direction: column;
  }

  .file-search {
    width: 100%;
  }

  .file-table th:nth-child(2),
  .file-table td:nth-child(2),
  .file-table th:nth-child(3),
  .file-table td:nth-child(3),
  .file-table th:nth-child(4),
  .file-table td:nth-child(4) {
    display: none;
  }
}
</style>
