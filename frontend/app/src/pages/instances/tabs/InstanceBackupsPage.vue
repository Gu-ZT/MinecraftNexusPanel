<script setup lang="ts">
/**
 * 实例备份页（原型）。
 * 启用备份时自动把工作目录下的 ./backups/ 映射到默认备份根目录下的实例专属目录；
 * 容器模式同时追加容器路径映射。TODO(M3): 备份快照清单、恢复、下载 API。
 */

import { computed } from 'vue';
import { useRoute } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { PermissionGate, formatTime } from '@mcnp/ui';
import { useApi } from '@/composables';

/** 启用备份时自动追加的容器内挂载点 */
const BACKUP_CONTAINER_PATH = '/backups';

const api = useApi();
const route = useRoute();
const queryClient = useQueryClient();
const instanceId = computed(() => route.params.id as string);

const { data: instance } = useQuery({
  queryKey: computed(() => ['instances', instanceId.value]),
  queryFn: () => api.instances.get(instanceId.value),
});

const { data: panelSettings } = useQuery({ queryKey: ['panel-settings'], queryFn: () => api.panel.getSettings() });

// 原型阶段以任务中心的 BACKUP 类任务近似备份历史
const { data: tasks } = useQuery({ queryKey: ['tasks'], queryFn: () => api.tasks.list() });

const backups = computed(() =>
  (tasks.value ?? []).filter((t) => t.kind === 'BACKUP' && t.instanceId === instanceId.value),
);

/** 备份落盘目录推导：{默认备份根目录}{实例名称}/ */
const derivedTargetDir = computed(() => {
  const root = panelSettings.value?.defaultBackupRoot ?? '';
  const name = instance.value?.name.trim() ?? '';
  if (!root || !name) return '';
  return `${root.replace(/\/+$/, '')}/${name}/`;
});

const toggleMutation = useMutation({
  mutationFn: async (enable: boolean) => {
    const inst = instance.value;
    if (!inst) throw new ApiError(400, 'NOT_READY', '实例信息未加载');
    // 仅移除「备份自动挂载」（容器内路径与宿主机路径均匹配的项），不动用户手工挂载
    const isAutoBackupMount = (m: { hostPath: string; containerPath: string }): boolean =>
      m.containerPath === BACKUP_CONTAINER_PATH && m.hostPath === inst.backupTargetDir;
    if (!enable) {
      if (inst.runtimeMode !== 'CONTAINER') {
        return api.instances.updateSettings(inst.id, { backupEnabled: false });
      }
      return api.instances.updateSettings(inst.id, {
        backupEnabled: false,
        containerMounts: inst.containerMounts.filter((m) => !isAutoBackupMount(m)),
      });
    }
    if (!derivedTargetDir.value) throw new ApiError(400, 'NO_BACKUP_ROOT', '未配置默认备份根目录，请先在设置页配置');
    // 启用备份：工作目录 ./backups/ ↔ {默认备份根目录}{实例名称}/
    if (inst.runtimeMode !== 'CONTAINER') {
      return api.instances.updateSettings(inst.id, { backupEnabled: true, backupTargetDir: derivedTargetDir.value });
    }
    const mounts = inst.containerMounts.filter((m) => !isAutoBackupMount(m));
    mounts.push({ hostPath: derivedTargetDir.value, containerPath: BACKUP_CONTAINER_PATH });
    return api.instances.updateSettings(inst.id, {
      backupEnabled: true,
      backupTargetDir: derivedTargetDir.value,
      containerMounts: mounts,
    });
  },
  onSuccess: (_, enable) => {
    Message.success(enable ? '备份已启用，已自动映射备份目录' : '备份已停用');
    void queryClient.invalidateQueries({ queryKey: ['instances', instanceId.value] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '操作失败'),
});

function triggerBackup(): void {
  Message.warning('手动备份将在后续版本接入');
}
</script>

<template>
  <div class="backup-page">
    <div class="mcnp-card">
      <div class="backup-config">
        <div>
          <h3>自动备份</h3>
          <p class="text-secondary">
            <template v-if="instance?.backupEnabled">
              已启用：工作目录下的 <span class="mono">./backups/</span> 映射为
              <span class="mono">{{ instance.backupTargetDir }}</span>
            </template>
            <template v-else>
              启用后自动映射备份目录：<span class="mono">{{ derivedTargetDir || '（请先在设置页配置默认备份根目录）' }}</span>
            </template>
          </p>
        </div>
        <!-- 容器模式下开关会同时修改 containerMounts，需要两组权限 -->
        <PermissionGate :when="instance?.runtimeMode === 'CONTAINER' ? ['instance.settings.basic', 'instance.settings.container'] : 'instance.settings.basic'">
          <ASwitch
            :model-value="instance?.backupEnabled ?? false"
            :loading="toggleMutation.isPending.value"
            :disabled="!instance"
            @change="(v: string | number | boolean) => toggleMutation.mutate(Boolean(v))"
          />
        </PermissionGate>
      </div>
      <p v-if="instance?.backupEnabled && instance.runtimeMode === 'HOST'" class="text-secondary">
        HOST 模式下备份文件直接写入目标目录。
      </p>
    </div>

    <div class="mcnp-card">
      <div class="toolbar">
        <h3>备份历史</h3>
        <PermissionGate when="instance.control">
          <AButton type="primary" :disabled="!instance?.backupEnabled" @click="triggerBackup">立即备份</AButton>
        </PermissionGate>
      </div>
      <ATable :data="backups" :pagination="false" row-key="id" :scroll="{ x: 960 }">
        <template #columns>
          <ATableColumn title="备份任务" data-index="title" :width="260" />
          <ATableColumn title="结果" :width="100">
            <template #cell="{ record }">
              <ATag :color="record.status === 'SUCCESS' ? 'green' : 'red'" size="small">{{ record.status }}</ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="产物" :width="320">
            <template #cell="{ record }"><span class="mono">{{ record.message ?? '—' }}</span></template>
          </ATableColumn>
          <ATableColumn title="时间" :width="170">
            <template #cell="{ record }">{{ formatTime(record.finishedAt ?? record.createdAt) }}</template>
          </ATableColumn>
        </template>
      </ATable>
    </div>
  </div>
</template>

<style scoped>
.backup-page {
  display: flex;
  flex-direction: column;
  gap: var(--mcnp-space-4);
}

.backup-config {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--mcnp-space-4);
}

.backup-config h3,
.toolbar h3 {
  margin: 0 0 var(--mcnp-space-1);
  font-size: 15px;
}

.backup-config p {
  margin: 0;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: var(--mcnp-space-3);
}

.toolbar h3 {
  margin: 0;
}
</style>
