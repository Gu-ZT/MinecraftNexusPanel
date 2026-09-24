<script setup lang="ts">
/** 模组/插件：聚合搜索（Modrinth/Hangar 等来源）与已安装列表。 */

import { computed, ref } from 'vue';
import { useRoute } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message, Modal } from '@arco-design/web-vue';
import { ApiError, type ExtensionProject } from '@mcnp/api-client';
import { PermissionGate, formatRelative } from '@mcnp/ui';
import { useApi, usePlatform } from '@/composables';

const api = useApi();
const platform = usePlatform();
const route = useRoute();
const queryClient = useQueryClient();
const instanceId = computed(() => route.params.id as string);

const query = ref('');

const { data: installed, isLoading: loadingInstalled } = useQuery({
  queryKey: computed(() => ['extensions', instanceId.value]),
  queryFn: () => api.extensions.installed(instanceId.value),
});

const { data: searchResults, isFetching: searching, refetch: runSearch } = useQuery({
  queryKey: computed(() => ['extensions', instanceId.value, 'search', query.value]),
  queryFn: () => api.extensions.search(instanceId.value, query.value),
  enabled: false,
});

const installMutation = useMutation({
  mutationFn: (project: ExtensionProject) => api.extensions.install(instanceId.value, project),
  onSuccess: (_, project) => {
    Message.success(`${project.name} 安装任务已提交`);
    void queryClient.invalidateQueries({ queryKey: ['extensions', instanceId.value] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '安装失败'),
});

function removeInstall(install: { id: string; name: string }): void {
  Modal.warning({
    title: '删除扩展',
    content: `确认从实例中删除 ${install.name}？`,
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        await api.extensions.remove(instanceId.value, install.id);
        Message.success('删除任务已提交');
        void queryClient.invalidateQueries({ queryKey: ['extensions', instanceId.value] });
      } catch (err) {
        Message.error(err instanceof ApiError ? err.message : '删除失败');
      }
    },
  });
}

const installedIds = computed(() => new Set((installed.value ?? []).map((e) => e.projectId)));
</script>

<template>
  <div class="ext-page">
    <div class="mcnp-card">
      <h3>已安装</h3>
      <ATable :data="installed ?? []" :loading="loadingInstalled" :pagination="false" row-key="id">
        <template #columns>
          <ATableColumn title="名称" data-index="name" />
          <ATableColumn title="来源" :width="110">
            <template #cell="{ record }"><ATag size="small">{{ record.source }}</ATag></template>
          </ATableColumn>
          <ATableColumn title="版本" :width="120">
            <template #cell="{ record }">
              <span class="mono">{{ record.version }}</span>
              <ATag v-if="record.updateAvailable" size="small" color="orange" style="margin-left: 4px">
                可更新 {{ record.updateAvailable }}
              </ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="文件名">
            <template #cell="{ record }"><span class="mono">{{ record.fileName }}</span></template>
          </ATableColumn>
          <ATableColumn title="安装时间" :width="110">
            <template #cell="{ record }">{{ formatRelative(record.installedAt) }}</template>
          </ATableColumn>
          <ATableColumn title="操作" :width="90">
            <template #cell="{ record }">
              <PermissionGate when="extension.manage">
                <AButton size="mini" status="danger" type="text" @click="removeInstall(record)">删除</AButton>
              </PermissionGate>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <div class="mcnp-card">
      <h3>搜索内容源</h3>
      <div class="search-bar">
        <AInputSearch v-model="query" placeholder="搜索 Modrinth / Hangar 项目" allow-clear @search="() => runSearch()" />
      </div>
      <ASpin :loading="searching" style="width: 100%">
        <AList :data="searchResults ?? []" :bordered="false">
          <AListItem v-for="project in searchResults ?? []" :key="`${project.source}:${project.projectId}`">
            <AListItemMeta :title="project.name" :description="project.summary" />
            <template #extra>
              <ASpace>
                <ATag size="small">{{ project.source }}</ATag>
                <span class="text-secondary">{{ project.downloads.toLocaleString() }} 下载</span>
                <AButton size="mini" type="text" @click="platform.openExternal(project.url)">来源页</AButton>
                <PermissionGate when="extension.manage">
                  <AButton
                    size="mini"
                    type="primary"
                    :disabled="installedIds.has(project.projectId)"
                    :loading="installMutation.isPending.value"
                    @click="installMutation.mutate(project)"
                  >
                    {{ installedIds.has(project.projectId) ? '已安装' : '安装' }}
                  </AButton>
                </PermissionGate>
              </ASpace>
            </template>
          </AListItem>
        </AList>
      </ASpin>
    </div>
  </div>
</template>

<style scoped>
.ext-page {
  display: flex;
  flex-direction: column;
  gap: var(--mcnp-space-4);
}

h3 {
  margin: 0 0 var(--mcnp-space-3);
  font-size: 15px;
}

.search-bar {
  margin-bottom: var(--mcnp-space-3);
  max-width: 420px;
}
</style>
