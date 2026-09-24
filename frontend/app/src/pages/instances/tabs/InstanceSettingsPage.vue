<script setup lang="ts">
/** 实例设置：基本/启动/路径/容器/CPU/MCDR 分组，各组对应独立权限点（PLAN 4.4）。 */

import { computed, reactive, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError, type CpuPolicyMode, type MountBinding, type Permission } from '@mcnp/api-client';
import { PairTable, type PairColumn, type PairRow } from '@mcnp/ui';
import { useApi } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const route = useRoute();
const router = useRouter();
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

// 容器镜像下拉：来自镜像管理页维护的节点镜像列表
const { data: images } = useQuery({
  queryKey: computed(() => ['images', instance.value?.coreId ?? '']),
  queryFn: () => api.images.list(instance.value?.coreId ?? ''),
  enabled: computed(() => instance.value !== undefined && auth.has('image.read')),
});

const basic = reactive({ name: '', expiresAt: null as number | null, tags: '' });
const launch = reactive({ launchCommand: '', updateCommand: '' });
const mcdr = reactive({ checkUpdate: true, autoReload: false, language: 'zh_cn' });
const path = reactive({ workDir: '' });
const container = reactive({
  runtimeMode: 'HOST' as 'HOST' | 'CONTAINER',
  image: '',
  ports: [] as PairRow[],
  env: [] as PairRow[],
  mounts: [] as PairRow[],
});
const cpu = reactive({ mode: 'SHARED' as CpuPolicyMode, cpuSet: [] as number[], exclusive: false, numaNodeId: null as number | null });

/** "25565:25565" ↔ { host, container } */
function portsToRows(ports: string[]): PairRow[] {
  return ports.map((p) => {
    const idx = p.indexOf(':');
    return idx < 0 ? { host: p, container: p } : { host: p.slice(0, idx), container: p.slice(idx + 1) };
  });
}

function rowsToPorts(rows: PairRow[]): string[] {
  return rows.map((r) => `${r.host}:${r.container}`);
}

function envToRows(env: Record<string, string>): PairRow[] {
  return Object.entries(env).map(([key, value]) => ({ key, value }));
}

function rowsToEnv(rows: PairRow[]): Record<string, string> {
  return Object.fromEntries(rows.map((r) => [r.key ?? '', r.value ?? '']));
}

function mountsToRows(mounts: MountBinding[]): PairRow[] {
  return mounts.map((m) => ({ hostPath: m.hostPath, containerPath: m.containerPath }));
}

function rowsToMounts(rows: PairRow[]): MountBinding[] {
  return rows.map((r) => ({ hostPath: r.hostPath ?? '', containerPath: r.containerPath ?? '' }));
}

watch(
  instance,
  (inst) => {
    if (!inst) return;
    basic.name = inst.name;
    basic.tags = inst.tags.join(', ');
    basic.expiresAt = inst.expiresAt;
    launch.launchCommand = inst.launchCommand;
    launch.updateCommand = inst.updateCommand ?? '';
    mcdr.checkUpdate = inst.mcdrSettings.checkUpdate;
    mcdr.autoReload = inst.mcdrSettings.autoReload;
    mcdr.language = inst.mcdrSettings.language;
    path.workDir = inst.workDir;
    container.runtimeMode = inst.runtimeMode;
    container.image = inst.containerImage ?? '';
    container.ports = portsToRows(inst.containerPorts);
    container.env = envToRows(inst.containerEnv);
    container.mounts = mountsToRows(inst.containerMounts);
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

const imageOptions = computed(() => (images.value ?? []).flatMap((img) => img.repoTags));

// ADatePicker 不接受 null，用代理在 null ↔ undefined 间转换
const expiresAtModel = computed({
  get: () => basic.expiresAt ?? undefined,
  set: (v) => {
    basic.expiresAt = typeof v === 'number' ? v : null;
  },
});

const saveMutation = useMutation({
  mutationFn: ({ group }: { group: string }) => {
    if (group === 'basic') {
      return api.instances.updateSettings(instanceId.value, {
        name: basic.name,
        tags: basic.tags.split(',').map((t) => t.trim()).filter(Boolean),
        expiresAt: basic.expiresAt,
      });
    }
    if (group === 'launch') {
      return api.instances.updateSettings(instanceId.value, {
        launchCommand: launch.launchCommand,
        updateCommand: launch.updateCommand || null,
      });
    }
    if (group === 'mcdr') {
      return api.instances.updateSettings(instanceId.value, { mcdrSettings: { ...mcdr } });
    }
    if (group === 'path') {
      return api.instances.updateSettings(instanceId.value, { workDir: path.workDir });
    }
    // container
    if (container.runtimeMode === 'CONTAINER' && !container.image) {
      throw new ApiError(400, 'IMAGE_REQUIRED', '容器模式必须选择镜像；如无镜像请先在「镜像管理」拉取');
    }
    const ports = rowsToPorts(container.ports);
    if (new Set(ports).size !== ports.length) throw new ApiError(400, 'PORT_DUPLICATED', '端口映射存在重复条目');
    const envKeys = container.env.map((r) => r.key);
    if (new Set(envKeys).size !== envKeys.length) throw new ApiError(400, 'ENV_DUPLICATED', '环境变量存在重复变量名');
    // 合并备份自动挂载：备份页独占管理 /backups 项，避免本页持旧副本保存时将其覆盖
    const backupMounts = (instance.value?.containerMounts ?? []).filter(
      (m) => m.containerPath === '/backups' && !container.mounts.some((r) => r.containerPath === '/backups'),
    );
    return api.instances.updateSettings(instanceId.value, {
      runtimeMode: container.runtimeMode,
      containerImage: container.image || null,
      containerPorts: ports,
      containerEnv: rowsToEnv(container.env),
      containerMounts: [...rowsToMounts(container.mounts), ...backupMounts],
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

const PORT_COLUMNS: [PairColumn, PairColumn] = [
  { key: 'host', title: '宿主机端口', placeholder: '25565' },
  { key: 'container', title: '容器端口', placeholder: '25565' },
];

const ENV_COLUMNS: [PairColumn, PairColumn] = [
  { key: 'key', title: '变量名', placeholder: 'EULA' },
  { key: 'value', title: '值', placeholder: 'TRUE' },
];

const MOUNT_COLUMNS: [PairColumn, PairColumn] = [
  { key: 'hostPath', title: '宿主机路径', placeholder: '/opt/mc/data' },
  { key: 'containerPath', title: '容器内路径', placeholder: '/data' },
];

function validatePort(row: PairRow): string | null {
  const host = Number(row.host);
  const containerPort = Number(row.container);
  if (!Number.isInteger(host) || host < 1 || host > 65535) return '宿主机端口必须是 1-65535 的整数';
  if (!Number.isInteger(containerPort) || containerPort < 1 || containerPort > 65535) return '容器端口必须是 1-65535 的整数';
  return null;
}

function validateEnv(row: PairRow): string | null {
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(row.key ?? '')) return '变量名只能包含字母、数字与下划线，且不能以数字开头';
  return null;
}

function validateMount(row: PairRow): string | null {
  if (!row.hostPath?.startsWith('/') && !/^[A-Za-z]:[\\/]/.test(row.hostPath ?? '')) return '宿主机路径必须是绝对路径';
  if (!row.containerPath?.startsWith('/')) return '容器内路径必须以 / 开头';
  return null;
}
</script>

<template>
  <ATabs type="line">
    <!-- 基本设置 -->
    <ATabPane key="basic" title="基本">
      <AForm :model="basic" layout="vertical" class="settings-form">
        <AFormItem label="实例名称"><AInput v-model="basic.name" :disabled="!can('instance.settings.basic')" /></AFormItem>
        <AFormItem label="到期时间" extra="到达该时间点后按到期策略处理；留空表示不限期">
          <ADatePicker
            v-model="expiresAtModel"
            show-time
            value-format="timestamp"
            allow-clear
            style="width: 280px"
            :disabled="!can('instance.settings.basic')"
          />
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
        <AButton type="primary" :disabled="!can('instance.settings.launch')" :loading="saveMutation.isPending.value" @click="saveMutation.mutate({ group: 'launch' })">保存</AButton>
      </AForm>
    </ATabPane>

    <!-- MCDR 设置（仅 MCDR 实例可见，独立于启动命令） -->
    <ATabPane v-if="instance?.supervisorMode === 'MCDR'" key="mcdr" title="MCDR">
      <AForm :model="mcdr" layout="vertical" class="settings-form">
        <AFormItem label="启动时检查更新"><ASwitch v-model="mcdr.checkUpdate" :disabled="!can('instance.settings.launch')" /></AFormItem>
        <AFormItem label="配置变更自动重载"><ASwitch v-model="mcdr.autoReload" :disabled="!can('instance.settings.launch')" /></AFormItem>
        <AFormItem label="语言包">
          <ASelect v-model="mcdr.language" :disabled="!can('instance.settings.launch')" style="width: 220px">
            <AOption value="zh_cn">简体中文（zh_cn）</AOption>
            <AOption value="en_us">English（en_us）</AOption>
          </ASelect>
        </AFormItem>
        <AButton type="primary" :disabled="!can('instance.settings.launch')" :loading="saveMutation.isPending.value" @click="saveMutation.mutate({ group: 'mcdr' })">保存</AButton>
      </AForm>
    </ATabPane>

    <!-- 路径设置 -->
    <ATabPane key="path" title="路径">
      <AForm :model="path" layout="vertical" class="settings-form">
        <AFormItem label="工作目录" extra="仅允许节点实例根目录之下的路径">
          <AInput v-model="path.workDir" class="mono" :disabled="!can('instance.settings.path')" />
        </AFormItem>
        <AButton type="primary" :disabled="!can('instance.settings.path')" :loading="saveMutation.isPending.value" @click="saveMutation.mutate({ group: 'path' })">保存</AButton>
      </AForm>
    </ATabPane>

    <!-- 容器设置 -->
    <ATabPane key="container" title="容器">
      <AForm :model="container" layout="vertical" class="settings-form settings-form--wide">
        <AFormItem label="运行模式" extra="HOST：直接运行在节点系统上；CONTAINER：在容器中隔离运行">
          <ARadioGroup v-model="container.runtimeMode" :disabled="!can('instance.settings.container')">
            <ARadio value="HOST">HOST</ARadio>
            <ARadio value="CONTAINER">CONTAINER</ARadio>
          </ARadioGroup>
        </AFormItem>
        <template v-if="container.runtimeMode === 'CONTAINER'">
          <AFormItem label="镜像">
            <ASelect v-model="container.image" class="mono" placeholder="选择已有镜像" :disabled="!can('instance.settings.container')">
              <AOption v-for="tag in imageOptions" :key="tag" :value="tag">{{ tag }}</AOption>
            </ASelect>
            <template #extra>
              <span v-if="!auth.has('image.read')">缺少 image.read 权限，无法加载镜像列表。</span>
              <template v-else>
                列表为空？
                <a style="cursor: pointer" @click="router.push('/images')">前往「镜像管理」拉取或构建镜像</a>
              </template>
            </template>
          </AFormItem>
          <AFormItem label="端口映射">
            <PairTable
              :columns="PORT_COLUMNS"
              :rows="container.ports"
              :disabled="!can('instance.settings.container')"
              :validate="validatePort"
              @change="(rows) => (container.ports = rows)"
            />
          </AFormItem>
          <AFormItem label="环境变量">
            <PairTable
              :columns="ENV_COLUMNS"
              :rows="container.env"
              :disabled="!can('instance.settings.container')"
              :validate="validateEnv"
              @change="(rows) => (container.env = rows)"
            />
          </AFormItem>
          <AFormItem label="路径映射" extra="把宿主机路径映射为容器内路径；启用备份时会自动追加备份目录映射">
            <PairTable
              :columns="MOUNT_COLUMNS"
              :rows="container.mounts"
              :disabled="!can('instance.settings.container')"
              :validate="validateMount"
              @change="(rows) => (container.mounts = rows)"
            />
          </AFormItem>
        </template>
        <AButton type="primary" :disabled="!can('instance.settings.container')" :loading="saveMutation.isPending.value" @click="saveMutation.mutate({ group: 'container' })">
          保存
        </AButton>
      </AForm>
    </ATabPane>

    <!-- CPU 调度 -->
    <ATabPane key="cpu" title="CPU 调度">
      <AForm :model="cpu" layout="vertical" class="settings-form">
        <AFormItem v-if="cpuPolicy" label="当前策略状态">
          <ATag :color="cpuPolicy.status === 'applied' ? 'green' : cpuPolicy.status === 'degraded' ? 'orange' : 'gray'">
            {{ CPU_STATUS_LABEL[cpuPolicy.status] ?? cpuPolicy.status }}
          </ATag>
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
  max-width: 560px;
  padding-top: var(--mcnp-space-3);
}

.settings-form--wide {
  max-width: 720px;
}
</style>
