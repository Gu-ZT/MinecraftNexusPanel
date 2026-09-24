<script setup lang="ts">
/** 镜像管理：镜像列表、拉取/删除、构建入口、构建历史与镜像仓库管理。 */

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

const { data: registries } = useQuery({
  queryKey: computed(() => ['image-registries', activeCoreId.value]),
  queryFn: () => api.images.registries(activeCoreId.value),
  enabled: computed(() => activeCoreId.value !== ''),
});

const pullVisible = ref(false);
const pullForm = reactive({ reference: '', registryId: '' });
const buildVisible = ref(false);
const buildForm = reactive({ tag: 'mcnp/custom:latest', dockerfile: 'FROM eclipse-temurin:21-jre\nWORKDIR /data\nCOPY server.jar /data/server.jar\nENTRYPOINT ["java","-jar","server.jar"]' });
const registryVisible = ref(false);
const registryForm = reactive({ name: '', url: '' });

const pullMutation = useMutation({
  mutationFn: () => api.images.pull(activeCoreId.value, pullForm.reference, pullForm.registryId || undefined),
  onSuccess: () => {
    Message.success('拉取任务已提交');
    pullVisible.value = false;
    pullForm.reference = '';
    pullForm.registryId = '';
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

const addRegistryMutation = useMutation({
  mutationFn: () => api.images.addRegistry(activeCoreId.value, { name: registryForm.name, url: registryForm.url }),
  onSuccess: () => {
    Message.success('仓库已添加');
    registryVisible.value = false;
    registryForm.name = '';
    registryForm.url = '';
    void queryClient.invalidateQueries({ queryKey: ['image-registries', activeCoreId.value] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '添加失败'),
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

function removeRegistry(registry: { id: string; name: string }): void {
  Modal.warning({
    title: '删除仓库',
    content: `确认删除仓库 ${registry.name}？`,
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        await api.images.removeRegistry(activeCoreId.value, registry.id);
        Message.success('仓库已删除');
        void queryClient.invalidateQueries({ queryKey: ['image-registries', activeCoreId.value] });
      } catch (err) {
        Message.error(err instanceof ApiError ? err.message : '删除失败');
      }
    },
  });
}
</script>

<template>
  <div>
    <PageHeader title="镜像管理" subtitle="拉取、构建与管理各节点的容器镜像">
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
      <ATable :data="images ?? []" :loading="isLoading" :pagination="false" row-key="id" :scroll="{ x: 800 }">
        <template #columns>
          <ATableColumn title="标签" :width="320">
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
          <ATableColumn title="操作" :width="90" fixed="right">
            <template #cell="{ record }">
              <PermissionGate when="image.manage">
                <AButton size="mini" status="danger" type="text" @click="removeImage(record)">删除</AButton>
              </PermissionGate>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <div class="mcnp-card" style="margin-bottom: 16px">
      <div class="card-head">
        <h3>镜像仓库</h3>
        <PermissionGate when="image.manage">
          <AButton size="small" @click="registryVisible = true">添加仓库</AButton>
        </PermissionGate>
      </div>
      <ATable :data="registries ?? []" :pagination="false" row-key="id" :scroll="{ x: 720 }">
        <template #columns>
          <ATableColumn title="查找顺序" :width="90">
            <template #cell="{ rowIndex }">{{ rowIndex + 1 }}</template>
          </ATableColumn>
          <ATableColumn title="名称" data-index="name" :width="200" />
          <ATableColumn title="地址" :width="320">
            <template #cell="{ record }"><span class="mono">{{ record.url }}</span></template>
          </ATableColumn>
          <ATableColumn title="操作" :width="80" fixed="right">
            <template #cell="{ record }">
              <PermissionGate when="image.manage">
                <AButton size="mini" status="danger" type="text" @click="removeRegistry(record)">删除</AButton>
              </PermissionGate>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <div class="mcnp-card">
      <h3>构建历史</h3>
      <ATable :data="builds ?? []" :pagination="false" row-key="id" :scroll="{ x: 720 }">
        <template #columns>
          <ATableColumn title="目标标签" :width="320">
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
          <ATableColumn title="操作" :width="110" fixed="right">
            <template #cell="{ record }">
              <AButton size="mini" @click="router.push(`/images/builds/${record.id}`)">构建日志</AButton>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <AModal v-model:visible="pullVisible" title="拉取镜像" :ok-loading="pullMutation.isPending.value" @ok="pullMutation.mutate()">
      <AForm :model="pullForm" layout="vertical">
        <AFormItem label="镜像引用" required>
          <AInput v-model="pullForm.reference" class="mono" placeholder="itzg/minecraft-server:latest" />
        </AFormItem>
        <AFormItem label="目标仓库">
          <ASelect v-model="pullForm.registryId" allow-clear placeholder="自动（按上表顺序尝试）">
            <AOption v-for="(reg, idx) in registries ?? []" :key="reg.id" :value="reg.id">
              {{ idx + 1 }}. {{ reg.name }}（{{ reg.url }}）
            </AOption>
          </ASelect>
        </AFormItem>
      </AForm>
    </AModal>

    <AModal
      v-model:visible="registryVisible"
      title="添加镜像仓库"
      :ok-loading="addRegistryMutation.isPending.value"
      :ok-button-props="{ disabled: !registryForm.name.trim() || !registryForm.url.trim() }"
      @ok="addRegistryMutation.mutate()"
    >
      <AForm :model="registryForm" layout="vertical">
        <AFormItem label="名称" required><AInput v-model="registryForm.name" placeholder="Docker Hub（官方）" /></AFormItem>
        <AFormItem label="地址" required><AInput v-model="registryForm.url" class="mono" placeholder="https://registry-1.docker.io" /></AFormItem>
      </AForm>
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

.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--mcnp-space-3);
}

.card-head h3 {
  margin: 0;
}
</style>
