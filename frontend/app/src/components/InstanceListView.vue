<script setup lang="ts">
import {
  Button as AButton,
  Empty as AEmpty,
  Input as AInput,
  InputNumber as AInputNumber,
  Modal as AModal,
  Option as AOption,
  Pagination as APagination,
  Popconfirm as APopconfirm,
  Select as ASelect,
  Spin as ASpin,
  Textarea as ATextarea,
  Tooltip as ATooltip,
} from '@arco-design/web-vue';
import {
  IconApps,
  IconCheck,
  IconCloud,
  IconCode,
  IconLeft,
  IconPlayArrow,
  IconPlus,
  IconRefresh,
  IconRight,
  IconSearch,
  IconSettings,
  IconStop,
} from '@arco-design/web-vue/es/icon';
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRoute, useRouter } from 'vue-router';

import type {
  Core,
  InstallTemplateVersion,
  Instance,
  InstanceCreate,
  InstanceKind,
  InstanceState,
  ManagedRuntime,
  PanelApiClient,
  ProvisionOperation,
  TemplateProvisionRequest,
} from '@mcnp/api-client';

import { canStartInstance, canStopInstance, describeError, statusClass } from '../utils/presentation';

type CreateStep = 1 | 2 | 3;
type LaunchProfile = 'java' | 'bedrock-native' | 'pocketmine' | 'custom';
const systemRuntimeValue = '__system__';

interface InstanceKindOption {
  label: string;
  value: InstanceKind;
}

interface InstanceKindGroup {
  key: string;
  kinds: InstanceKindOption[];
}

const props = defineProps<{
  cores: Core[];
  instances: Instance[];
  client: PanelApiClient;
  loading: boolean;
  actionPending: string | null;
}>();

const emit = defineEmits<{
  action: [action: 'start' | 'stop' | 'kill' | 'reset', coreId: string, instanceId: string];
  created: [instance: Instance];
}>();

const { t, te } = useI18n();
const route = useRoute();
const router = useRouter();
const query = ref('');
const stateFilter = ref('');
const currentPage = ref(1);
const createVisible = ref(false);
const createPending = ref(false);
const createError = ref('');
const createStep = ref<CreateStep>(1);
const createCoreId = ref('');
const createId = ref('');
const createName = ref('');
const createKind = ref<InstanceKind | null>(null);
const createVersions = ref<InstallTemplateVersion[]>([]);
const createRuntimes = ref<ManagedRuntime[]>([]);
const createOptionsPending = ref(false);
const createMinecraftVersion = ref('');
const createLoaderVersion = ref('');
const createRuntimeId = ref(systemRuntimeValue);
const createIdAutomatic = ref(true);
const createProvisionState = ref<'resolving' | 'installing' | ''>('');
const createDirectory = ref('');
const createExecutable = ref('java');
const createJvmArguments = ref('');
const createArtifactPath = ref('server.jar');
const createArguments = ref('nogui');
const createStopCommand = ref('stop');
const createStopTimeoutSeconds = ref(30);
const pageSize = 12;
const coreFilter = ref(routeCoreId());
const stateOptions: InstanceState[] = [
  'RUNNING',
  'STARTING',
  'STOPPING',
  'STOPPED',
  'FAILED',
  'UNKNOWN',
  'CREATED',
];
const instanceKindGroups: InstanceKindGroup[] = [
  {
    key: 'javaVanilla',
    kinds: [{ label: 'Vanilla', value: 'VANILLA' }],
  },
  {
    key: 'javaModded',
    kinds: [
      { label: 'NeoForge', value: 'NEO_FORGE' },
      { label: 'Forge', value: 'FORGE' },
      { label: 'Fabric', value: 'FABRIC' },
    ],
  },
  {
    key: 'javaPlugin',
    kinds: [
      { label: 'Bukkit', value: 'BUKKIT' },
      { label: 'Spigot', value: 'SPIGOT' },
      { label: 'Paper', value: 'PAPER' },
      { label: 'Purpur', value: 'PURPUR' },
      { label: 'Pufferfish', value: 'PUFFERFISH' },
      { label: 'Folia', value: 'FOLIA' },
      { label: 'Leaf', value: 'LEAF' },
    ],
  },
  {
    key: 'javaHybrid',
    kinds: [
      { label: 'Mohist', value: 'MOHIST' },
      { label: 'Magma', value: 'MAGMA' },
      { label: 'Sponge', value: 'SPONGE' },
      { label: 'Arclight', value: 'ARCLIGHT' },
      { label: 'Youer', value: 'YOUER' },
      { label: 'Silkard', value: 'SILKARD' },
      { label: 'CatServer', value: 'CAT_SERVER' },
    ],
  },
  {
    key: 'proxy',
    kinds: [
      { label: 'Velocity', value: 'VELOCITY' },
      { label: 'Waterfall', value: 'WATERFALL' },
      { label: 'BungeeCord', value: 'BUNGEE_CORD' },
      { label: 'Lightfall', value: 'LIGHTFALL' },
      { label: 'Geyser', value: 'GEYSER' },
    ],
  },
  {
    key: 'bedrock',
    kinds: [
      { label: 'Bedrock Dedicated Server', value: 'BEDROCK_DEDICATED_SERVER' },
      { label: 'PocketMine-MP', value: 'POCKET_MINE_MP' },
      { label: 'Nukkit', value: 'NUKKIT' },
      { label: 'Cloudburst Nukkit', value: 'CLOUDBURST_NUKKIT' },
    ],
  },
  {
    key: 'custom',
    kinds: [{ label: 'Custom', value: 'CUSTOM' }],
  },
];
const instanceKindOptions = instanceKindGroups.flatMap((group) => group.kinds);

const filteredInstances = computed(() => {
  const normalizedQuery = query.value.trim().toLocaleLowerCase();
  return props.instances.filter((instance) => {
    if (coreFilter.value && instance.coreId !== coreFilter.value) {
      return false;
    }
    if (stateFilter.value && instance.runtime.state !== stateFilter.value) {
      return false;
    }
    if (!normalizedQuery) {
      return true;
    }
    return [instance.name, instance.id, instance.kind, coreName(instance.coreId)]
      .join(' ')
      .toLocaleLowerCase()
      .includes(normalizedQuery);
  });
});
const pagedInstances = computed(() => {
  const offset = (currentPage.value - 1) * pageSize;
  return filteredInstances.value.slice(offset, offset + pageSize);
});
const selectedCreateCore = computed(
  () => props.cores.find((core) => core.id === createCoreId.value) ?? null,
);
const createLaunchProfile = computed<LaunchProfile>(() =>
  createKind.value ? launchProfile(createKind.value) : 'custom',
);
const createTemplateId = computed(() =>
  createKind.value ? templateIdForKind(createKind.value) : null,
);
const automaticProvision = computed(() => createTemplateId.value === 'neoforge');
const createGameVersions = computed(() =>
  createVersions.value
    .filter((version) => version.kind === 'GAME')
    .sort((left, right) => compareVersions(right.id, left.id)),
);
const createLoaderVersions = computed(() =>
  createVersions.value
    .filter(
      (version) =>
        version.kind === 'LOADER' &&
        (!version.gameVersion || version.gameVersion === createMinecraftVersion.value),
    )
    .sort((left, right) => compareVersions(right.id, left.id)),
);
const createManagedJavaRuntimes = computed(() =>
  createRuntimes.value.filter(
    (runtime) => runtime.kind === 'JAVA' && runtime.validation === 'VALID' && runtime.runtimeId,
  ),
);
const createLaunchPreview = computed(() => {
  if (!automaticProvision.value || !createLoaderVersion.value) {
    return '';
  }
  return `java @user_jvm_args.txt @libraries/net/neoforged/neoforge/${createLoaderVersion.value}/{os}_args.txt nogui`;
});
const canCreateInstance = computed(
  () =>
    /^[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}$/.test(createId.value) &&
    createName.value.trim().length > 0 &&
    createDirectory.value.trim().length > 0 &&
    (automaticProvision.value
      ? createMinecraftVersion.value.length > 0 && createLoaderVersion.value.length > 0
      : createExecutable.value.trim().length > 0 &&
        (createLaunchProfile.value === 'bedrock-native' ||
          createLaunchProfile.value === 'custom' ||
          createArtifactPath.value.trim().length > 0)) &&
    createStopCommand.value.trim().length > 0 &&
    createStopTimeoutSeconds.value >= 1 &&
    createStopTimeoutSeconds.value <= 300 &&
    selectedCreateCore.value !== null &&
    createKind.value !== null &&
    !createOptionsPending.value,
);

watch(
  () => route.params.coreId,
  () => {
    coreFilter.value = routeCoreId();
  },
);

watch([query, stateFilter, coreFilter], () => {
  currentPage.value = 1;
});

watch(createId, (value, previousValue) => {
  if (!createDirectory.value || createDirectory.value === `instances/${previousValue}`) {
    createDirectory.value = value ? `instances/${value}` : '';
  }
});

function openCreate(): void {
  const selectedRouteCoreId = props.cores.some((core) => core.id === routeCoreId())
    ? routeCoreId()
    : '';
  createCoreId.value = selectedRouteCoreId;
  createStep.value = selectedRouteCoreId ? 2 : 1;
  createId.value = '';
  createName.value = '';
  createKind.value = null;
  createVersions.value = [];
  createRuntimes.value = [];
  createOptionsPending.value = false;
  createMinecraftVersion.value = '';
  createLoaderVersion.value = '';
  createRuntimeId.value = systemRuntimeValue;
  createIdAutomatic.value = true;
  createProvisionState.value = '';
  createDirectory.value = '';
  createExecutable.value = '';
  createJvmArguments.value = '';
  createArtifactPath.value = '';
  createArguments.value = '';
  createStopCommand.value = 'stop';
  createStopTimeoutSeconds.value = 30;
  createError.value = '';
  createVisible.value = true;
}

async function createInstance(): Promise<void> {
  const kind = createKind.value;
  if (!canCreateInstance.value || !kind) {
    return;
  }
  createPending.value = true;
  createError.value = '';
  try {
    const instance = automaticProvision.value
      ? await provisionInstance()
      : await createConfiguredInstance(kind);
    createVisible.value = false;
    emit('created', instance);
  } catch (error) {
    createError.value = describeError(error, t('error.instanceCreate'));
  } finally {
    createProvisionState.value = '';
    createPending.value = false;
  }
}

async function continueCreate(): Promise<void> {
  if (createStep.value === 1 && createCoreId.value) {
    createStep.value = 2;
  } else if (createStep.value === 2 && createKind.value) {
    createStep.value = 3;
    await loadCreateOptions();
  }
}

function previousCreateStep(): void {
  if (createStep.value > 1) {
    createStep.value = (createStep.value - 1) as CreateStep;
  }
}

function goToCreateStep(step: CreateStep): void {
  if (step <= createStep.value || (step === 2 && createCoreId.value)) {
    createStep.value = step;
  }
}

function selectCreateCore(coreId: string): void {
  createCoreId.value = coreId;
  createError.value = '';
}

function selectCreateKind(kind: InstanceKind): void {
  if (createKind.value !== kind) {
    createKind.value = kind;
    createVersions.value = [];
    createMinecraftVersion.value = '';
    createLoaderVersion.value = '';
    createIdAutomatic.value = true;
    applyKindDefaults(kind);
  }
  createError.value = '';
}

function applyKindDefaults(kind: InstanceKind): void {
  const profile = launchProfile(kind);
  createJvmArguments.value = '';
  createStopCommand.value = 'stop';
  createStopTimeoutSeconds.value = 30;

  if (profile === 'java') {
    createExecutable.value = 'java';
    createJvmArguments.value = '-Xms1G\n-Xmx2G';
    createArtifactPath.value = javaArtifactName(kind);
    createArguments.value = defaultJavaArguments(kind);
  } else if (profile === 'bedrock-native') {
    createExecutable.value = 'bedrock_server';
    createArtifactPath.value = '';
    createArguments.value = '';
  } else if (profile === 'pocketmine') {
    createExecutable.value = 'php';
    createArtifactPath.value = 'PocketMine-MP.phar';
    createArguments.value = '--no-wizard';
  } else {
    createExecutable.value = '';
    createArtifactPath.value = '';
    createArguments.value = '';
  }
}

async function loadCreateOptions(): Promise<void> {
  if (!automaticProvision.value || !createTemplateId.value || !createCoreId.value) {
    return;
  }
  createOptionsPending.value = true;
  createError.value = '';
  try {
    const [versions, runtimes] = await Promise.all([
      props.client.listInstallTemplateVersions(createTemplateId.value),
      props.client.listManagedRuntimes(createCoreId.value),
    ]);
    createVersions.value = versions.items;
    createRuntimes.value = runtimes.items;
    createMinecraftVersion.value =
      createGameVersions.value.find((version) => version.stable)?.id ??
      createGameVersions.value[0]?.id ??
      '';
    selectDefaultLoaderVersion();
    updateSuggestedInstanceIdentity();
  } catch (error) {
    createError.value = describeError(error, t('error.instanceVersions'));
  } finally {
    createOptionsPending.value = false;
  }
}

function selectGameVersion(): void {
  selectDefaultLoaderVersion();
  updateSuggestedInstanceIdentity();
}

function selectLoaderVersion(): void {
  updateSuggestedInstanceIdentity();
}

function selectDefaultLoaderVersion(): void {
  createLoaderVersion.value =
    createLoaderVersions.value.find((version) => version.stable)?.id ??
    createLoaderVersions.value[0]?.id ??
    '';
}

function markCreateIdEdited(): void {
  createIdAutomatic.value = false;
}

function updateSuggestedInstanceIdentity(): void {
  if (!automaticProvision.value || !createMinecraftVersion.value || !createTemplateId.value) {
    return;
  }
  const prefix = `${createTemplateId.value}-${createMinecraftVersion.value}-server-`;
  const existingIds = new Set(props.instances.map((instance) => instance.id));
  let sequence = 1;
  while (existingIds.has(`${prefix}${String(sequence).padStart(2, '0')}`)) {
    sequence += 1;
  }
  const suffix = String(sequence).padStart(2, '0');
  if (createIdAutomatic.value) {
    createId.value = `${prefix}${suffix}`;
  }
  if (!createName.value.trim()) {
    createName.value = `${kindLabel(createKind.value ?? 'NEO_FORGE')} ${createMinecraftVersion.value} Server ${suffix}`;
  }
}

async function provisionInstance(): Promise<Instance> {
  const templateId = createTemplateId.value;
  if (!templateId) {
    throw new Error(t('error.instanceCreate'));
  }
  const request: TemplateProvisionRequest = {
    instanceId: createId.value,
    instanceName: createName.value.trim(),
    instanceDirectory: createDirectory.value.trim(),
    minecraftVersion: createMinecraftVersion.value,
    loaderVersion: createLoaderVersion.value,
    runtimeId: createRuntimeId.value === systemRuntimeValue ? null : createRuntimeId.value,
    jvmArguments: argumentLines(createJvmArguments.value),
    stopCommand: createStopCommand.value.trim(),
    stopTimeoutSeconds: createStopTimeoutSeconds.value,
  };
  createProvisionState.value = 'resolving';
  const resolution = await props.client.resolveTemplateProvisionPlan(
    createCoreId.value,
    templateId,
    request,
  );
  createProvisionState.value = 'installing';
  const accepted = await props.client.executeProvision(
    createCoreId.value,
    resolution.resolvedPlan,
    resolution.planHash,
  );
  const completed = await waitForProvisionTask(createCoreId.value, accepted);
  if (completed.state === 'FAILED') {
    throw new Error(completed.error || t('error.instanceProvision'));
  }
  if (!completed.instance) {
    throw new Error(t('error.instanceProvision'));
  }
  return completed.instance;
}

async function createConfiguredInstance(kind: InstanceKind): Promise<Instance> {
  const request: InstanceCreate = {
    id: createId.value,
    name: createName.value.trim(),
    kind,
    directory: createDirectory.value.trim(),
    launch: {
      executable: createExecutable.value.trim(),
      args: createLaunchArguments(),
      environment: {},
      stopCommand: createStopCommand.value.trim(),
      stopTimeoutSeconds: createStopTimeoutSeconds.value,
      runtimeMode: 'HOST',
      supervisorMode: 'DIRECT',
      mcdr: null,
    },
  };
  return props.client.createInstance(createCoreId.value, request);
}

async function waitForProvisionTask(
  coreId: string,
  accepted: ProvisionOperation,
): Promise<ProvisionOperation> {
  const deadline = Date.now() + 15 * 60 * 1000;
  while (Date.now() < deadline) {
    const task = await props.client.getProvisionTask(coreId, accepted.taskId);
    if (task.state === 'SUCCEEDED' || task.state === 'FAILED') {
      return task;
    }
    await delay(750);
  }
  throw new Error(t('error.instanceProvisionTimeout'));
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}

function launchProfile(kind: InstanceKind): LaunchProfile {
  if (kind === 'BEDROCK_DEDICATED_SERVER') {
    return 'bedrock-native';
  }
  if (kind === 'POCKET_MINE_MP') {
    return 'pocketmine';
  }
  if (kind === 'CUSTOM') {
    return 'custom';
  }
  return 'java';
}

function javaArtifactName(kind: InstanceKind): string {
  if (kind === 'GEYSER') {
    return 'Geyser-Standalone.jar';
  }
  return 'server.jar';
}

function defaultJavaArguments(kind: InstanceKind): string {
  if (
    kind === 'VELOCITY' ||
    kind === 'WATERFALL' ||
    kind === 'BUNGEE_CORD' ||
    kind === 'LIGHTFALL' ||
    kind === 'GEYSER' ||
    kind === 'NUKKIT' ||
    kind === 'CLOUDBURST_NUKKIT'
  ) {
    return '';
  }
  return 'nogui';
}

function templateIdForKind(kind: InstanceKind): string | null {
  const templateIds: Partial<Record<InstanceKind, string>> = {
    VANILLA: 'vanilla',
    PAPER: 'paper',
    VELOCITY: 'velocity',
    FABRIC: 'fabric',
    NEO_FORGE: 'neoforge',
    FORGE: 'forge',
    BUKKIT: 'bukkit',
    SPIGOT: 'spigot',
    PURPUR: 'purpur',
    PUFFERFISH: 'pufferfish',
    FOLIA: 'folia',
    LEAF: 'leaf',
    MOHIST: 'mohist',
    MAGMA: 'magma',
    SPONGE: 'sponge',
    ARCLIGHT: 'arclight',
    YOUER: 'youer',
    SILKARD: 'silkard',
    CAT_SERVER: 'catserver',
    WATERFALL: 'waterfall',
    BUNGEE_CORD: 'bungeecord',
    LIGHTFALL: 'lightfall',
    GEYSER: 'geyser',
    BEDROCK_DEDICATED_SERVER: 'bedrock-dedicated-server',
    POCKET_MINE_MP: 'pocketmine-mp',
    NUKKIT: 'nukkit',
    CLOUDBURST_NUKKIT: 'cloudburst-nukkit',
  };
  return templateIds[kind] ?? null;
}

function compareVersions(left: string, right: string): number {
  return left.localeCompare(right, undefined, { numeric: true, sensitivity: 'base' });
}

function createLaunchArguments(): string[] {
  const runtimeArguments = argumentLines(createArguments.value);
  if (createLaunchProfile.value === 'java') {
    return [
      ...argumentLines(createJvmArguments.value),
      '-jar',
      createArtifactPath.value.trim(),
      ...runtimeArguments,
    ];
  }
  if (createLaunchProfile.value === 'pocketmine') {
    return [createArtifactPath.value.trim(), ...runtimeArguments];
  }
  return runtimeArguments;
}

function argumentLines(value: string): string[] {
  return value
    .split(/\r?\n/u)
    .map((argument) => argument.trim())
    .filter(Boolean);
}

function kindLabel(kind: InstanceKind): string {
  return instanceKindOptions.find((option) => option.value === kind)?.label ?? kind;
}

async function changeCoreFilter(value: unknown): Promise<void> {
  const coreId = typeof value === 'string' ? value : '';
  coreFilter.value = coreId;
  if (coreId) {
    await router.push({ name: 'core-instances', params: { coreId } });
  } else {
    await router.push({ name: 'instances' });
  }
}

function routeCoreId(): string {
  return typeof route.params.coreId === 'string' ? route.params.coreId : '';
}

function coreName(coreId: string): string {
  return props.cores.find((core) => core.id === coreId)?.name ?? coreId;
}

function statusLabel(status: string): string {
  const key = `status.${status}`;
  return te(key) ? t(key) : status;
}

function actionKey(action: string, instance: Instance): string {
  return `${action}:${instance.coreId}:${instance.id}`;
}

function canReset(state: InstanceState): boolean {
  return state === 'FAILED' || state === 'UNKNOWN';
}
</script>

<template>
  <main class="console-page">
    <header class="page-heading page-heading--toolbar">
      <div>
        <p class="page-eyebrow"><IconApps /> {{ t('instances.eyebrow') }}</p>
        <h1>{{ t('instances.title') }}</h1>
      </div>
      <div class="instance-page-actions">
        <p>{{ t('instances.summary', { filtered: filteredInstances.length, total: instances.length }) }}</p>
        <a-button type="primary" :disabled="cores.length === 0" @click="openCreate">
          <template #icon><IconPlus /></template>
          {{ t('instances.create') }}
        </a-button>
      </div>
    </header>

    <section class="filter-bar">
      <a-select
        :model-value="coreFilter"
        :placeholder="t('instances.allCores')"
        allow-clear
        @change="changeCoreFilter"
      >
        <a-option v-for="core in cores" :key="core.id" :value="core.id">{{ core.name }}</a-option>
      </a-select>
      <a-select v-model="stateFilter" :placeholder="t('instances.allStates')" allow-clear>
        <a-option v-for="state in stateOptions" :key="state" :value="state">{{ statusLabel(state) }}</a-option>
      </a-select>
      <a-input v-model="query" allow-clear :placeholder="t('instances.filterPlaceholder')">
        <template #prefix><IconSearch /></template>
      </a-input>
    </section>

    <a-spin class="page-spinner" :loading="loading">
      <section v-if="pagedInstances.length" class="instance-card-grid">
        <article v-for="instance in pagedInstances" :key="`${instance.coreId}:${instance.id}`" class="instance-card">
          <header>
            <span :class="['instance-card__mark', statusClass(instance.runtime.state)]"></span>
            <div>
              <h2>{{ instance.name }}</h2>
              <p>{{ coreName(instance.coreId) }} · {{ instance.kind }}</p>
            </div>
            <i :class="statusClass(instance.runtime.state)"><span></span>{{ statusLabel(instance.runtime.state) }}</i>
          </header>

          <dl>
            <div>
              <dt>{{ t('instances.instanceId') }}</dt>
              <dd>{{ instance.id }}</dd>
            </div>
            <div>
              <dt>{{ t('instances.directory') }}</dt>
              <dd>{{ instance.directory }}</dd>
            </div>
            <div>
              <dt>{{ t('console.pid') }}</dt>
              <dd>{{ instance.runtime.pid ?? t('common.none') }}</dd>
            </div>
            <div>
              <dt>{{ t('instances.players') }}</dt>
              <dd>
                {{ instance.runtime.players?.online ?? 0 }}/{{ instance.runtime.players?.max ?? t('common.unknown') }}
              </dd>
            </div>
          </dl>

          <footer>
            <RouterLink
              class="instance-open-link"
              :to="{
                name: 'instance-workspace',
                params: { coreId: instance.coreId, instanceId: instance.id, view: 'overview' },
              }"
            >
              <a-button type="primary" size="small">
                <template #icon><IconCode /></template>
                {{ t('instances.manage') }}
              </a-button>
            </RouterLink>
            <a-tooltip :content="t('console.start')">
              <a-button
                size="small"
                :loading="actionPending === actionKey('start', instance)"
                :disabled="actionPending !== null || !canStartInstance(instance.runtime.state)"
                :aria-label="t('console.start')"
                @click="emit('action', 'start', instance.coreId, instance.id)"
              >
                <template #icon><IconPlayArrow /></template>
              </a-button>
            </a-tooltip>
            <a-tooltip :content="t('console.stop')">
              <a-button
                size="small"
                :loading="actionPending === actionKey('stop', instance)"
                :disabled="actionPending !== null || !canStopInstance(instance.runtime.state)"
                :aria-label="t('console.stop')"
                @click="emit('action', 'stop', instance.coreId, instance.id)"
              >
                <template #icon><IconStop /></template>
              </a-button>
            </a-tooltip>
            <a-popconfirm
              :content="t('instances.killConfirm', { name: instance.name })"
              @ok="emit('action', 'kill', instance.coreId, instance.id)"
            >
              <a-tooltip :content="t('console.kill')">
                <a-button
                  size="small"
                  status="danger"
                  :loading="actionPending === actionKey('kill', instance)"
                  :disabled="actionPending !== null || !canStopInstance(instance.runtime.state)"
                  :aria-label="t('console.kill')"
                >
                  <template #icon><IconStop /></template>
                </a-button>
              </a-tooltip>
            </a-popconfirm>
            <a-popconfirm
              :content="t('instances.resetConfirm', { name: instance.name })"
              @ok="emit('action', 'reset', instance.coreId, instance.id)"
            >
              <a-tooltip :content="t('instances.reset')">
                <a-button
                  size="small"
                  :loading="actionPending === actionKey('reset', instance)"
                  :disabled="actionPending !== null || !canReset(instance.runtime.state)"
                  :aria-label="t('instances.reset')"
                >
                  <template #icon><IconRefresh /></template>
                </a-button>
              </a-tooltip>
            </a-popconfirm>
          </footer>
        </article>
      </section>
      <a-empty v-else :description="instances.length ? t('instances.noMatches') : t('instances.empty')" />
    </a-spin>

    <footer v-if="filteredInstances.length > pageSize" class="page-pagination">
      <a-pagination v-model:current="currentPage" :page-size="pageSize" :total="filteredInstances.length" />
    </footer>

    <a-modal
      v-model:visible="createVisible"
      :title="t('instances.createTitle')"
      :footer="false"
      :closable="!createPending"
      :mask-closable="!createPending"
      :esc-to-close="!createPending"
      width="min(760px, calc(100vw - 2rem))"
      unmount-on-close
    >
      <form class="instance-create-wizard" @submit.prevent="createInstance">
        <nav class="create-progress" :aria-label="t('instances.createProgress')">
          <button
            type="button"
            :class="{ active: createStep === 1, complete: createStep > 1 }"
            :aria-current="createStep === 1 ? 'step' : undefined"
            @click="goToCreateStep(1)"
          >
            <span><IconCheck v-if="createStep > 1" /><IconCloud v-else /></span>
            {{ t('instances.steps.core') }}
          </button>
          <button
            type="button"
            :class="{ active: createStep === 2, complete: createStep > 2 }"
            :disabled="!createCoreId"
            :aria-current="createStep === 2 ? 'step' : undefined"
            @click="goToCreateStep(2)"
          >
            <span><IconCheck v-if="createStep > 2" /><IconApps v-else /></span>
            {{ t('instances.steps.kind') }}
          </button>
          <button
            type="button"
            :class="{ active: createStep === 3 }"
            :disabled="createStep < 3"
            :aria-current="createStep === 3 ? 'step' : undefined"
            @click="goToCreateStep(3)"
          >
            <span><IconSettings /></span>
            {{ t('instances.steps.settings') }}
          </button>
        </nav>

        <section v-if="createStep === 1" class="create-step create-core-step">
          <h3>{{ t('instances.selectCore') }}</h3>
          <div class="create-core-options">
            <button
              v-for="core in cores"
              :key="core.id"
              type="button"
              :class="['create-core-option', { selected: createCoreId === core.id }]"
              :aria-pressed="createCoreId === core.id"
              @click="selectCreateCore(core.id)"
            >
              <span class="create-core-option__icon"><IconCloud /></span>
              <strong><span>{{ core.name }}</span><small>{{ core.address }}</small></strong>
              <i :class="statusClass(core.status)"><span></span>{{ statusLabel(core.status) }}</i>
              <IconCheck v-if="createCoreId === core.id" class="create-option-check" />
            </button>
          </div>
        </section>

        <section v-else-if="createStep === 2" class="create-step create-kind-step">
          <h3>{{ t('instances.selectKind') }}</h3>
          <div class="create-kind-groups">
            <section v-for="group in instanceKindGroups" :key="group.key" class="create-kind-group">
              <h4>{{ t(`instances.kindGroups.${group.key}`) }}</h4>
              <div>
                <button
                  v-for="kind in group.kinds"
                  :key="kind.value"
                  type="button"
                  :class="['create-kind-option', { selected: createKind === kind.value }]"
                  :aria-pressed="createKind === kind.value"
                  @click="selectCreateKind(kind.value)"
                >
                  <span>{{ kind.label }}</span>
                  <IconCheck v-if="createKind === kind.value" />
                </button>
              </div>
            </section>
          </div>
        </section>

        <section v-else class="create-step create-settings-step">
          <div class="create-context">
            <span><IconCloud /> {{ selectedCreateCore?.name }}</span>
            <span><IconApps /> {{ createKind ? kindLabel(createKind) : '' }}</span>
          </div>

          <section v-if="automaticProvision" class="create-form-section create-version-section">
            <h3>{{ t('instances.versionSection') }}</h3>
            <a-spin :loading="createOptionsPending">
              <div class="instance-create-form__grid">
                <label>
                  <span>{{ t('instances.minecraftVersion') }}</span>
                  <a-select
                    v-model="createMinecraftVersion"
                    :placeholder="t('instances.selectMinecraftVersion')"
                    @change="selectGameVersion"
                  >
                    <a-option v-for="version in createGameVersions" :key="version.id" :value="version.id">
                      {{ version.id }}
                    </a-option>
                  </a-select>
                </label>
                <label>
                  <span>{{ t('instances.loaderVersion') }}</span>
                  <a-select
                    v-model="createLoaderVersion"
                    :placeholder="t('instances.selectLoaderVersion')"
                    :disabled="!createMinecraftVersion"
                    @change="selectLoaderVersion"
                  >
                    <a-option v-for="version in createLoaderVersions" :key="version.id" :value="version.id">
                      {{ version.id }}
                    </a-option>
                  </a-select>
                </label>
              </div>
              <label>
                <span>{{ t('instances.javaRuntime') }}</span>
                <a-select v-model="createRuntimeId">
                  <a-option :value="systemRuntimeValue">{{ t('instances.systemJavaAuto') }}</a-option>
                  <a-option
                    v-for="runtime in createManagedJavaRuntimes"
                    :key="runtime.runtimeId ?? runtime.executable"
                    :value="runtime.runtimeId ?? ''"
                  >
                    {{ runtime.distribution ?? runtime.runtimeId }} · Java {{ runtime.version ?? t('common.unknown') }}
                  </a-option>
                </a-select>
              </label>
            </a-spin>
          </section>

          <section class="create-form-section">
            <h3>{{ t('instances.detailsSection') }}</h3>
            <div class="instance-create-form__grid">
              <label>
                <span>{{ t('instances.instanceId') }}</span>
                <a-input v-model="createId" :max-length="64" allow-clear @input="markCreateIdEdited" />
              </label>
              <label>
                <span>{{ t('instances.name') }}</span>
                <a-input v-model="createName" :max-length="128" allow-clear />
              </label>
            </div>
            <label>
              <span>{{ t('instances.directory') }}</span>
              <a-input v-model="createDirectory" :max-length="1024" allow-clear />
            </label>
          </section>

          <section class="create-form-section">
            <h3>{{ t('instances.launchSection') }}</h3>
            <div v-if="!automaticProvision" class="instance-create-form__grid">
              <label>
                <span>
                  {{
                    createLaunchProfile === 'java'
                      ? t('instances.javaExecutable')
                      : createLaunchProfile === 'pocketmine'
                        ? t('instances.phpExecutable')
                        : createLaunchProfile === 'bedrock-native'
                          ? t('instances.nativeExecutable')
                          : t('instances.executable')
                  }}
                </span>
                <a-input v-model="createExecutable" :max-length="4096" allow-clear />
              </label>
              <label v-if="createLaunchProfile === 'java' || createLaunchProfile === 'pocketmine'">
                <span>
                  {{ createLaunchProfile === 'java' ? t('instances.serverJar') : t('instances.pharPath') }}
                </span>
                <a-input v-model="createArtifactPath" :max-length="4096" allow-clear />
              </label>
            </div>
            <label v-if="createLaunchProfile === 'java'">
              <span>{{ t('instances.jvmArguments') }}</span>
              <a-textarea v-model="createJvmArguments" :auto-size="{ minRows: 2, maxRows: 5 }" />
              <small>{{ t('instances.jvmArgumentsHint') }}</small>
            </label>
            <label v-if="!automaticProvision">
              <span>
                {{
                  createLaunchProfile === 'java' || createLaunchProfile === 'pocketmine'
                    ? t('instances.serverArguments')
                    : t('instances.arguments')
                }}
              </span>
              <a-textarea v-model="createArguments" :auto-size="{ minRows: 2, maxRows: 6 }" />
              <small>
                {{
                  createLaunchProfile === 'java' || createLaunchProfile === 'pocketmine'
                    ? t('instances.argumentsHint')
                    : t('instances.rawArgumentsHint')
                }}
              </small>
            </label>
            <div v-else class="launch-command-preview">
              <span>{{ t('instances.launchCommand') }}</span>
              <code>{{ createLaunchPreview }}</code>
            </div>
          </section>

          <section class="create-form-section">
            <h3>{{ t('instances.shutdownSection') }}</h3>
            <div class="instance-create-form__grid">
              <label>
                <span>{{ t('instances.stopCommand') }}</span>
                <a-input v-model="createStopCommand" :max-length="8192" allow-clear />
              </label>
              <label>
                <span>{{ t('instances.stopTimeout') }}</span>
                <a-input-number v-model="createStopTimeoutSeconds" :min="1" :max="300" />
              </label>
            </div>
          </section>
        </section>

        <p v-if="createError" class="form-error" role="alert">{{ createError }}</p>
        <p v-else-if="createProvisionState" class="provision-state" role="status">
          {{ t(`instances.provisionState.${createProvisionState}`) }}
        </p>
        <div class="instance-create-form__actions">
          <div>
            <a-button :disabled="createPending" @click="createVisible = false">
              {{ t('common.cancel') }}
            </a-button>
            <a-button v-if="createStep > 1" :disabled="createPending" @click="previousCreateStep">
              <template #icon><IconLeft /></template>
              {{ t('common.back') }}
            </a-button>
          </div>
          <a-button
            v-if="createStep < 3"
            type="primary"
            :disabled="(createStep === 1 && !createCoreId) || (createStep === 2 && !createKind)"
            @click="continueCreate"
          >
            {{ t('common.next') }}
            <template #icon><IconRight /></template>
          </a-button>
          <a-button
            v-else
            type="primary"
            html-type="submit"
            :loading="createPending"
            :disabled="!canCreateInstance"
          >
            {{ t('instances.create') }}
          </a-button>
        </div>
      </form>
    </a-modal>
  </main>
</template>

<style scoped>
.instance-page-actions,
.instance-create-form__actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.instance-page-actions p {
  margin: 0;
}

.instance-create-wizard {
  display: grid;
  gap: 1rem;
}

.create-progress {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  border-bottom: 1px solid var(--mcnp-border);
}

.create-progress button {
  display: flex;
  min-width: 0;
  min-height: 3rem;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  border: 0;
  border-bottom: 2px solid transparent;
  padding: 0.55rem 0.7rem;
  background: transparent;
  color: var(--mcnp-text-faint);
  cursor: pointer;
  font-size: 0.72rem;
  font-weight: 650;
}

.create-progress button:not(:disabled):hover {
  background: var(--mcnp-surface-hover);
  color: var(--mcnp-text);
}

.create-progress button.active {
  border-bottom-color: var(--mcnp-primary);
  color: var(--mcnp-primary);
}

.create-progress button.complete {
  color: var(--mcnp-success);
}

.create-progress button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.create-progress button > span {
  display: grid;
  width: 1.65rem;
  height: 1.65rem;
  flex: 0 0 auto;
  border: 1px solid currentColor;
  border-radius: 50%;
  place-items: center;
}

.create-step {
  min-height: 22rem;
  max-height: min(32rem, calc(100vh - 17rem));
  overflow-y: auto;
  padding-right: 0.25rem;
}

.create-step > h3,
.create-form-section h3,
.create-kind-group h4 {
  margin: 0;
  color: var(--mcnp-text);
}

.create-step > h3,
.create-form-section h3 {
  font-size: 0.82rem;
}

.create-core-step,
.create-kind-step,
.create-settings-step,
.create-form-section {
  display: grid;
  align-content: start;
  gap: 0.85rem;
}

.create-core-options {
  display: grid;
  border-top: 1px solid var(--mcnp-border);
}

.create-core-option {
  display: grid;
  min-width: 0;
  grid-template-columns: 2.25rem minmax(0, 1fr) auto 1rem;
  align-items: center;
  gap: 0.75rem;
  border: 0;
  border-bottom: 1px solid var(--mcnp-border-subtle);
  padding: 0.8rem;
  background: transparent;
  color: var(--mcnp-text-muted);
  cursor: pointer;
  text-align: left;
}

.create-core-option:hover,
.create-core-option.selected {
  background: var(--mcnp-primary-soft);
}

.create-core-option.selected {
  color: var(--mcnp-primary);
}

.create-core-option__icon {
  display: grid;
  width: 2.25rem;
  height: 2.25rem;
  border-radius: 5px;
  place-items: center;
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-primary);
}

.create-core-option strong {
  display: grid;
  min-width: 0;
  gap: 0.2rem;
  color: var(--mcnp-text);
  font-size: 0.78rem;
}

.create-core-option strong span,
.create-core-option strong small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.create-core-option strong small {
  color: var(--mcnp-text-faint);
  font-size: 0.66rem;
  font-weight: 400;
}

.create-option-check {
  color: var(--mcnp-primary);
}

.create-kind-groups {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  column-gap: 1.25rem;
}

.create-kind-group {
  display: grid;
  align-content: start;
  gap: 0.55rem;
  border-top: 1px solid var(--mcnp-border);
  padding: 0.75rem 0 0.85rem;
}

.create-kind-group h4 {
  color: var(--mcnp-text-faint);
  font-size: 0.66rem;
  font-weight: 700;
}

.create-kind-group > div {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.create-kind-option {
  display: inline-flex;
  min-height: 2rem;
  align-items: center;
  gap: 0.35rem;
  border: 1px solid var(--mcnp-border);
  border-radius: 4px;
  padding: 0.35rem 0.6rem;
  background: var(--mcnp-surface);
  color: var(--mcnp-text-muted);
  cursor: pointer;
  font-size: 0.68rem;
}

.create-kind-option:hover {
  border-color: var(--mcnp-primary-hover);
  color: var(--mcnp-primary);
}

.create-kind-option.selected {
  border-color: var(--mcnp-primary);
  background: var(--mcnp-primary-soft);
  color: var(--mcnp-primary);
  font-weight: 650;
}

.create-context {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  border-bottom: 1px solid var(--mcnp-border);
  padding-bottom: 0.75rem;
}

.create-context span {
  display: inline-flex;
  min-width: 0;
  align-items: center;
  gap: 0.35rem;
  border-radius: 4px;
  padding: 0.35rem 0.55rem;
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-text-muted);
  font-size: 0.68rem;
}

.create-form-section + .create-form-section {
  border-top: 1px solid var(--mcnp-border-subtle);
  padding-top: 0.85rem;
}

.create-version-section :deep(.arco-spin),
.create-version-section :deep(.arco-spin-children) {
  width: 100%;
}

.create-version-section :deep(.arco-spin-children) {
  display: grid;
  gap: 0.85rem;
}

.launch-command-preview {
  display: grid;
  min-width: 0;
  gap: 0.4rem;
  color: var(--mcnp-text-muted);
  font-size: 0.78rem;
  font-weight: 600;
}

.launch-command-preview code {
  overflow-x: auto;
  border: 1px solid var(--mcnp-border);
  border-radius: 4px;
  padding: 0.7rem 0.8rem;
  background: var(--mcnp-surface-raised);
  color: var(--mcnp-text);
  font-size: 0.68rem;
  font-weight: 500;
  white-space: nowrap;
}

.provision-state {
  margin: 0;
  color: var(--mcnp-primary);
  font-size: 0.75rem;
}

.instance-create-form__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1rem;
}

.instance-create-wizard label {
  display: grid;
  min-width: 0;
  gap: 0.4rem;
  color: var(--mcnp-text-muted);
  font-size: 0.78rem;
  font-weight: 600;
}

.instance-create-wizard label > small {
  color: var(--mcnp-text-faint);
  font-size: 0.66rem;
  font-weight: 400;
}

.instance-create-form__actions {
  justify-content: space-between;
  border-top: 1px solid var(--mcnp-border);
  padding-top: 0.25rem;
}

.instance-create-form__actions > div {
  display: flex;
  gap: 0.5rem;
}

.instance-create-wizard :deep(.arco-input-number) {
  width: 100%;
}

.page-spinner {
  display: block;
  min-height: 18rem;
}

.filter-bar {
  display: grid;
  grid-template-columns: minmax(10rem, 14rem) minmax(9rem, 12rem) minmax(15rem, 1fr);
  gap: 0.65rem;
  margin-bottom: 0.85rem;
}

.filter-bar :deep(.arco-select-view),
.filter-bar :deep(.arco-input-wrapper) {
  border-color: var(--mcnp-border);
  background: var(--mcnp-surface);
}

.instance-card-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.8rem;
}

.instance-card {
  display: flex;
  min-width: 0;
  min-height: 14.5rem;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-surface);
}

.instance-card > header {
  display: grid;
  grid-template-columns: 0.25rem minmax(0, 1fr) auto;
  align-items: center;
  gap: 0.7rem;
  padding: 0.9rem;
  border-bottom: 1px solid var(--mcnp-border-subtle);
}

.instance-card__mark {
  width: 0.22rem;
  height: 2.25rem;
  border-radius: 2px;
  background: var(--mcnp-text-faint);
}

.instance-card > header div {
  display: grid;
  min-width: 0;
  gap: 0.2rem;
}

.instance-card h2,
.instance-card p {
  overflow: hidden;
  margin: 0;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-card h2 {
  color: var(--mcnp-text);
  font-size: 0.85rem;
}

.instance-card p {
  color: var(--mcnp-text-faint);
  font-size: 0.65rem;
}

.instance-card dl {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
  margin: 0;
  padding: 0.9rem;
}

.instance-card dl div {
  display: grid;
  min-width: 0;
  gap: 0.22rem;
}

.instance-card dt {
  color: var(--mcnp-text-faint);
  font-size: 0.62rem;
}

.instance-card dd {
  overflow: hidden;
  margin: 0;
  color: var(--mcnp-text-muted);
  font-size: 0.7rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-card footer {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  margin-top: auto;
  padding: 0.7rem 0.9rem;
  border-top: 1px solid var(--mcnp-border-subtle);
  background: var(--mcnp-surface-raised);
}

.instance-open-link {
  margin-right: auto;
  text-decoration: none;
}

.page-pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 1rem;
}

@media (max-width: 70rem) {
  .instance-card-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 46rem) {
  .instance-page-actions {
    width: 100%;
    justify-content: space-between;
  }

  .instance-create-form__grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .create-progress button {
    min-height: 3.5rem;
    flex-direction: column;
    gap: 0.25rem;
    padding-inline: 0.3rem;
    font-size: 0.62rem;
  }

  .create-progress button > span {
    width: 1.4rem;
    height: 1.4rem;
  }

  .create-step {
    max-height: calc(100vh - 18rem);
  }

  .create-kind-groups {
    grid-template-columns: minmax(0, 1fr);
  }

  .create-core-option {
    grid-template-columns: 2.25rem minmax(0, 1fr) 1rem;
  }

  .create-core-option > .status {
    grid-column: 2;
  }

  .create-option-check {
    grid-column: 3;
    grid-row: 1;
  }

  .instance-create-form__actions {
    align-items: stretch;
    flex-direction: column-reverse;
  }

  .instance-create-form__actions > div {
    justify-content: space-between;
  }

  .filter-bar,
  .instance-card-grid {
    grid-template-columns: 1fr;
  }
}
</style>
