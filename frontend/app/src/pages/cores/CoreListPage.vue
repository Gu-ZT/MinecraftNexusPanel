<script setup lang="ts">
/** Core 节点列表：添加节点、连通性测试与移除。 */

import { reactive, ref } from 'vue';
import { useRouter } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { CoreStatusBadge, DangerConfirmButton, PageHeader, PermissionGate, formatRelative } from '@mcnp/ui';
import { useApi } from '@/composables';

const api = useApi();
const router = useRouter();
const queryClient = useQueryClient();

const { data: cores, isLoading } = useQuery({ queryKey: ['cores'], queryFn: () => api.cores.list() });

const addVisible = ref(false);
const addForm = reactive({ name: '', address: '', psk: '' });

const createMutation = useMutation({
  mutationFn: () => api.cores.create({ ...addForm }),
  onSuccess: () => {
    Message.success('节点已添加，建议立即测试连通性');
    addVisible.value = false;
    addForm.name = '';
    addForm.address = '';
    addForm.psk = '';
    void queryClient.invalidateQueries({ queryKey: ['cores'] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '添加失败'),
});

const removeMutation = useMutation({
  mutationFn: (coreId: string) => api.cores.remove(coreId),
  onSuccess: () => {
    Message.success('节点移除任务已提交');
    void queryClient.invalidateQueries({ queryKey: ['cores'] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '移除失败'),
});

const testing = ref<string | null>(null);

async function test(coreId: string): Promise<void> {
  testing.value = coreId;
  try {
    const { latencyMs } = await api.cores.testConnection(coreId);
    Message.success(`握手成功，延迟 ${latencyMs}ms`);
    void queryClient.invalidateQueries({ queryKey: ['cores'] });
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '连接失败');
  } finally {
    testing.value = null;
  }
}
</script>

<template>
  <div>
    <PageHeader title="Core 节点" subtitle="查看各节点的在线状态、版本与资源使用情况">
      <template #extra>
        <PermissionGate when="core.manage">
          <AButton type="primary" @click="addVisible = true">添加节点</AButton>
        </PermissionGate>
      </template>
    </PageHeader>

    <div class="mcnp-card">
      <ATable :data="cores ?? []" :loading="isLoading" :pagination="false" row-key="id" :scroll="{ x: 1200 }">
        <template #columns>
          <ATableColumn title="名称" data-index="name" :width="160" />
          <ATableColumn title="地址" :width="190">
            <template #cell="{ record }"><span class="mono">{{ record.address }}</span></template>
          </ATableColumn>
          <ATableColumn title="状态" :width="100">
            <template #cell="{ record }"><CoreStatusBadge :status="record.status" /></template>
          </ATableColumn>
          <ATableColumn title="系统" :width="160">
            <template #cell="{ record }">{{ record.os }} / {{ record.arch }}</template>
          </ATableColumn>
          <ATableColumn title="能力" :width="220">
            <template #cell="{ record }">
              <ATag v-for="cap in record.capabilities" :key="cap" size="small" style="margin-right: 4px">{{ cap }}</ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="最近心跳" :width="110">
            <template #cell="{ record }">{{ formatRelative(record.lastSeenAt) }}</template>
          </ATableColumn>
          <ATableColumn title="操作" :width="260" fixed="right">
            <template #cell="{ record }">
              <ASpace>
                <AButton size="small" @click="router.push(`/cores/${record.id}`)">详情</AButton>
                <PermissionGate when="core.manage">
                  <AButton size="small" :loading="testing === record.id" @click="test(record.id)">测试连接</AButton>
                  <DangerConfirmButton
                    confirm-text="移除节点后其上的实例将从面板断开，确认移除？"
                    size="small"
                    @confirm="removeMutation.mutate(record.id)"
                  >
                    移除
                  </DangerConfirmButton>
                </PermissionGate>
              </ASpace>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <AModal v-model:visible="addVisible" title="添加 Core 节点" @ok="createMutation.mutate()">
      <AForm :model="addForm" layout="vertical">
        <AFormItem label="节点名称" required><AInput v-model="addForm.name" placeholder="上海节点-02" /></AFormItem>
        <AFormItem label="地址 (host:port)" required><AInput v-model="addForm.address" placeholder="192.168.10.12:24444" /></AFormItem>
        <AFormItem label="节点连接密钥 (PSK)" required extra="密钥经信封加密落库，不会明文回显">
          <AInputPassword v-model="addForm.psk" />
        </AFormItem>
      </AForm>
    </AModal>
  </div>
</template>
