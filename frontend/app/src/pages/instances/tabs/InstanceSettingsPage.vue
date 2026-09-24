<script setup lang="ts">
/** 实例设置：基本/启动/路径/容器/CPU 五组，各组对应独立权限点（PLAN 4.4）。 */

import { computed, reactive, watch } from 'vue';
import { useRoute } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError, type CpuPolicyMode, type Permission } from '@mcnp/api-client';
import { useApi } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const route = useRoute();
const queryClient = useQueryClient();
const auth = useAuthStore();
const instanceId = computed(() => route.params.id as string);

const { data: instance } = useQuery({
  queryKey: computed(() => ['instances', instanceId.value]),
  queryFn: () => api.instances.get(instanceId.value),
});

const { data: cpuPolicy } = useQuery({
  queryKey: computed(() => ['cpu-policy', instanceId.value]),
  queryFn: () => api.instances.cpuPolicy(instanceId.value),
});

const { data: topology } = useQuery({
  queryKey: computed(() => ['cores', instance.value?.coreId ?? '', 'cpu-topology']),
  queryFn: () => api.cores.cpuTopology(instance.value?.coreId ?? ''),
  enabled: computed(() => instance.value !== undefined),
});

const basic = reactive({ name: '', expiresInDays: 0, tags: '' });
const launch = reactive({ launchCommand: '', updateCommand: '', supervisorMode: 'DIRECT' as 'DIRECT' | 'MCDR' });
const path = reactive({ workDir: '' });
const container = reactive({ runtimeMode: 'HOST' as 'HOST' | 'CONTAINER', image: '', ports: '', env: '' });
const cpu = reactive({ mode: 'SHARED' as CpuPolicyMode, cpuSet: [] as number[], exclusive: false, numaNodeId: null as number | null });

watch(
  instance,
  (inst) => {
    if (!inst) return;
    basic.name = inst.name;
    basic.tags = inst.tags.join(', ');
    basic.expiresInDays = inst.expiresAt ? Math.max(0, Math.round((inst.expiresAt - Date.now()) / 86_400_000)) : 0;
    launch.launchCommand = inst.launchCommand;
    launch.updateCommand = inst.updateCommand ?? '';
    launch.supervisorMode = inst.supervisorMode;
    path.workDir = inst.workDir;
    container.runtimeMode = inst.runtimeMode;
    container.image = inst.containerImage ?? '';
    container.ports = inst.containerPorts.join('\n');
    container.env = Object.entries(inst.containerEnv)
      .map(([k, v]) => `${k}=${v}`)
      .join('\n');
  },
  { immediate: true },
);

watch(
  cpuPolicy,
  (policy) => {
    if (!policy) return;
    cpu.mode = policy.mode;
    cpu.cpuSet = [...policy.cpuSet];
    cpu.exclusive = policy.exclusive;
    cpu.numaNodeId = policy.numaNodeId;
  },
  { immediate: true },
);

function can(permission: Permission): boolean {
  return auth.has(permission);
}

const saveMutation = useMutation({
  mutationFn: ({ group }: { group: string }) => {
    if (group === 'basic') {
      return api.instances.updateSettings(instanceId.value, {
        name: basic.name,
        tags: basic.tags.split(',').map((t) => t.trim()).filter(Boolean),
        expiresAt: basic.expiresInDays > 0 ? Date.now() + basic.expiresInDays * 86_400_000 : null,
      });
    }
    if (group === 'launch') {
      return api.instances.updateSettings(instanceId.value, {
        launchCommand: launch.launchCommand,
        updateCommand: launch.updateCommand || null,
        supervisorMode: launch.supervisorMode,
      });
    }
    if (group === 'path') {
      return api.instances.updateSettings(instanceId.value, { workDir: path.workDir });
    }
    // container：镜像、端口映射与环境变量
    const envEntries = container.env
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => {
        const idx = line.indexOf('=');
        if (idx <= 0) throw new ApiError(400, 'BAD_ENV', `环境变量格式错误：${line}（应为 KEY=VALUE）`);
        return [line.slice(0, idx), line.slice(idx + 1)] as const;
      });
    return api.instances.updateSettings(instanceId.value, {
      runtimeMode: container.runtimeMode,
      containerImage: container.image || null,
      containerPorts: container.ports.split('\n').map((p) => p.trim()).filter(Boolean),
      containerEnv: Object.fromEntries(envEntries),
    });
  },
  onSuccess: () => {
    Message.success('设置已保存');
    void queryClient.invalidateQueries({ queryKey: ['instances', instanceId.value] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '保存失败'),
});

const cpuMutation = useMutation({
  mutationFn: () =>
    api.instances.updateCpuPolicy(instanceId.value, {
      mode: cpu.mode,
      cpuSet: cpu.cpuSet,
      exclusive: cpu.exclusive,
      numaNodeId: cpu.numaNodeId,
    }),
  onSuccess: (policy) => {
    Message.success(`CPU 策略已提交（状态：${policy.status}）`);
    void queryClient.invalidateQueries({ queryKey: ['cpu-policy', instanceId.value] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '保存失败'),
});

const CPU_STATUS_LABEL: Record<string, string> = { requested: '已请求', applied: '已生效', degraded: '已降级' };
</script>

<template>
  <ATabs type="line">
    <!-- 基本设置 -->
    <ATabPane key="basic" title="基本">
      <AForm :model="basic" layout="vertical" class="settings-form">
        <AFormItem label="实例名称"><AInput v-model="basic.name" :disabled="!can('instance.settings.basic')" /></AFormItem>
        <AFormItem label="到期时间（天，0 为不限）">
          <AInputNumber v-model="basic.expiresInDays" :min="0" :disabled="!can('instance.settings.basic')" />
        </AFormItem>
        <AFormItem label="标签（逗号分隔）"><AInput v-model="basic.tags" :disabled="!can('instance.settings.basic')" /></AFormItem>
        <AButton type="primary" :disabled="!can('instance.settings.basic')" :loading="saveMutation.isPending.value" @click="saveMutation.mutate({ group: 'basic' })">
          保存
        </AButton>
      </AForm>
    </ATabPane>

    <!-- 启动设置 -->
    <ATabPane key="launch" title="启动">
      <AForm :model="launch" layout="vertical" class="settings-form">
        <AFormItem label="启动命令"><AInput v-model="launch.launchCommand" class="mono" :disabled="!can('instance.settings.launch')" /></AFormItem>
        <AFormItem label="更新命令"><AInput v-model="launch.updateCommand" class="mono" :disabled="!can('instance.settings.launch')" /></AFormItem>
        <AFormItem label="进程包装">
          <ARadioGroup v-model="launch.supervisorMode" :disabled="!can('instance.settings.launch')">
            <ARadio value="DIRECT">直接启动</ARadio>
            <ARadio value="MCDR">MCDR 包装</ARadio>
          </ARadioGroup>
        </AFormItem>
        <AButton type="primary" :disabled="!can('instance.settings.launch')" @click="saveMutation.mutate({ group: 'launch' })">保存</AButton>
      </AForm>
    </ATabPane>

    <!-- 路径设置 -->
    <ATabPane key="path" title="路径">
      <AForm :model="path" layout="vertical" class="settings-form">
        <AFormItem label="工作目录" extra="仅允许节点实例根目录之下的路径">
          <AInput v-model="path.workDir" class="mono" :disabled="!can('instance.settings.path')" />
        </AFormItem>
        <AButton type="primary" :disabled="!can('instance.settings.path')" @click="saveMutation.mutate({ group: 'path' })">保存</AButton>
      </AForm>
    </ATabPane>

    <!-- 容器设置 -->
    <ATabPane key="container" title="容器">
      <AForm :model="container" layout="vertical" class="settings-form">
        <AFormItem label="运行模式" extra="HOST 直接运行于宿主机；CONTAINER 经 Docker Engine 启动（M4）">
          <ARadioGroup v-model="container.runtimeMode" :disabled="!can('instance.settings.container')">
            <ARadio value="HOST">HOST</ARadio>
            <ARadio value="CONTAINER">CONTAINER</ARadio>
          </ARadioGroup>
        </AFormItem>
        <template v-if="container.runtimeMode === 'CONTAINER'">
          <AFormItem label="镜像"><AInput v-model="container.image" class="mono" placeholder="itzg/minecraft-server:latest" :disabled="!can('instance.settings.container')" /></AFormItem>
          <AFormItem label="端口映射"><ATextarea v-model="container.ports" class="mono" placeholder="25565:25565（每行一条）" :disabled="!can('instance.settings.container')" /></AFormItem>
          <AFormItem label="环境变量"><ATextarea v-model="container.env" class="mono" placeholder="KEY=VALUE（每行一条）" :disabled="!can('instance.settings.container')" /></AFormItem>
        </template>
        <AButton type="primary" :disabled="!can('instance.settings.container')" @click="saveMutation.mutate({ group: 'container' })">保存</AButton>
      </AForm>
    </ATabPane>

    <!-- CPU 调度 -->
    <ATabPane key="cpu" title="CPU 调度">
      <AForm :model="cpu" layout="vertical" class="settings-form">
        <AFormItem v-if="cpuPolicy" label="当前策略状态">
          <ATag :color="cpuPolicy.status === 'applied' ? 'green' : cpuPolicy.status === 'degraded' ? 'orange' : 'gray'">
            {{ CPU_STATUS_LABEL[cpuPolicy.status] ?? cpuPolicy.status }}
          </ATag>
          <span class="text-secondary" style="margin-left: 8px">策略不会把"偏好大核"伪装成硬保证</span>
        </AFormItem>
        <AFormItem label="调度模式">
          <ARadioGroup v-model="cpu.mode" :disabled="!can('instance.settings.cpu')">
            <ARadio value="AUTO_PERFORMANCE">自动优先性能核</ARadio>
            <ARadio value="MANUAL">手动 CPU 集合</ARadio>
            <ARadio value="SHARED">共享调度</ARadio>
          </ARadioGroup>
        </AFormItem>
        <AFormItem v-if="cpu.mode === 'MANUAL'" label="CPU 集合">
          <ASelect v-model="cpu.cpuSet" multiple :disabled="!can('instance.settings.cpu')">
            <AOption v-for="n in topology?.logicalCpus ?? 0" :key="n - 1" :value="n - 1">
              CPU {{ n - 1 }}{{ topology?.performanceCores.includes(n - 1) ? '（性能核）' : '（能效核）' }}
            </AOption>
          </ASelect>
        </AFormItem>
        <AFormItem label="独占预留"><ASwitch v-model="cpu.exclusive" :disabled="!can('instance.settings.cpu')" /></AFormItem>
        <AButton type="primary" :disabled="!can('instance.settings.cpu')" :loading="cpuMutation.isPending.value" @click="cpuMutation.mutate()">
          保存
        </AButton>
      </AForm>
    </ATabPane>
  </ATabs>
</template>

<style scoped>
.settings-form {
  max-width: 520px;
  padding-top: var(--mcnp-space-3);
}
</style>
