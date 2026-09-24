<script setup lang="ts">
/** 用户组：权限矩阵与实例范围（instanceScopes：Core/实例/标签选择器）。 */

import { reactive, ref, computed } from 'vue';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message, Modal } from '@arco-design/web-vue';
import { ALL_PERMISSIONS, ApiError, type Group, type Permission } from '@mcnp/api-client';
import { PageHeader, PermissionGate } from '@mcnp/ui';
import { useApi } from '@/composables';

const api = useApi();
const queryClient = useQueryClient();

const { data: groups, isLoading } = useQuery({ queryKey: ['groups'], queryFn: () => api.users.groups() });
const { data: cores } = useQuery({ queryKey: ['cores'], queryFn: () => api.cores.list() });
const { data: instances } = useQuery({ queryKey: ['instances', 'all'], queryFn: () => api.instances.list() });

const editorVisible = ref(false);
const editingGroup = ref<Group | null>(null);
const form = reactive({
  name: '',
  description: '',
  permissions: [] as Permission[],
  scopeCoreIds: [] as string[],
  scopeInstanceIds: [] as string[],
  scopeTags: [] as string[],
});
const formModel = computed(() => form);

function openEditor(group?: Group): void {
  editingGroup.value = group ?? null;
  form.name = group?.name ?? '';
  form.description = group?.description ?? '';
  form.permissions = [...(group?.permissions ?? [])];
  form.scopeCoreIds = [...(group?.instanceScopes.coreIds ?? [])];
  form.scopeInstanceIds = [...(group?.instanceScopes.instanceIds ?? [])];
  form.scopeTags = [...(group?.instanceScopes.tagSelectors ?? [])];
  editorVisible.value = true;
}

const saveMutation = useMutation({
  mutationFn: () => {
    const input = {
      name: form.name,
      description: form.description,
      permissions: form.permissions,
      instanceScopes: {
        coreIds: form.scopeCoreIds,
        instanceIds: form.scopeInstanceIds,
        tagSelectors: form.scopeTags,
      },
    };
    return editingGroup.value ? api.users.updateGroup(editingGroup.value.id, input) : api.users.createGroup(input);
  },
  onSuccess: () => {
    Message.success('已保存');
    editorVisible.value = false;
    void queryClient.invalidateQueries({ queryKey: ['groups'] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '保存失败'),
});

function removeGroup(group: Group): void {
  Modal.warning({
    title: '删除用户组',
    content: `确认删除「${group.name}」？组内用户将失去对应权限。`,
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        await api.users.removeGroup(group.id);
        Message.success('已删除');
        void queryClient.invalidateQueries({ queryKey: ['groups'] });
      } catch (err) {
        Message.error(err instanceof ApiError ? err.message : '删除失败');
      }
    },
  });
}

function scopeLabel(group: Group): string {
  const s = group.instanceScopes;
  if (s.coreIds.length === 0 && s.instanceIds.length === 0 && s.tagSelectors.length === 0) return '全部实例';
  const parts: string[] = [];
  if (s.coreIds.length > 0) parts.push(`${s.coreIds.length} 个节点`);
  if (s.instanceIds.length > 0) parts.push(`${s.instanceIds.length} 个实例`);
  if (s.tagSelectors.length > 0) parts.push(`标签 ${s.tagSelectors.join('、')}`);
  return parts.join(' + ');
}
</script>

<template>
  <div>
    <PageHeader title="用户组" subtitle="定义权限组合与可见实例范围，批量授予组内成员">
      <template #extra>
        <PermissionGate when="user.manage">
          <AButton type="primary" @click="openEditor()">新建用户组</AButton>
        </PermissionGate>
      </template>
    </PageHeader>

    <div class="mcnp-card">
      <ATable :data="groups ?? []" :loading="isLoading" :pagination="false" row-key="id" :scroll="{ x: 840 }">
        <template #columns>
          <ATableColumn title="名称" :width="140">
            <template #cell="{ record }">
              {{ record.name }}
              <ATag v-if="record.builtIn" size="small" color="arcoblue" style="margin-left: 4px">内置</ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="描述" data-index="description" :width="260" ellipsis tooltip />
          <ATableColumn title="权限点数" :width="90">
            <template #cell="{ record }">{{ record.permissions.length }}</template>
          </ATableColumn>
          <ATableColumn title="实例范围" :width="180">
            <template #cell="{ record }">{{ scopeLabel(record) }}</template>
          </ATableColumn>
          <ATableColumn title="操作" :width="180" fixed="right">
            <template #cell="{ record }">
              <PermissionGate when="user.manage">
                <ASpace>
                  <AButton size="mini" @click="openEditor(record)">编辑</AButton>
                  <AButton v-if="!record.builtIn" size="mini" status="danger" type="text" @click="removeGroup(record)">删除</AButton>
                </ASpace>
              </PermissionGate>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <AModal
      v-model:visible="editorVisible"
      :title="editingGroup ? `编辑用户组：${editingGroup.name}` : '新建用户组'"
      width="720px"
      :ok-loading="saveMutation.isPending.value"
      @ok="saveMutation.mutate()"
    >
      <AForm :model="formModel" layout="vertical">
        <AFormItem label="名称" required><AInput v-model="form.name" :disabled="editingGroup?.builtIn" /></AFormItem>
        <AFormItem label="描述"><AInput v-model="form.description" /></AFormItem>
        <AFormItem label="权限点" extra="内置管理员组权限不可裁剪">
          <ACheckboxGroup v-model="form.permissions" :disabled="editingGroup?.builtIn">
            <div class="perm-grid">
              <ACheckbox v-for="perm in ALL_PERMISSIONS" :key="perm" :value="perm">
                <span class="mono perm-label">{{ perm }}</span>
              </ACheckbox>
            </div>
          </ACheckboxGroup>
        </AFormItem>
        <AFormItem label="节点范围" extra="三个范围全部留空 = 不限制（慎用）">
          <ASelect v-model="form.scopeCoreIds" multiple allow-clear>
            <AOption v-for="core in cores ?? []" :key="core.id" :value="core.id">{{ core.name }}</AOption>
          </ASelect>
        </AFormItem>
        <AFormItem label="实例范围">
          <ASelect v-model="form.scopeInstanceIds" multiple allow-clear>
            <AOption v-for="inst in instances ?? []" :key="inst.id" :value="inst.id">{{ inst.name }}</AOption>
          </ASelect>
        </AFormItem>
        <AFormItem label="标签选择器">
          <AInputTag v-model="form.scopeTags" placeholder="如 production" allow-clear />
        </AFormItem>
      </AForm>
    </AModal>
  </div>
</template>

<style scoped>
.perm-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: var(--mcnp-space-1) var(--mcnp-space-3);
}

.perm-label {
  font-size: 12px;
}
</style>
