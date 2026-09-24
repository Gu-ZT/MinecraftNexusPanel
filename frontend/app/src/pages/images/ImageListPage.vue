<script setup lang="ts">
/** 镜像管理：镜像列表、拉取/删除、构建入口与构建历史。 */

import { computed, reactive, ref } from 'vue';
import { useRouter } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message, Modal } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { PageHeader, PermissionGate, formatBytes, formatRelative, formatTime } from '@mcnp/ui';
import { useApi } from '@/composables';
import { useCoreStore } from '@/stores/core';

const api = useApi();
const router = useRouter();
const coreStore = useCoreStore();
const queryClient = useQueryClient();

const { data: cores } = useQuery({ queryKey: ['cores'], queryFn: () => api.cores.list() });
const activeCoreId = computed(() => coreStore.selectedCoreId ?? cores.value?.[0]?.id ?? '');

const { data: images, isLoading } = useQuery({
  queryKey: computed(() => ['images', activeCoreId.value]),
  queryFn: () => api.images.list(activeCoreId.value),
  enabled: computed(() => activeCoreId.value !== ''),
});

const { data: builds } = useQuery({
  queryKey: computed(() => ['image-builds', activeCoreId.value]),
  queryFn: () => api.images.builds(activeCoreId.value),
  enabled: computed(() => activeCoreId.value !== ''),
});

const pullVisible = ref(false);
const pullRef = ref('');
const buildVisible = ref(false);
const buildForm = reactive({ tag: 'mcnp/custom:latest', dockerfile: 'FROM eclipse-temurin:21-jre\nWORKDIR /data\nCOPY server.jar /data/server.jar\nENTRYPOINT ["java","-jar","server.jar"]' });

const pullMutation = useMutation({
  mutationFn: () => api.images.pull(activeCoreId.value, pullRef.value),
  onSuccess: () => {
    Message.success('拉取任务已提交');
    pullVisible.value = false;
    pullRef.value = '';
    void queryClient.invalidateQueries({ queryKey: ['images'] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '拉取失败'),
});

const buildMutation = useMutation({
  mutationFn: () => api.images.build(activeCoreId.value, buildForm.tag, buildForm.dockerfile),
  onSuccess: ({ buildId }) => {
    Message.success('构建已开始');
    buildVisible.value = false;
    void router.push(`/images/builds/${buildId}`);
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '构建失败'),
});

function removeImage(image: { id: string; repoTags: string[] }): void {
  Modal.warning({
    title: '删除镜像',
    content: `确认删除 ${image.repoTags[0] ?? image.id}？`,
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        await api.images.remove(activeCoreId.value, image.id);
        Message.success('删除任务已提交');
        void queryClient.invalidateQueries({ queryKey: ['images'] });
      } catch (err) {
        Message.error(err instanceof ApiError ? err.message : '删除失败');
      }
    },
  });
}
</script>

<template>
  <div>
    <PageHeader title="镜像管理" subtitle="Core 经 Docker Engine API 管理镜像；默认禁止特权容器与越界挂载">
      <template #extra>
        <PermissionGate when="image.manage">
          <ASpace>
            <AButton @click="pullVisible = true">拉取镜像</AButton>
            <PermissionGate when="image.build">
              <AButton type="primary" @click="buildVisible = true">构建镜像</AButton>
            </PermissionGate>
          </ASpace>
        </PermissionGate>
      </template>
    </PageHeader>

    <div class="mcnp-card" style="margin-bottom: 16px">
      <h3>镜像</h3>
      <ATable :data="images ?? []" :loading="isLoading" :pagination="false" row-key="id">
        <template #columns>
          <ATableColumn title="标签">
            <template #cell="{ record }">
              <ATag v-for="tag in record.repoTags" :key="tag" size="small" class="mono" style="margin-right: 4px">{{ tag }}</ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="大小" :width="120">
            <template #cell="{ record }">{{ formatBytes(record.sizeBytes) }}</template>
          </ATableColumn>
          <ATableColumn title="创建时间" :width="120">
            <template #cell="{ record }">{{ formatRelative(record.createdAt) }}</template>
          </ATableColumn>
          <ATableColumn title="操作" :width="90">
            <template #cell="{ record }">
              <PermissionGate when="image.manage">
                <AButton size="mini" status="danger" type="text" @click="removeImage(record)">删除</AButton>
              </PermissionGate>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <div class="mcnp-card">
      <h3>构建历史</h3>
      <ATable :data="builds ?? []" :pagination="false" row-key="id">
        <template #columns>
          <ATableColumn title="目标标签">
            <template #cell="{ record }"><span class="mono">{{ record.tag }}</span></template>
          </ATableColumn>
          <ATableColumn title="状态" :width="100">
            <template #cell="{ record }">
              <ATag :color="record.status === 'SUCCESS' ? 'green' : record.status === 'RUNNING' ? 'arcoblue' : 'red'" size="small">
                {{ record.status }}
              </ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="开始时间" :width="170">
            <template #cell="{ record }">{{ formatTime(record.startedAt) }}</template>
          </ATableColumn>
          <ATableColumn title="操作" :width="110">
            <template #cell="{ record }">
              <AButton size="mini" @click="router.push(`/images/builds/${record.id}`)">构建日志</AButton>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <AModal v-model:visible="pullVisible" title="拉取镜像" :ok-loading="pullMutation.isPending.value" @ok="pullMutation.mutate()">
      <AInput v-model="pullRef" class="mono" placeholder="itzg/minecraft-server:latest" />
    </AModal>

    <AModal v-model:visible="buildVisible" title="构建镜像" width="640px" :ok-loading="buildMutation.isPending.value" @ok="buildMutation.mutate()">
      <AForm :model="buildForm" layout="vertical">
        <AFormItem label="目标标签"><AInput v-model="buildForm.tag" class="mono" /></AFormItem>
        <AFormItem label="Dockerfile"><ATextarea v-model="buildForm.dockerfile" class="mono" :auto-size="{ minRows: 8, maxRows: 16 }" /></AFormItem>
      </AForm>
    </AModal>
  </div>
</template>

<style scoped>
h3 {
  margin: 0 0 var(--mcnp-space-3);
  font-size: 15px;
}
</style>
