<script setup lang="ts">
/** 环境管理：受管 Java/Node.js/Python 版本发现、安装与删除。 */

import { computed, reactive, ref } from 'vue';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message, Modal } from '@arco-design/web-vue';
import { ApiError, type ManagedRuntimeKind } from '@mcnp/api-client';
import { PageHeader, PermissionGate, formatBytes, formatRelative } from '@mcnp/ui';
import { useApi } from '@/composables';
import { useCoreStore } from '@/stores/core';

const api = useApi();
const coreStore = useCoreStore();
const queryClient = useQueryClient();

const { data: cores } = useQuery({ queryKey: ['cores'], queryFn: () => api.cores.list() });

// 环境管理是节点级能力：默认取全局选中节点，否则第一个节点
const activeCoreId = computed(() => coreStore.selectedCoreId ?? cores.value?.[0]?.id ?? '');

const { data: runtimes, isLoading } = useQuery({
  queryKey: computed(() => ['runtimes', activeCoreId.value]),
  queryFn: () => api.environments.list(activeCoreId.value),
  enabled: computed(() => activeCoreId.value !== ''),
});

const KIND_LABEL: Record<ManagedRuntimeKind, string> = { JAVA: 'Java', NODE: 'Node.js', PYTHON: 'Python' };

const installVisible = ref(false);
const installForm = reactive({ kind: 'JAVA' as ManagedRuntimeKind, version: '' });

const { data: versions } = useQuery({
  queryKey: computed(() => ['runtime-versions', activeCoreId.value, installForm.kind]),
  queryFn: () => api.environments.availableVersions(activeCoreId.value, installForm.kind),
  enabled: computed(() => installVisible.value && activeCoreId.value !== ''),
});

const installMutation = useMutation({
  mutationFn: () => api.environments.install(activeCoreId.value, installForm.kind, installForm.version),
  onSuccess: () => {
    Message.success('安装任务已提交，可在任务中心查看进度');
    installVisible.value = false;
    void queryClient.invalidateQueries({ queryKey: ['runtimes'] });
    void queryClient.invalidateQueries({ queryKey: ['tasks'] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '安装失败'),
});

function removeRuntime(runtime: { id: string; kind: ManagedRuntimeKind; version: string }): void {
  Modal.warning({
    title: '删除运行时',
    content: `确认删除 ${KIND_LABEL[runtime.kind]} ${runtime.version}？被实例引用的运行时无法删除。`,
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        await api.environments.remove(runtime.id);
        Message.success('删除任务已提交');
        void queryClient.invalidateQueries({ queryKey: ['runtimes'] });
      } catch (err) {
        Message.error(err instanceof ApiError ? err.message : '删除失败');
      }
    },
  });
}
</script>

<template>
  <div>
    <PageHeader title="环境管理" subtitle="集中管理各节点的 Java、Node.js 与 Python 运行时">
      <template #extra>
        <PermissionGate when="environment.manage">
          <AButton type="primary" :disabled="!activeCoreId" @click="installVisible = true">安装运行时</AButton>
        </PermissionGate>
      </template>
    </PageHeader>

    <div class="mcnp-card">
      <div class="toolbar">
        <span class="text-secondary">节点：</span>
        <ASelect
          :model-value="coreStore.selectedCoreId ?? undefined"
          style="width: 220px"
          placeholder="选择节点"
          @change="(v) => coreStore.select(v === undefined ? null : String(v))"
        >
          <AOption v-for="core in cores ?? []" :key="core.id" :value="core.id">{{ core.name }}</AOption>
        </ASelect>
      </div>
      <ATable :data="runtimes ?? []" :loading="isLoading" :pagination="false" row-key="id" :scroll="{ x: 1000 }">
        <template #columns>
          <ATableColumn title="类型" :width="110">
            <template #cell="{ record }"><ATag size="small">{{ KIND_LABEL[record.kind as ManagedRuntimeKind] }}</ATag></template>
          </ATableColumn>
          <ATableColumn title="版本" :width="130">
            <template #cell="{ record }"><span class="mono">{{ record.version }}</span></template>
          </ATableColumn>
          <ATableColumn title="安装路径" :width="320">
            <template #cell="{ record }"><span class="mono">{{ record.path }}</span></template>
          </ATableColumn>
          <ATableColumn title="SHA-256" :width="120">
            <template #cell="{ record }">
              <span v-if="record.sha256" class="mono">{{ record.sha256 }}…</span>
              <ATag v-else size="small" color="orange">未校验</ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="大小" :width="110">
            <template #cell="{ record }">{{ formatBytes(record.sizeBytes) }}</template>
          </ATableColumn>
          <ATableColumn title="安装时间" :width="110">
            <template #cell="{ record }">{{ formatRelative(record.installedAt) }}</template>
          </ATableColumn>
          <ATableColumn title="操作" :width="90" fixed="right">
            <template #cell="{ record }">
              <PermissionGate when="environment.manage">
                <AButton size="mini" status="danger" type="text" @click="removeRuntime(record)">删除</AButton>
              </PermissionGate>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <AModal v-model:visible="installVisible" title="安装运行时" :ok-loading="installMutation.isPending.value" @ok="installMutation.mutate()">
      <AForm :model="installForm" layout="vertical">
        <AFormItem label="类型">
          <ARadioGroup v-model="installForm.kind" @change="installForm.version = ''">
            <ARadio value="JAVA">Java</ARadio>
            <ARadio value="NODE">Node.js</ARadio>
            <ARadio value="PYTHON">Python</ARadio>
          </ARadioGroup>
        </AFormItem>
        <AFormItem label="版本" required extra="下载地址与 SHA-256 来自受信来源清单">
          <ASelect v-model="installForm.version">
            <AOption v-for="v in versions ?? []" :key="v" :value="v">{{ v }}</AOption>
          </ASelect>
        </AFormItem>
      </AForm>
    </AModal>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: var(--mcnp-space-2);
  margin-bottom: var(--mcnp-space-3);
}
</style>
