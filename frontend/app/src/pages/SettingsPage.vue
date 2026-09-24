<script setup lang="ts">
/** 设置：个人安全（密码/会话）与 Panel 默认路径（实例位置/备份位置）。 */

import { reactive, ref, watch } from 'vue';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { PageHeader, PermissionGate, formatRelative } from '@mcnp/ui';
import { useApi } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const auth = useAuthStore();
const queryClient = useQueryClient();

const pwd = reactive({ old: '', next: '', confirm: '' });
const pwdLoading = ref(false);

async function changePassword(): Promise<void> {
  if (pwd.next !== pwd.confirm) {
    Message.warning('两次输入的新密码不一致');
    return;
  }
  pwdLoading.value = true;
  try {
    await api.auth.changePassword(pwd.old, pwd.next);
    Message.success('密码已修改');
    pwd.old = pwd.next = pwd.confirm = '';
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '修改失败');
  } finally {
    pwdLoading.value = false;
  }
}

const { data: sessions } = useQuery({ queryKey: ['sessions'], queryFn: () => api.auth.listSessions() });

async function revoke(sessionId: string): Promise<void> {
  try {
    await api.auth.revokeSession(sessionId);
    Message.success('会话已注销');
    void queryClient.invalidateQueries({ queryKey: ['sessions'] });
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '操作失败');
  }
}

const PLATFORM_LABEL: Record<string, string> = {
  browser: '浏览器',
  'tauri-desktop': '桌面端',
  'tauri-mobile': '移动端',
  unknown: '未知',
};

// ---- Panel 默认路径 ----
const { data: panelSettings } = useQuery({ queryKey: ['panel-settings'], queryFn: () => api.panel.getSettings() });

const defaults = reactive({ defaultInstanceRoot: '', defaultBackupRoot: '' });
watch(
  panelSettings,
  (settings) => {
    if (!settings) return;
    defaults.defaultInstanceRoot = settings.defaultInstanceRoot;
    defaults.defaultBackupRoot = settings.defaultBackupRoot;
  },
  { immediate: true },
);

const defaultsMutation = useMutation({
  mutationFn: () =>
    api.panel.updateSettings({
      defaultInstanceRoot: defaults.defaultInstanceRoot.trim(),
      defaultBackupRoot: defaults.defaultBackupRoot.trim(),
    }),
  onSuccess: () => {
    Message.success('默认路径已保存');
    void queryClient.invalidateQueries({ queryKey: ['panel-settings'] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '保存失败'),
});
</script>

<template>
  <div>
    <PageHeader title="设置" :subtitle="auth.user ? `当前账户：${auth.user.displayName}（${auth.user.username}）` : ''" />

    <div class="settings-grid">
      <div class="mcnp-card">
        <h3>默认路径</h3>
        <AForm :model="defaults" layout="vertical" style="max-width: 420px">
          <AFormItem
            label="默认实例位置"
            extra="新建实例时自动填充工作目录：例如默认 /opt/servers/、实例名 fabric-1 时推导为 /opt/servers/fabric-1/"
          >
            <AInput v-model="defaults.defaultInstanceRoot" class="mono" placeholder="/opt/servers/" :disabled="!auth.has('user.manage')" />
          </AFormItem>
          <AFormItem
            label="默认备份位置"
            extra="启用备份时自动映射：例如默认 /fs/backups/、实例名 fabric-1 时，工作目录下 ./backups/ 映射为 /fs/backups/fabric-1/"
          >
            <AInput v-model="defaults.defaultBackupRoot" class="mono" placeholder="/fs/backups/" :disabled="!auth.has('user.manage')" />
          </AFormItem>
          <PermissionGate when="user.manage">
            <AButton type="primary" :loading="defaultsMutation.isPending.value" @click="defaultsMutation.mutate()">保存</AButton>
          </PermissionGate>
        </AForm>
      </div>

      <div class="mcnp-card">
        <h3>修改密码</h3>
        <AForm :model="pwd" layout="vertical" style="max-width: 360px">
          <AFormItem label="原密码"><AInputPassword v-model="pwd.old" /></AFormItem>
          <AFormItem label="新密码"><AInputPassword v-model="pwd.next" /></AFormItem>
          <AFormItem label="确认新密码"><AInputPassword v-model="pwd.confirm" /></AFormItem>
          <AButton type="primary" :loading="pwdLoading" @click="changePassword">保存</AButton>
        </AForm>
      </div>

      <div class="mcnp-card">
        <h3>登录会话与设备</h3>
        <AList :data="sessions ?? []" :bordered="false">
          <AListItem v-for="session in sessions ?? []" :key="session.id">
            <AListItemMeta
              :title="`${session.deviceName}${session.current ? '（当前会话）' : ''}`"
              :description="`${PLATFORM_LABEL[session.platform] ?? session.platform} · ${session.ip} · 最近活跃 ${formatRelative(session.lastActiveAt)}`"
            />
            <template #extra>
              <AButton v-if="!session.current" size="mini" status="danger" type="text" @click="revoke(session.id)">注销</AButton>
            </template>
          </AListItem>
        </AList>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
  gap: var(--mcnp-space-4);
}

h3 {
  margin: 0 0 var(--mcnp-space-3);
  font-size: 15px;
}
</style>
