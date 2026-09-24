<script setup lang="ts">
/** 用户管理：用户列表、创建、禁用与重置密码。 */

import { computed, reactive, ref } from 'vue';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message, Modal } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { PageHeader, PermissionGate, formatRelative } from '@mcnp/ui';
import { useApi } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const auth = useAuthStore();
const queryClient = useQueryClient();

const { data: users, isLoading } = useQuery({ queryKey: ['users'], queryFn: () => api.users.list() });
const { data: groups } = useQuery({ queryKey: ['groups'], queryFn: () => api.users.groups() });

const createVisible = ref(false);
const form = reactive({ username: '', displayName: '', password: '', groupIds: [] as string[] });

const createMutation = useMutation({
  mutationFn: () => api.users.create({ ...form }),
  onSuccess: () => {
    Message.success('用户已创建');
    createVisible.value = false;
    form.username = '';
    form.displayName = '';
    form.password = '';
    form.groupIds = [];
    void queryClient.invalidateQueries({ queryKey: ['users'] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '创建失败'),
});

function groupNames(groupIds: string[]): string {
  return groupIds.map((id) => groups.value?.find((g) => g.id === id)?.name ?? id).join('、');
}

function toggleDisabled(user: { id: string; username: string; disabled: boolean }): void {
  // 快照目标状态：Mock 会原地修改缓存对象，完成后不可再读 user.disabled 构造文案
  const willDisable = !user.disabled;
  void api.users
    .update(user.id, { disabled: willDisable })
    .then(() => {
      Message.success(willDisable ? '已禁用' : '已启用');
      void queryClient.invalidateQueries({ queryKey: ['users'] });
    })
    .catch((err) => Message.error(err instanceof ApiError ? err.message : '操作失败'));
}

function resetPassword(user: { id: string; username: string }): void {
  Modal.open({
    title: `重置密码：${user.username}`,
    content: '新密码将立即生效并记录审计（内容脱敏）',
    onOk: async () => {
      try {
        const newPassword = `reset-${Math.random().toString(36).slice(2, 10)}`;
        await api.users.resetPassword(user.id, newPassword);
        Message.success(`密码已重置（原型展示新密码：${newPassword}）`);
      } catch (err) {
        Message.error(err instanceof ApiError ? err.message : '重置失败');
      }
    },
  });
}

function removeUser(user: { id: string; username: string }): void {
  Modal.warning({
    title: '删除用户',
    content: `确认删除用户 ${user.username}？`,
    okButtonProps: { status: 'danger' },
    onOk: async () => {
      try {
        await api.users.remove(user.id);
        Message.success('已删除');
        void queryClient.invalidateQueries({ queryKey: ['users'] });
      } catch (err) {
        Message.error(err instanceof ApiError ? err.message : '删除失败');
      }
    },
  });
}

const isSelf = computed(() => (id: string) => auth.user?.id === id);
</script>

<template>
  <div>
    <PageHeader title="用户" subtitle="管理面板账户及其所属用户组">
      <template #extra>
        <PermissionGate when="user.manage">
          <AButton type="primary" @click="createVisible = true">新建用户</AButton>
        </PermissionGate>
      </template>
    </PageHeader>

    <div class="mcnp-card">
      <ATable :data="users ?? []" :loading="isLoading" :pagination="false" row-key="id" :scroll="{ x: 960 }">
        <template #columns>
          <ATableColumn title="用户名" data-index="username" :width="120" />
          <ATableColumn title="显示名" data-index="displayName" :width="140" />
          <ATableColumn title="用户组" :width="200">
            <template #cell="{ record }">{{ groupNames(record.groupIds) }}</template>
          </ATableColumn>
          <ATableColumn title="状态" :width="90">
            <template #cell="{ record }">
              <ATag :color="record.disabled ? 'red' : 'green'" size="small">{{ record.disabled ? '已禁用' : '正常' }}</ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="最近登录" :width="110">
            <template #cell="{ record }">{{ formatRelative(record.lastLoginAt) }}</template>
          </ATableColumn>
          <ATableColumn title="操作" :width="270" fixed="right">
            <template #cell="{ record }">
              <PermissionGate when="user.manage">
                <ASpace>
                  <AButton size="mini" @click="toggleDisabled(record)">{{ record.disabled ? '启用' : '禁用' }}</AButton>
                  <AButton size="mini" @click="resetPassword(record)">重置密码</AButton>
                  <AButton v-if="!isSelf(record.id)" size="mini" status="danger" type="text" @click="removeUser(record)">删除</AButton>
                </ASpace>
              </PermissionGate>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <AModal v-model:visible="createVisible" title="新建用户" :ok-loading="createMutation.isPending.value" @ok="createMutation.mutate()">
      <AForm :model="form" layout="vertical">
        <AFormItem label="用户名" required><AInput v-model="form.username" /></AFormItem>
        <AFormItem label="显示名"><AInput v-model="form.displayName" /></AFormItem>
        <AFormItem label="初始密码" required><AInputPassword v-model="form.password" /></AFormItem>
        <AFormItem label="用户组" required>
          <ASelect v-model="form.groupIds" multiple>
            <AOption v-for="group in groups ?? []" :key="group.id" :value="group.id">{{ group.name }}</AOption>
          </ASelect>
        </AFormItem>
      </AForm>
    </AModal>
  </div>
</template>
