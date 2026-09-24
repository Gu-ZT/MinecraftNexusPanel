<script setup lang="ts">
/** 实例详情外壳：头部控制按钮 + 功能 Tab（终端/文件/配置/扩展/计划/备份/设置）。 */

import { computed, onBeforeUnmount, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError, type Permission } from '@mcnp/api-client';
import { DangerConfirmButton, InstanceStateTag, PageHeader } from '@mcnp/ui';
import { useApi, useRealtime } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const realtime = useRealtime();
const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const queryClient = useQueryClient();

const instanceId = computed(() => route.params.id as string);

const { data: instance } = useQuery({
  queryKey: computed(() => ['instances', instanceId.value]),
  queryFn: () => api.instances.get(instanceId.value),
});

// 状态变更以事件为准（PLAN 第 6 节），事件到达后刷新实例数据
const off = realtime.subscribe({ kind: 'instance-state', instanceId: instanceId.value }, () => {
  void queryClient.invalidateQueries({ queryKey: ['instances'] });
});
onBeforeUnmount(off);

interface TabItem {
  key: string;
  label: string;
  permission: Permission[];
}

const TABS: TabItem[] = [
  { key: 'console', label: '终端', permission: ['instance.console.read'] },
  { key: 'files', label: '文件', permission: ['file.read'] },
  { key: 'configs', label: '配置', permission: ['config.read'] },
  { key: 'extensions', label: '模组/插件', permission: ['extension.read'] },
  { key: 'schedules', label: '计划任务', permission: ['schedule.read'] },
  { key: 'backups', label: '备份', permission: ['instance.read'] },
  { key: 'settings', label: '设置', permission: ['instance.read'] },
];

const visibleTabs = computed(() => TABS.filter((t) => auth.hasAll(t.permission)));
const activeTab = computed(() => (route.path.split('/').pop() ?? 'console'));

function switchTab(key: string): void {
  void router.push(`/instances/${instanceId.value}/${key}`);
}

const busy = ref(false);

async function control(action: 'start' | 'stop' | 'restart'): Promise<void> {
  busy.value = true;
  try {
    await api.instances[action](instanceId.value);
    await queryClient.invalidateQueries({ queryKey: ['instances'] });
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '操作失败');
  } finally {
    busy.value = false;
  }
}

async function kill(): Promise<void> {
  try {
    await api.instances.kill(instanceId.value);
    await queryClient.invalidateQueries({ queryKey: ['instances'] });
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '操作失败');
  }
}
</script>

<template>
  <div>
    <PageHeader
      :title="instance ? `${instance.name}` : '实例'"
      :subtitle="instance ? `${instance.serverType} ${instance.version} · ${instance.workDir}` : ''"
    >
      <template #extra>
        <ASpace v-if="instance">
          <InstanceStateTag :state="instance.state" />
          <template v-if="auth.has('instance.control')">
            <AButton
              v-if="['CREATED', 'STOPPED', 'FAILED'].includes(instance.state)"
              type="primary"
              :loading="busy"
              @click="control('start')"
            >
              启动
            </AButton>
            <AButton v-if="instance.state === 'RUNNING'" :loading="busy" @click="control('restart')">重启</AButton>
            <AButton v-if="instance.state === 'RUNNING'" :loading="busy" @click="control('stop')">停止</AButton>
            <DangerConfirmButton
              v-if="instance.state === 'RUNNING' || instance.state === 'STARTING'"
              confirm-text="强制终止会丢失未保存的数据，确认？"
              @confirm="kill"
            >
              强制终止
            </DangerConfirmButton>
          </template>
        </ASpace>
      </template>
    </PageHeader>

    <ATabs :active-key="activeTab" type="card-gutter" lazy-load class="instance-detail-tabs" @change="(key: string | number) => switchTab(String(key))">
      <!-- 内容放入各 TabPane 内部，确保渲染在 arco-tabs-content 容器中 -->
      <ATabPane v-for="tab in visibleTabs" :key="tab.key" :title="tab.label">
        <RouterView v-if="activeTab === tab.key" :key="route.fullPath" />
      </ATabPane>
    </ATabs>
  </div>
</template>
