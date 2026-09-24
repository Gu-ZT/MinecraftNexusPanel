<script setup lang="ts">
/** 实例文件管理：目录浏览、文本编辑、新建目录与删除。 */

import { computed, ref } from 'vue';
import { useRoute } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message, Modal } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { PermissionGate, formatBytes, formatRelative } from '@mcnp/ui';
import { useApi } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const route = useRoute();
const queryClient = useQueryClient();
const auth = useAuthStore();
const authCanWrite = computed(() => auth.has('file.write'));
const instanceId = computed(() => route.params.id as string);

const currentPath = ref('/');
const editingFile = ref<string | null>(null);
const editorContent = ref('');
const mkdirVisible = ref(false);
const newDirName = ref('');

const { data: entries, isLoading } = useQuery({
  queryKey: computed(() => ['files', instanceId.value, currentPath.value]),
  queryFn: () => api.files.list(instanceId.value, currentPath.value),
});

const breadcrumbs = computed(() => {
  const parts = currentPath.value.split('/').filter(Boolean);
  const crumbs = [{ name: '根目录', path: '/' }];
  let acc = '';
  for (const part of parts) {
    acc += `/${part}`;
    crumbs.push({ name: part, path: acc });
  }
  return crumbs;
});

function refresh(): void {
  void queryClient.invalidateQueries({ queryKey: ['files', instanceId.value] });
}

async function openEntry(entry: { path: string; isDir: boolean }): Promise<void> {
  if (entry.isDir) {
    currentPath.value = entry.path;
    return;
  }
  try {
    editorContent.value = await api.files.read(instanceId.value, entry.path);
    editingFile.value = entry.path;
  } catch (err) {
    Message.warning(err instanceof ApiError ? err.message : '无法打开文件');
  }
}

const saveMutation = useMutation({
  mutationFn: () => api.files.write(instanceId.value, editingFile.value ?? '', editorContent.value),
  onSuccess: () => {
    Message.success('已保存');
    editingFile.value = null;
    refresh();
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '保存失败'),
});

async function mkdir(): Promise<void> {
  const name = newDirName.value.trim();
  if (!name) return;
  try {
    const base = currentPath.value === '/' ? '' : currentPath.value;
    await api.files.mkdir(instanceId.value, `${base}/${name}`);
    Message.success('目录已创建');
    mkdirVisible.value = false;
    newDirName.value = '';
    refresh();
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '创建失败');
  }
}

function removeEntry(entry: { path: string; name: string }): void {
  Modal.warning({
    title: '删除确认',
    content: `确认删除 ${entry.name}？该操作不可恢复。`,
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        await api.files.remove(instanceId.value, entry.path);
        Message.success('已删除');
        refresh();
      } catch (err) {
        Message.error(err instanceof ApiError ? err.message : '删除失败');
      }
    },
  });
}
</script>

<template>
  <div class="mcnp-card">
    <div class="file-toolbar">
      <ABreadcrumb>
        <ABreadcrumbItem v-for="crumb in breadcrumbs" :key="crumb.path">
          <a @click="currentPath = crumb.path">{{ crumb.name }}</a>
        </ABreadcrumbItem>
      </ABreadcrumb>
      <PermissionGate when="file.write">
        <AButton size="small" @click="mkdirVisible = true">新建目录</AButton>
      </PermissionGate>
    </div>

    <ATable :data="entries ?? []" :loading="isLoading" :pagination="false" row-key="path">
      <template #columns>
        <ATableColumn title="名称">
          <template #cell="{ record }">
            <a class="file-link" @click="openEntry(record)">
              {{ record.isDir ? '📁' : '📄' }} {{ record.name }}
            </a>
          </template>
        </ATableColumn>
        <ATableColumn title="大小" :width="120">
          <template #cell="{ record }">{{ record.isDir ? '—' : formatBytes(record.sizeBytes) }}</template>
        </ATableColumn>
        <ATableColumn title="修改时间" :width="130">
          <template #cell="{ record }">{{ formatRelative(record.modifiedAt) }}</template>
        </ATableColumn>
        <ATableColumn title="操作" :width="100">
          <template #cell="{ record }">
            <PermissionGate when="file.write">
              <AButton size="mini" status="danger" type="text" @click="removeEntry(record)">删除</AButton>
            </PermissionGate>
          </template>
        </ATableColumn>
      </template>
    </ATable>

    <AModal v-model:visible="mkdirVisible" title="新建目录" @ok="mkdir">
      <AInput v-model="newDirName" placeholder="目录名称" />
    </AModal>

    <AModal
      :visible="editingFile !== null"
      title="编辑文件"
      width="720px"
      :ok-loading="saveMutation.isPending.value"
      @ok="saveMutation.mutate()"
      @cancel="editingFile = null"
    >
      <p class="mono text-secondary" style="margin-top: 0">{{ editingFile }}</p>
      <ATextarea v-model="editorContent" class="mono" :auto-size="{ minRows: 14, maxRows: 24 }" :disabled="!authCanWrite" />
    </AModal>
  </div>
</template>

<style scoped>
.file-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--mcnp-space-3);
}

.file-link {
  cursor: pointer;
}
</style>
