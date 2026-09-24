<script setup lang="ts">
/** 模组/插件：模组与插件分开展示，各自维护已安装列表与内容源搜索。 */

import { computed, ref } from 'vue';
import { useRoute } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message, Modal } from '@arco-design/web-vue';
import { ApiError, type ExtensionKind, type ExtensionProject } from '@mcnp/api-client';
import { useApi } from '@/composables';
import ExtensionTables from './ExtensionTables.vue';
import ExtensionSearch from './ExtensionSearch.vue';

const api = useApi();
const route = useRoute();
const queryClient = useQueryClient();
const instanceId = computed(() => route.params.id as string);

const kind = ref<ExtensionKind>('PLUGIN');
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

/** 当前形态（模组/插件）下的已安装与搜索结果 */
const installedOfKind = computed(() => (installed.value ?? []).filter((e) => e.kind === kind.value));
const resultsOfKind = computed(() => (searchResults.value ?? []).filter((p) => p.kind === kind.value));

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

/** 已安装判重：来源+项目 ID 联合（跨来源可能存在同 ID 项目）。 */
const installedIds = computed(() => new Set((installed.value ?? []).map((e) => `${e.source}:${e.projectId}`)));

function isInstalled(project: ExtensionProject): boolean {
  return installedIds.value.has(`${project.source}:${project.projectId}`);
}

function switchKind(key: string | number): void {
  kind.value = key as ExtensionKind;
  query.value = '';
}
</script>

<template>
  <!-- 内容放入各 TabPane 内部，确保渲染在 arco-tabs-content 容器中 -->
  <ATabs :active-key="kind" type="line" lazy-load @change="switchKind">
    <ATabPane key="PLUGIN" title="插件">
      <div v-if="kind === 'PLUGIN'" class="ext-page">
        <div class="mcnp-card">
          <h3>已安装插件</h3>
          <ExtensionTables
            :installed="installedOfKind"
            :loading="loadingInstalled"
            @remove="removeInstall"
          />
        </div>

        <div class="mcnp-card">
          <h3>搜索插件内容源</h3>
          <ExtensionSearch
            v-model:query="query"
            placeholder="搜索 Modrinth / Hangar 插件项目"
            :results="resultsOfKind"
            :searching="searching"
            :install-pending="installMutation.isPending.value"
            :is-installed="isInstalled"
            @search="() => runSearch()"
            @install="(p: ExtensionProject) => installMutation.mutate(p)"
          />
        </div>
      </div>
    </ATabPane>

    <ATabPane key="MOD" title="模组">
      <div v-if="kind === 'MOD'" class="ext-page">
        <div class="mcnp-card">
          <h3>已安装模组</h3>
          <ExtensionTables
            :installed="installedOfKind"
            :loading="loadingInstalled"
            @remove="removeInstall"
          />
        </div>

        <div class="mcnp-card">
          <h3>搜索模组内容源</h3>
          <ExtensionSearch
            v-model:query="query"
            placeholder="搜索 Modrinth 模组项目"
            :results="resultsOfKind"
            :searching="searching"
            :install-pending="installMutation.isPending.value"
            :is-installed="isInstalled"
            @search="() => runSearch()"
            @install="(p: ExtensionProject) => installMutation.mutate(p)"
          />
        </div>
      </div>
    </ATabPane>
  </ATabs>
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
</style>
