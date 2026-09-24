/**
 * MockApi：McnpApi 的内存实现，驱动前端原型。
 *
 * 行为对齐 PLAN.md：
 * - 耗时操作返回 { taskId }（对应 202 Accepted），经任务引擎异步推进；
 * - 实例状态按第 6 节状态机流转，状态变更以事件广播（不由请求成功推断）；
 * - 权限点在服务端侧校验（PLAN 4.4），越权抛 403 / 404。
 */

import type {
  AuditFilter,
  CreateCoreInput,
  CreateGroupInput,
  CreateInstanceInput,
  CreateScheduleInput,
  CreateUserInput,
  InstanceFilter,
  InstanceSettingsPatch,
  McnpApi,
} from '../api';
import type {
  AuditEvent,
  CoreNode,
  CpuPolicy,
  ExtensionInstall,
  Group,
  ImageBuild,
  ImageInfo,
  Instance,
  LoginResult,
  ManagedRuntime,
  Permission,
  RegistryInfo,
  Schedule,
  User,
} from '../domain';
import { ApiError, latency, MockStore } from './store';

/** 新实例的默认目录骨架。 */
function seedDefaultFiles(store: MockStore, instanceId: string): void {
  const now = Date.now();
  store.state.fileTree.set(`${instanceId}:/`, [
    { name: 'logs', path: '/logs', isDir: true, sizeBytes: 0, modifiedAt: now },
    { name: 'server.properties', path: '/server.properties', isDir: false, sizeBytes: 512, modifiedAt: now },
    { name: 'eula.txt', path: '/eula.txt', isDir: false, sizeBytes: 10, modifiedAt: now },
  ]);
  store.state.fileTree.set(`${instanceId}:/logs`, []);
  store.state.fileContents.set(`${instanceId}:/server.properties`, '#Minecraft server properties\nmotd=A Minecraft Server\nserver-port=25565\n');
  store.state.fileContents.set(`${instanceId}:/eula.txt`, 'eula=true\n');
}

const START_LOGS = [
  'Starting mcnp-supervised server...',
  'Loading libraries, please wait...',
  'Environment: Java 21.0.5, 4G heap',
  'Preparing level "world"...',
  'Done (3.8s)! For help, type "help"',
];

const STOP_LOGS = ['Stopping the server...', 'Saving players and worlds...', 'All dimensions saved to disk.', 'Server stopped.'];

export class MockApi implements McnpApi {
  constructor(private readonly store: MockStore) {}

  private recordAudit(action: string, target: string, result: AuditEvent['result'], detail: string | null = null): void {
    const actor = this.store.currentUserId
      ? (this.store.state.users.find((u) => u.id === this.store.currentUserId) ?? null)
      : null; // 登录前的操作（如 login 本身）为匿名
    this.store.state.audit.unshift({
      id: this.store.nextId('aud'),
      actorId: actor?.id ?? 'anonymous',
      actorName: actor?.username ?? 'anonymous',
      action,
      target,
      result,
      requestId: `req-${Math.random().toString(16).slice(2, 8)}`,
      sourceIp: '127.0.0.1',
      createdAt: Date.now(),
      detail,
    });
  }

  // -------------------------------------------------------------------------
  // auth
  // -------------------------------------------------------------------------

  readonly auth: McnpApi['auth'] = {
    setupRequired: async () => {
      await latency();
      return this.store.state.users.length === 0;
    },

    setupAdmin: async (input) => {
      await latency();
      if (this.store.state.users.length > 0) throw new ApiError(409, 'ALREADY_INITIALIZED', '已完成初始化');
      throw new ApiError(501, 'NOT_IMPLEMENTED_IN_MOCK', `Mock 种子数据已含用户，初始化流程不可达（${input.username}）`);
    },

    login: async (input) => {
      await latency();
      const user = this.store.state.users.find((u) => u.username === input.username);
      if (!user || user.disabled || this.store.state.credentials[input.username] !== input.password) {
        this.recordAudit('auth.login', input.username, 'FAILED', input.deviceName);
        throw new ApiError(401, 'BAD_CREDENTIALS', '用户名或密码错误');
      }
      this.store.currentUserId = user.id;
      user.lastLoginAt = Date.now();
      const sessionId = this.store.nextId('sess');
      this.store.state.sessions.unshift({
        id: sessionId,
        userId: user.id,
        deviceName: input.deviceName,
        platform: 'browser',
        ip: '127.0.0.1',
        createdAt: Date.now(),
        lastActiveAt: Date.now(),
        current: true,
      });
      this.recordAudit('auth.login', user.username, 'SUCCESS', input.deviceName);
      return this.loginResult(user, sessionId);
    },

    logout: async () => {
      await latency();
      this.store.currentUserId = null;
    },

    me: async () => {
      await latency();
      if (!this.store.currentUserId) return null;
      const user = this.store.currentUser();
      return { user, permissions: this.store.permissionsOf(user) };
    },

    changePassword: async (oldPassword, newPassword) => {
      const user = this.store.currentUser();
      await latency();
      if (this.store.state.credentials[user.username] !== oldPassword) {
        throw new ApiError(400, 'BAD_CREDENTIALS', '原密码不正确');
      }
      this.store.state.credentials[user.username] = newPassword;
      this.recordAudit('auth.changePassword', user.username, 'SUCCESS');
    },

    listSessions: async () => {
      const user = this.store.currentUser();
      await latency();
      return this.store.state.sessions.filter((s) => s.userId === user.id);
    },

    revokeSession: async (sessionId) => {
      const user = this.store.currentUser();
      await latency();
      const idx = this.store.state.sessions.findIndex((s) => s.id === sessionId && s.userId === user.id);
      if (idx < 0) throw new ApiError(404, 'SESSION_NOT_FOUND', '会话不存在');
      const removed = this.store.state.sessions.splice(idx, 1)[0];
      this.recordAudit('auth.revokeSession', removed?.deviceName ?? sessionId, 'SUCCESS');
    },
  };

  private loginResult(user: User, sessionId: string): LoginResult {
    return {
      user,
      permissions: this.store.permissionsOf(user),
      token: {
        // 令牌内嵌用户 ID，Mock 重启后凭它恢复会话（见 MockStore.restoreSessionByAccessToken）
        accessToken: `mock-at-${sessionId}.${user.id}`,
        refreshToken: `mock-rt-${sessionId}`,
        expiresAt: Date.now() + 2 * 3_600_000,
        deviceId: sessionId,
      },
    };
  }

  // -------------------------------------------------------------------------
  // cores
  // -------------------------------------------------------------------------

  readonly cores: McnpApi['cores'] = {
    list: async () => {
      this.store.requirePermission('core.read');
      await latency();
      return [...this.store.state.cores];
    },

    get: async (coreId) => {
      this.store.requirePermission('core.read');
      await latency();
      const core = this.store.state.cores.find((c) => c.id === coreId);
      if (!core) throw new ApiError(404, 'CORE_NOT_FOUND', '节点不存在');
      return core;
    },

    create: async (input: CreateCoreInput) => {
      this.store.requirePermission('core.manage');
      await latency();
      const core: CoreNode = {
        id: this.store.nextId('core'),
        name: input.name,
        address: input.address,
        status: 'OFFLINE',
        version: 'unknown',
        os: 'unknown',
        arch: 'unknown',
        capabilities: [],
        lastSeenAt: null,
        instanceCount: 0,
        runningCount: 0,
        cpuUsage: 0,
        memoryUsage: 0,
        createdAt: Date.now(),
      };
      this.store.state.cores.push(core);
      this.recordAudit('core.create', `${core.id} (${input.name})`, 'SUCCESS');
      return core;
    },

    remove: async (coreId) => {
      this.store.requirePermission('core.manage');
      await latency();
      const core = this.store.state.cores.find((c) => c.id === coreId);
      if (!core) throw new ApiError(404, 'CORE_NOT_FOUND', '节点不存在');
      const task = this.store.startTask({
        kind: 'CORE_REMOVE',
        title: `移除节点 ${core.name}`,
        coreId,
        durationMs: 2_000,
        onDone: () => {
          this.store.state.cores = this.store.state.cores.filter((c) => c.id !== coreId);
          // 级联清理该节点上的实例，避免孤儿数据
          const orphanIds = this.store.state.instances.filter((i) => i.coreId === coreId).map((i) => i.id);
          this.store.state.instances = this.store.state.instances.filter((i) => i.coreId !== coreId);
          for (const id of orphanIds) this.store.state.runtimes.delete(id);
        },
      });
      this.recordAudit('core.remove', `${coreId} (${core.name})`, 'SUCCESS');
      return { taskId: task.id };
    },

    testConnection: async (coreId) => {
      this.store.requirePermission('core.manage');
      const core = this.store.state.cores.find((c) => c.id === coreId);
      await latency();
      if (!core) throw new ApiError(404, 'CORE_NOT_FOUND', '节点不存在');
      if (core.status === 'OFFLINE') throw new ApiError(502, 'CORE_UNREACHABLE', '握手失败：连接被拒绝');
      core.lastSeenAt = Date.now();
      this.store.emitter.emit('core-status', { coreId, status: core.status, cpuUsage: core.cpuUsage, memoryUsage: core.memoryUsage });
      return { latencyMs: Math.round(3 + Math.random() * 30) };
    },

    cpuTopology: async (coreId) => {
      this.store.requirePermission('core.read');
      await latency();
      const topo = this.store.state.cpuTopologies.find((t) => t.coreId === coreId);
      if (!topo) throw new ApiError(404, 'TOPOLOGY_NOT_FOUND', '该节点未上报 CPU 拓扑');
      return topo;
    },
  };

  // -------------------------------------------------------------------------
  // instances
  // -------------------------------------------------------------------------

  private visibleInstances(filter?: InstanceFilter): Instance[] {
    const user = this.store.currentUser();
    return this.store.state.instances.filter((i) => {
      if (!this.store.canSeeInstance(i, user)) return false;
      if (filter?.coreId && i.coreId !== filter.coreId) return false;
      if (filter?.state && i.state !== filter.state) return false;
      if (filter?.tag && !i.tags.includes(filter.tag)) return false;
      if (filter?.keyword && !i.name.toLowerCase().includes(filter.keyword.toLowerCase())) return false;
      return true;
    });
  }

  readonly instances: McnpApi['instances'] = {
    list: async (filter) => {
      this.store.requirePermission('instance.read');
      await latency();
      return this.visibleInstances(filter);
    },

    get: async (instanceId) => {
      this.store.requirePermission('instance.read');
      await latency();
      return this.store.requireVisibleInstance(instanceId);
    },

    create: async (input: CreateInstanceInput) => {
      this.store.requirePermission('instance.create');
      await latency();
      const core = this.store.state.cores.find((c) => c.id === input.coreId);
      if (!core) throw new ApiError(404, 'CORE_NOT_FOUND', '节点不存在');
      const instance: Instance = {
        id: this.store.nextId('inst'),
        coreId: input.coreId,
        name: input.name,
        serverType: input.serverType,
        version: input.version,
        state: 'CREATED',
        workDir: input.workDir,
        javaRuntimeId: input.javaRuntimeId,
        runtimeMode: 'HOST',
        supervisorMode: input.supervisorMode,
        containerImage: null,
        containerPorts: [],
        containerEnv: {},
        containerMounts: [],
        mcdrSettings: { checkUpdate: true, autoReload: false, language: 'zh_cn' },
        backupEnabled: false,
        backupTargetDir: null,
        launchCommand: 'java -Xmx2G -jar server.jar nogui',
        updateCommand: null,
        expiresAt: input.expiresAt,
        tags: [],
        createdAt: Date.now(),
      };
      this.store.state.instances.push(instance);
      this.store.state.runtimes.set(instance.id, {
        instanceId: instance.id,
        pid: null,
        state: 'CREATED',
        startedAt: null,
        exitCode: null,
        cpuUsage: 0,
        memoryBytes: 0,
        playerCount: null,
      });
      this.store.state.cpuPolicies.set(instance.id, {
        instanceId: instance.id,
        mode: 'SHARED',
        cpuSet: [],
        exclusive: false,
        numaNodeId: null,
        status: 'requested',
      });
      seedDefaultFiles(this.store, instance.id);
      core.instanceCount += 1;
      const templateName = input.templateId ? (this.store.state.templates.find((t) => t.id === input.templateId)?.name ?? '自定义') : '空白实例';
      const task = this.store.startTask({
        kind: 'INSTANCE_CREATE',
        title: `创建实例 ${input.name}（${templateName} ${input.version}）`,
        coreId: input.coreId,
        instanceId: instance.id,
        durationMs: input.templateId ? 6_000 : 1_500,
        onDone: () => {
          this.store.emitter.emit('instance-state', { instanceId: instance.id, state: instance.state, runtime: this.store.runtimeOf(instance.id) });
        },
      });
      this.recordAudit('instance.create', `${instance.id} (${input.name})`, 'SUCCESS');
      return { taskId: task.id, instanceId: instance.id };
    },

    delete: async (instanceId) => {
      this.store.requirePermission('instance.delete');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      if (instance.state === 'RUNNING' || instance.state === 'STARTING' || instance.state === 'STOPPING') {
        throw new ApiError(409, 'INSTANCE_BUSY', '实例运行中，请先停止');
      }
      const task = this.store.startTask({
        kind: 'INSTANCE_DELETE',
        title: `删除实例 ${instance.name}`,
        coreId: instance.coreId,
        instanceId,
        durationMs: 2_000,
        onDone: () => {
          this.store.state.instances = this.store.state.instances.filter((i) => i.id !== instanceId);
          this.store.state.runtimes.delete(instanceId);
          const core = this.store.state.cores.find((c) => c.id === instance.coreId);
          if (core) core.instanceCount = Math.max(0, core.instanceCount - 1);
        },
      });
      this.recordAudit('instance.delete', `${instanceId} (${instance.name})`, 'SUCCESS');
      return { taskId: task.id };
    },

    start: async (instanceId) => {
      this.store.requirePermission('instance.control');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      if (instance.state !== 'CREATED' && instance.state !== 'STOPPED' && instance.state !== 'FAILED') {
        throw new ApiError(409, 'BAD_STATE', `当前状态 ${instance.state} 不允许启动`);
      }
      this.store.setInstanceState(instanceId, 'STARTING', { pid: null, exitCode: null });
      this.store.scheduleTransition(instanceId, 'RUNNING', 3_000, START_LOGS, {
        pid: 20000 + Math.floor(Math.random() * 9000),
        startedAt: Date.now(),
        playerCount: 0,
      });
      const task = this.store.startTask({ kind: 'INSTANCE_START', title: `启动 ${instance.name}`, coreId: instance.coreId, instanceId, durationMs: 3_000, cancellable: false });
      this.recordAudit('instance.start', `${instanceId} (${instance.name})`, 'SUCCESS');
      return { taskId: task.id };
    },

    stop: async (instanceId) => {
      this.store.requirePermission('instance.control');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      if (instance.state !== 'RUNNING') throw new ApiError(409, 'BAD_STATE', `当前状态 ${instance.state} 不允许停止`);
      this.store.setInstanceState(instanceId, 'STOPPING');
      this.store.scheduleTransition(instanceId, 'STOPPED', 2_000, STOP_LOGS, { pid: null, startedAt: null, exitCode: 0, cpuUsage: 0, memoryBytes: 0, playerCount: 0 });
      const task = this.store.startTask({ kind: 'INSTANCE_STOP', title: `停止 ${instance.name}`, coreId: instance.coreId, instanceId, durationMs: 2_000, cancellable: false });
      this.recordAudit('instance.stop', `${instanceId} (${instance.name})`, 'SUCCESS');
      return { taskId: task.id };
    },

    kill: async (instanceId) => {
      this.store.requirePermission('instance.control');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      if (instance.state !== 'RUNNING' && instance.state !== 'STARTING') {
        throw new ApiError(409, 'BAD_STATE', `当前状态 ${instance.state} 不允许强制终止`);
      }
      this.store.emitter.emit('console', { instanceId, line: '进程被强制终止 (SIGKILL)', stream: 'stderr', at: Date.now() });
      this.store.setInstanceState(instanceId, 'STOPPED', { pid: null, startedAt: null, exitCode: 137, cpuUsage: 0, memoryBytes: 0, playerCount: 0 });
      const task = this.store.startTask({ kind: 'INSTANCE_KILL', title: `强制终止 ${instance.name}`, coreId: instance.coreId, instanceId, durationMs: 600, cancellable: false });
      this.recordAudit('instance.kill', `${instanceId} (${instance.name})`, 'SUCCESS');
      return { taskId: task.id };
    },

    restart: async (instanceId) => {
      this.store.requirePermission('instance.control');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      if (instance.state !== 'RUNNING') throw new ApiError(409, 'BAD_STATE', '仅运行中的实例可重启');
      this.store.setInstanceState(instanceId, 'STOPPING');
      this.store.scheduleTransition(instanceId, 'STARTING', 1_500, STOP_LOGS, { pid: null, playerCount: 0 });
      this.store.scheduleTransition(instanceId, 'RUNNING', 4_500, ['', ...START_LOGS], {
        pid: 20000 + Math.floor(Math.random() * 9000),
        startedAt: Date.now(),
      });
      const task = this.store.startTask({ kind: 'INSTANCE_RESTART', title: `重启 ${instance.name}`, coreId: instance.coreId, instanceId, durationMs: 4_500, cancellable: false });
      this.recordAudit('instance.restart', `${instanceId} (${instance.name})`, 'SUCCESS');
      return { taskId: task.id };
    },

    runtime: async (instanceId) => {
      this.store.requirePermission('instance.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      return this.store.runtimeOf(instanceId);
    },

    sendCommand: async (instanceId, command) => {
      this.store.requirePermission('instance.console.write');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      if (instance.state !== 'RUNNING') throw new ApiError(409, 'BAD_STATE', '实例未运行，无法发送命令');
      this.store.emitter.emit('console', { instanceId, line: `> ${command}`, stream: 'stdout', at: Date.now() });
      setTimeout(() => {
        this.store.emitter.emit('console', { instanceId, line: `[Server thread/INFO]: 已执行：${command}`, stream: 'stdout', at: Date.now() });
      }, 300);
    },

    updateSettings: async (instanceId, patch: InstanceSettingsPatch) => {
      const permissionByField: [keyof InstanceSettingsPatch, Permission][] = [
        ['name', 'instance.settings.basic'],
        ['serverType', 'instance.settings.basic'],
        ['expiresAt', 'instance.settings.basic'],
        ['tags', 'instance.settings.basic'],
        ['backupEnabled', 'instance.settings.basic'],
        ['backupTargetDir', 'instance.settings.basic'],
        ['launchCommand', 'instance.settings.launch'],
        ['updateCommand', 'instance.settings.launch'],
        ['javaRuntimeId', 'instance.settings.launch'],
        ['supervisorMode', 'instance.settings.launch'],
        ['mcdrSettings', 'instance.settings.launch'],
        ['workDir', 'instance.settings.path'],
        ['runtimeMode', 'instance.settings.container'],
        ['containerImage', 'instance.settings.container'],
        ['containerPorts', 'instance.settings.container'],
        ['containerEnv', 'instance.settings.container'],
        ['containerMounts', 'instance.settings.container'],
      ];
      for (const [field, permission] of permissionByField) {
        if (patch[field] !== undefined) this.store.requirePermission(permission);
      }
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      Object.assign(instance, {
        name: patch.name ?? instance.name,
        serverType: patch.serverType ?? instance.serverType,
        expiresAt: patch.expiresAt === undefined ? instance.expiresAt : patch.expiresAt,
        tags: patch.tags ?? instance.tags,
        backupEnabled: patch.backupEnabled ?? instance.backupEnabled,
        backupTargetDir: patch.backupTargetDir === undefined ? instance.backupTargetDir : patch.backupTargetDir,
        launchCommand: patch.launchCommand ?? instance.launchCommand,
        updateCommand: patch.updateCommand === undefined ? instance.updateCommand : patch.updateCommand,
        javaRuntimeId: patch.javaRuntimeId === undefined ? instance.javaRuntimeId : patch.javaRuntimeId,
        supervisorMode: patch.supervisorMode ?? instance.supervisorMode,
        mcdrSettings: patch.mcdrSettings ? { ...instance.mcdrSettings, ...patch.mcdrSettings } : instance.mcdrSettings,
        workDir: patch.workDir ?? instance.workDir,
        runtimeMode: patch.runtimeMode ?? instance.runtimeMode,
        containerImage: patch.containerImage === undefined ? instance.containerImage : patch.containerImage,
        containerPorts: patch.containerPorts ?? instance.containerPorts,
        containerEnv: patch.containerEnv ?? instance.containerEnv,
        containerMounts: patch.containerMounts ?? instance.containerMounts,
      });
      this.recordAudit('instance.settings', `${instanceId} (${instance.name})`, 'SUCCESS', Object.keys(patch).join(', '));
      return instance;
    },

    cpuPolicy: async (instanceId) => {
      this.store.requirePermission('instance.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      const policy = this.store.state.cpuPolicies.get(instanceId);
      if (!policy) throw new ApiError(404, 'POLICY_NOT_FOUND', '该实例未配置 CPU 策略');
      return policy;
    },

    updateCpuPolicy: async (instanceId, policy) => {
      this.store.requirePermission('instance.settings.cpu');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      const next: CpuPolicy = { ...policy, instanceId, status: 'applied' };
      this.store.state.cpuPolicies.set(instanceId, next);
      this.recordAudit('instance.settings.cpu', instanceId, 'SUCCESS', `${policy.mode} cpus=[${policy.cpuSet.join(',')}]`);
      return next;
    },
  };

  // -------------------------------------------------------------------------
  // environments
  // -------------------------------------------------------------------------

  readonly environments: McnpApi['environments'] = {
    list: async (coreId) => {
      this.store.requirePermission('environment.read');
      await latency();
      return this.store.state.managedRuntimes.filter((r) => r.coreId === coreId);
    },

    availableVersions: async (_coreId, kind) => {
      this.store.requirePermission('environment.read');
      await latency();
      if (kind === 'JAVA') return ['21.0.5+11', '17.0.13+11', '1.8.0_432'];
      if (kind === 'NODE') return ['22.12.0', '20.18.1', '18.20.5'];
      return ['3.13.1', '3.12.8', '3.11.11'];
    },

    install: async (coreId, kind, version) => {
      this.store.requirePermission('environment.manage');
      await latency();
      const core = this.store.state.cores.find((c) => c.id === coreId);
      if (!core) throw new ApiError(404, 'CORE_NOT_FOUND', '节点不存在');
      const task = this.store.startTask({
        kind: 'RUNTIME_INSTALL',
        title: `安装 ${kind} ${version}（${core.name}）`,
        coreId,
        durationMs: 5_000,
        onDone: () => {
          const runtime: ManagedRuntime = {
            id: this.store.nextId('rt'),
            coreId,
            kind,
            version,
            majorVersion: Number.parseInt(version, 10) || 0,
            path: `/opt/mcnp/runtimes/${kind.toLowerCase()}/${version}`,
            sha256: Math.random().toString(16).slice(2, 8),
            installedAt: Date.now(),
            sizeBytes: 150_000_000,
          };
          this.store.state.managedRuntimes.push(runtime);
        },
      });
      task.message = '下载中…';
      this.recordAudit('environment.install', `${kind} ${version} @ ${core.name}`, 'SUCCESS');
      return { taskId: task.id };
    },

    remove: async (runtimeId) => {
      this.store.requirePermission('environment.manage');
      await latency();
      const runtime = this.store.state.managedRuntimes.find((r) => r.id === runtimeId);
      if (!runtime) throw new ApiError(404, 'RUNTIME_NOT_FOUND', '运行时不存在');
      const inUse = this.store.state.instances.some((i) => i.javaRuntimeId === runtimeId);
      if (inUse) throw new ApiError(409, 'RUNTIME_IN_USE', '该运行时正被实例引用，无法删除');
      const task = this.store.startTask({
        kind: 'RUNTIME_REMOVE',
        title: `删除 ${runtime.kind} ${runtime.version}`,
        coreId: runtime.coreId,
        durationMs: 1_500,
        onDone: () => {
          this.store.state.managedRuntimes = this.store.state.managedRuntimes.filter((r) => r.id !== runtimeId);
        },
      });
      this.recordAudit('environment.remove', `${runtime.kind} ${runtime.version}`, 'SUCCESS');
      return { taskId: task.id };
    },
  };

  // -------------------------------------------------------------------------
  // templates
  // -------------------------------------------------------------------------

  readonly templates: McnpApi['templates'] = {
    list: async () => {
      this.store.requirePermission('instance.read');
      await latency();
      return [...this.store.state.templates];
    },
  };

  // -------------------------------------------------------------------------
  // files
  // -------------------------------------------------------------------------

  private fileKey(instanceId: string, path: string): string {
    const normalized = path === '' ? '/' : path;
    return `${instanceId}:${normalized}`;
  }

  readonly files: McnpApi['files'] = {
    list: async (instanceId, path) => {
      this.store.requirePermission('file.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      const entries = this.store.state.fileTree.get(this.fileKey(instanceId, path));
      if (!entries) throw new ApiError(404, 'PATH_NOT_FOUND', `目录不存在：${path}`);
      return entries;
    },

    read: async (instanceId, path) => {
      this.store.requirePermission('file.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      const content = this.store.state.fileContents.get(this.fileKey(instanceId, path));
      if (content === undefined) throw new ApiError(415, 'NOT_TEXT', '该文件不是文本或不存在');
      return content;
    },

    write: async (instanceId, path, content) => {
      this.store.requirePermission('file.write');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      this.store.state.fileContents.set(this.fileKey(instanceId, path), content);
      this.recordAudit('file.write', `${instance.name}:${path}`, 'SUCCESS', `${content.length} 字节`);
    },

    mkdir: async (instanceId, path) => {
      this.store.requirePermission('file.write');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      const parent = path.slice(0, path.lastIndexOf('/')) || '/';
      const name = path.slice(path.lastIndexOf('/') + 1);
      const entries = this.store.state.fileTree.get(this.fileKey(instanceId, parent));
      if (!entries) throw new ApiError(404, 'PATH_NOT_FOUND', `目录不存在：${parent}`);
      entries.push({ name, path, isDir: true, sizeBytes: 0, modifiedAt: Date.now() });
      this.store.state.fileTree.set(this.fileKey(instanceId, path), []);
      this.recordAudit('file.mkdir', `${instance.name}:${path}`, 'SUCCESS');
    },

    remove: async (instanceId, path) => {
      this.store.requirePermission('file.write');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      const parent = path.slice(0, path.lastIndexOf('/')) || '/';
      const entries = this.store.state.fileTree.get(this.fileKey(instanceId, parent));
      const idx = entries?.findIndex((e) => e.path === path) ?? -1;
      if (!entries || idx < 0) throw new ApiError(404, 'PATH_NOT_FOUND', `路径不存在：${path}`);
      entries.splice(idx, 1);
      this.store.state.fileTree.delete(this.fileKey(instanceId, path));
      this.store.state.fileContents.delete(this.fileKey(instanceId, path));
      this.recordAudit('file.remove', `${instance.name}:${path}`, 'SUCCESS');
    },

    rename: async (instanceId, from, to) => {
      this.store.requirePermission('file.write');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      const parent = from.slice(0, from.lastIndexOf('/')) || '/';
      const entries = this.store.state.fileTree.get(this.fileKey(instanceId, parent));
      const entry = entries?.find((e) => e.path === from);
      if (!entry) throw new ApiError(404, 'PATH_NOT_FOUND', `路径不存在：${from}`);
      entry.path = to;
      entry.name = to.slice(to.lastIndexOf('/') + 1);
      const content = this.store.state.fileContents.get(this.fileKey(instanceId, from));
      if (content !== undefined) {
        this.store.state.fileContents.delete(this.fileKey(instanceId, from));
        this.store.state.fileContents.set(this.fileKey(instanceId, to), content);
      }
      this.recordAudit('file.rename', `${instance.name}:${from} → ${to}`, 'SUCCESS');
    },
  };

  // -------------------------------------------------------------------------
  // configs
  // -------------------------------------------------------------------------

  readonly configs: McnpApi['configs'] = {
    list: async (instanceId) => {
      this.store.requirePermission('config.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      return this.store.state.configs.filter((c) => c.instanceId === instanceId);
    },

    fields: async (instanceId, configId) => {
      this.store.requirePermission('config.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      const fields = this.store.state.configFields.get(configId);
      if (!fields) throw new ApiError(404, 'CONFIG_NOT_FOUND', '配置不存在或未识别');
      return fields;
    },

    updateFields: async (instanceId, configId, revision, changes) => {
      this.store.requirePermission('config.write');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      const doc = this.store.state.configs.find((c) => c.id === configId);
      const fields = this.store.state.configFields.get(configId);
      if (!doc || !fields) throw new ApiError(404, 'CONFIG_NOT_FOUND', '配置不存在或未识别');
      if (doc.revision !== revision) throw new ApiError(409, 'REVISION_CONFLICT', '配置已被他人修改，请刷新后重试');
      for (const field of fields) {
        const next = changes[field.key];
        if (next !== undefined) field.value = next;
      }
      doc.revision += 1;
      doc.updatedAt = Date.now();
      this.recordAudit('config.write', `${instance.name}:${doc.path}`, 'SUCCESS', `revision ${revision} → ${doc.revision}，字段 ${Object.keys(changes).join('、')}`);
      return { revision: doc.revision };
    },

    readRaw: async (instanceId, configId) => {
      this.store.requirePermission('config.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      const raw = this.store.state.configRaw.get(configId);
      if (raw === undefined) throw new ApiError(404, 'CONFIG_NOT_FOUND', '配置不存在');
      return raw;
    },

    writeRaw: async (instanceId, configId, revision, content) => {
      this.store.requirePermission('config.write');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      const doc = this.store.state.configs.find((c) => c.id === configId);
      if (!doc) throw new ApiError(404, 'CONFIG_NOT_FOUND', '配置不存在');
      if (doc.revision !== revision) throw new ApiError(409, 'REVISION_CONFLICT', '配置已被他人修改，请刷新后重试');
      this.store.state.configRaw.set(configId, content);
      doc.revision += 1;
      doc.updatedAt = Date.now();
      this.recordAudit('config.write', `${instance.name}:${doc.path}`, 'SUCCESS', `原始文本写入，revision ${revision} → ${doc.revision}`);
      return { revision: doc.revision };
    },
  };

  // -------------------------------------------------------------------------
  // extensions
  // -------------------------------------------------------------------------

  readonly extensions: McnpApi['extensions'] = {
    search: async (instanceId, query) => {
      this.store.requirePermission('extension.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      const q = query.toLowerCase();
      return this.store.state.extensionCatalog.filter(
        (p) => !q || p.name.toLowerCase().includes(q) || p.summary.toLowerCase().includes(q),
      );
    },

    installed: async (instanceId) => {
      this.store.requirePermission('extension.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      return this.store.state.extensions.filter((e) => e.instanceId === instanceId);
    },

    install: async (instanceId, project) => {
      this.store.requirePermission('extension.manage');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      if (this.store.state.extensions.some((e) => e.instanceId === instanceId && e.projectId === project.projectId)) {
        throw new ApiError(409, 'ALREADY_INSTALLED', `${project.name} 已安装`);
      }
      const task = this.store.startTask({
        kind: 'EXTENSION_INSTALL',
        title: `安装 ${project.name}（${instance.name}）`,
        coreId: instance.coreId,
        instanceId,
        durationMs: 4_000,
        onDone: () => {
          const install: ExtensionInstall = {
            id: this.store.nextId('ext'),
            instanceId,
            kind: project.kind,
            source: project.source,
            projectId: project.projectId,
            name: project.name,
            version: 'latest',
            sha256: Math.random().toString(16).slice(2, 8),
            fileName: `${project.name}-latest.jar`,
            installedAt: Date.now(),
            updateAvailable: null,
          };
          this.store.state.extensions.push(install);
        },
      });
      this.recordAudit('extension.manage', `${instance.name}`, 'SUCCESS', `安装 ${project.name} (${project.source})`);
      return { taskId: task.id };
    },

    remove: async (instanceId, installId) => {
      this.store.requirePermission('extension.manage');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      const install = this.store.state.extensions.find((e) => e.id === installId);
      if (!install) throw new ApiError(404, 'EXTENSION_NOT_FOUND', '扩展不存在');
      const task = this.store.startTask({
        kind: 'EXTENSION_REMOVE',
        title: `删除 ${install.name}（${instance.name}）`,
        coreId: instance.coreId,
        instanceId,
        durationMs: 1_200,
        onDone: () => {
          this.store.state.extensions = this.store.state.extensions.filter((e) => e.id !== installId);
        },
      });
      this.recordAudit('extension.manage', `${instance.name}`, 'SUCCESS', `删除 ${install.name}`);
      return { taskId: task.id };
    },
  };

  // -------------------------------------------------------------------------
  // images
  // -------------------------------------------------------------------------

  readonly images: McnpApi['images'] = {
    list: async (coreId) => {
      this.store.requirePermission('image.read');
      await latency();
      return this.store.state.images.filter((i) => i.coreId === coreId);
    },

    pull: async (coreId, reference, registryId) => {
      this.store.requirePermission('image.manage');
      await latency();
      const registry = registryId ? this.store.state.registries.find((r) => r.id === registryId && r.coreId === coreId) : undefined;
      if (registryId && !registry) throw new ApiError(404, 'REGISTRY_NOT_FOUND', '镜像仓库不存在');
      const via = registry ? registry.name : '按仓库顺序自动查找';
      const task = this.store.startTask({
        kind: 'IMAGE_PULL',
        title: `拉取镜像 ${reference}（${via}）`,
        coreId,
        durationMs: 6_000,
        onDone: () => {
          const image: ImageInfo = {
            id: this.store.nextId('img'),
            coreId,
            repoTags: [reference],
            sizeBytes: 300_000_000,
            createdAt: Date.now(),
          };
          this.store.state.images.push(image);
        },
      });
      this.recordAudit('image.pull', reference, 'SUCCESS', `仓库：${via}`);
      return { taskId: task.id };
    },

    remove: async (coreId, imageId) => {
      this.store.requirePermission('image.manage');
      await latency();
      const image = this.store.state.images.find((i) => i.id === imageId);
      if (!image) throw new ApiError(404, 'IMAGE_NOT_FOUND', '镜像不存在');
      const task = this.store.startTask({
        kind: 'IMAGE_REMOVE',
        title: `删除镜像 ${image.repoTags[0] ?? imageId}`,
        coreId,
        durationMs: 1_500,
        onDone: () => {
          this.store.state.images = this.store.state.images.filter((i) => i.id !== imageId);
        },
      });
      this.recordAudit('image.remove', image.repoTags[0] ?? imageId, 'SUCCESS');
      return { taskId: task.id };
    },

    builds: async (coreId) => {
      this.store.requirePermission('image.read');
      await latency();
      return this.store.state.builds.filter((b) => b.coreId === coreId);
    },

    build: async (coreId, tag, _dockerfile) => {
      this.store.requirePermission('image.build');
      await latency();
      const build: ImageBuild = { id: this.store.nextId('build'), coreId, tag, status: 'RUNNING', startedAt: Date.now(), finishedAt: null };
      this.store.state.builds.unshift(build);
      const lines = [
        'Step 1/5 : FROM eclipse-temurin:21-jre',
        ' ---> 3f2a1b9c',
        'Step 2/5 : WORKDIR /data',
        'Step 3/5 : COPY server.jar /data/server.jar',
        'Step 4/5 : ENV EULA=TRUE',
        'Step 5/5 : ENTRYPOINT ["java","-jar","server.jar"]',
        'Successfully built 8c4d2e01',
        `Successfully tagged ${tag}`,
      ];
      lines.forEach((line, idx) => {
        setTimeout(() => {
          this.store.emitter.emit('image-build-log', { buildId: build.id, line, at: Date.now() });
        }, 600 * (idx + 1));
      });
      const task = this.store.startTask({
        kind: 'IMAGE_BUILD',
        title: `构建镜像 ${tag}`,
        coreId,
        durationMs: 600 * (lines.length + 1),
        onDone: () => {
          build.status = 'SUCCESS';
          build.finishedAt = Date.now();
          this.store.state.images.push({ id: this.store.nextId('img'), coreId, repoTags: [tag], sizeBytes: 420_000_000, createdAt: Date.now() });
        },
      });
      this.recordAudit('image.build', tag, 'SUCCESS');
      return { taskId: task.id, buildId: build.id };
    },

    registries: async (coreId) => {
      this.store.requirePermission('image.read');
      await latency();
      return this.store.state.registries
        .filter((r) => r.coreId === coreId)
        .sort((a, b) => a.priority - b.priority);
    },

    addRegistry: async (coreId, input) => {
      this.store.requirePermission('image.manage');
      await latency();
      const siblings = this.store.state.registries.filter((r) => r.coreId === coreId);
      const registry: RegistryInfo = {
        id: this.store.nextId('reg'),
        coreId,
        name: input.name,
        url: input.url,
        priority: siblings.length === 0 ? 0 : Math.max(...siblings.map((r) => r.priority)) + 1,
      };
      this.store.state.registries.push(registry);
      this.recordAudit('image.registry.add', `${input.name} (${input.url})`, 'SUCCESS');
      return registry;
    },

    removeRegistry: async (coreId, registryId) => {
      this.store.requirePermission('image.manage');
      await latency();
      const registry = this.store.state.registries.find((r) => r.id === registryId && r.coreId === coreId);
      if (!registry) throw new ApiError(404, 'REGISTRY_NOT_FOUND', '镜像仓库不存在');
      this.store.state.registries = this.store.state.registries.filter((r) => r.id !== registryId);
      this.recordAudit('image.registry.remove', `${registry.name} (${registry.url})`, 'SUCCESS');
    },
  };

  // -------------------------------------------------------------------------
  // schedules
  // -------------------------------------------------------------------------

  readonly schedules: McnpApi['schedules'] = {
    list: async (instanceId) => {
      this.store.requirePermission('schedule.read');
      this.store.requireVisibleInstance(instanceId);
      await latency();
      return this.store.state.schedules.filter((s) => s.instanceId === instanceId);
    },

    create: async (instanceId, input: CreateScheduleInput) => {
      this.store.requirePermission('schedule.manage');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      const schedule: Schedule = {
        id: this.store.nextId('sch'),
        instanceId,
        name: input.name,
        enabled: input.enabled,
        trigger: input.trigger,
        action: input.action,
        lastRunAt: null,
        nextRunAt: input.trigger.kind === 'CRON' ? Date.now() + 3_600_000 : null,
      };
      this.store.state.schedules.push(schedule);
      this.recordAudit('schedule.manage', `${instance.name}/${input.name}`, 'SUCCESS');
      return schedule;
    },

    update: async (instanceId, scheduleId, patch) => {
      this.store.requirePermission('schedule.manage');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      const schedule = this.store.state.schedules.find((s) => s.id === scheduleId);
      if (!schedule) throw new ApiError(404, 'SCHEDULE_NOT_FOUND', '计划任务不存在');
      Object.assign(schedule, patch);
      this.recordAudit('schedule.manage', `${instance.name}/${schedule.name}`, 'SUCCESS');
      return schedule;
    },

    remove: async (instanceId, scheduleId) => {
      this.store.requirePermission('schedule.manage');
      const instance = this.store.requireVisibleInstance(instanceId);
      await latency();
      const schedule = this.store.state.schedules.find((s) => s.id === scheduleId);
      if (!schedule) throw new ApiError(404, 'SCHEDULE_NOT_FOUND', '计划任务不存在');
      this.store.state.schedules = this.store.state.schedules.filter((s) => s.id !== scheduleId);
      this.recordAudit('schedule.manage', `${instance.name}/${schedule.name}`, 'SUCCESS');
    },

    executions: async (scheduleId) => {
      this.store.requirePermission('schedule.read');
      await latency();
      return this.store.state.executions.filter((e) => e.scheduleId === scheduleId);
    },
  };

  // -------------------------------------------------------------------------
  // tasks
  // -------------------------------------------------------------------------

  readonly tasks: McnpApi['tasks'] = {
    list: async () => {
      this.store.currentUser();
      await latency();
      return [...this.store.state.tasks];
    },

    cancel: async (taskId) => {
      this.store.currentUser();
      await latency();
      this.store.cancelTask(taskId);
    },
  };

  // -------------------------------------------------------------------------
  // users
  // -------------------------------------------------------------------------

  readonly users: McnpApi['users'] = {
    list: async () => {
      this.store.requirePermission('user.read');
      await latency();
      return [...this.store.state.users];
    },

    create: async (input: CreateUserInput) => {
      this.store.requirePermission('user.manage');
      await latency();
      if (this.store.state.users.some((u) => u.username === input.username)) {
        throw new ApiError(409, 'USERNAME_TAKEN', '用户名已存在');
      }
      const user: User = {
        id: this.store.nextId('u'),
        username: input.username,
        displayName: input.displayName,
        groupIds: input.groupIds,
        disabled: false,
        lastLoginAt: null,
        createdAt: Date.now(),
      };
      this.store.state.users.push(user);
      this.store.state.credentials[input.username] = input.password;
      this.recordAudit('user.create', input.username, 'SUCCESS');
      return user;
    },

    update: async (userId, patch) => {
      this.store.requirePermission('user.manage');
      await latency();
      const user = this.store.state.users.find((u) => u.id === userId);
      if (!user) throw new ApiError(404, 'USER_NOT_FOUND', '用户不存在');
      Object.assign(user, patch);
      this.recordAudit('user.update', user.username, 'SUCCESS', Object.keys(patch).join(', '));
      return user;
    },

    resetPassword: async (userId, newPassword) => {
      this.store.requirePermission('user.manage');
      await latency();
      const user = this.store.state.users.find((u) => u.id === userId);
      if (!user) throw new ApiError(404, 'USER_NOT_FOUND', '用户不存在');
      this.store.state.credentials[user.username] = newPassword;
      this.recordAudit('user.resetPassword', user.username, 'SUCCESS', '密码已重置（内容脱敏）');
    },

    remove: async (userId) => {
      this.store.requirePermission('user.manage');
      await latency();
      const user = this.store.state.users.find((u) => u.id === userId);
      if (!user) throw new ApiError(404, 'USER_NOT_FOUND', '用户不存在');
      if (user.id === this.store.currentUserId) throw new ApiError(409, 'CANNOT_REMOVE_SELF', '不能删除当前登录用户');
      this.store.state.users = this.store.state.users.filter((u) => u.id !== userId);
      this.recordAudit('user.remove', user.username, 'SUCCESS');
    },

    groups: async () => {
      this.store.requirePermission('user.read');
      await latency();
      return [...this.store.state.groups];
    },

    createGroup: async (input: CreateGroupInput) => {
      this.store.requirePermission('user.manage');
      await latency();
      const group: Group = { id: this.store.nextId('g'), ...input, builtIn: false };
      this.store.state.groups.push(group);
      this.recordAudit('user.createGroup', input.name, 'SUCCESS');
      return group;
    },

    updateGroup: async (groupId, patch) => {
      this.store.requirePermission('user.manage');
      await latency();
      const group = this.store.state.groups.find((g) => g.id === groupId);
      if (!group) throw new ApiError(404, 'GROUP_NOT_FOUND', '用户组不存在');
      if (group.builtIn && patch.permissions && patch.permissions.length < group.permissions.length) {
        throw new ApiError(409, 'BUILTIN_GROUP', '内置管理员组的权限不可裁剪');
      }
      Object.assign(group, patch);
      this.recordAudit('user.updateGroup', group.name, 'SUCCESS');
      return group;
    },

    removeGroup: async (groupId) => {
      this.store.requirePermission('user.manage');
      await latency();
      const group = this.store.state.groups.find((g) => g.id === groupId);
      if (!group) throw new ApiError(404, 'GROUP_NOT_FOUND', '用户组不存在');
      if (group.builtIn) throw new ApiError(409, 'BUILTIN_GROUP', '内置用户组不可删除');
      this.store.state.groups = this.store.state.groups.filter((g) => g.id !== groupId);
      this.recordAudit('user.removeGroup', group.name, 'SUCCESS');
    },
  };

  // -------------------------------------------------------------------------
  // audit
  // -------------------------------------------------------------------------

  readonly audit: McnpApi['audit'] = {
    list: async (filter?: AuditFilter) => {
      this.store.requirePermission('audit.read');
      await latency();
      return this.store.state.audit
        .filter((e) => {
          if (filter?.actorId && e.actorId !== filter.actorId) return false;
          if (filter?.result && e.result !== filter.result) return false;
          if (filter?.keyword) {
            const q = filter.keyword.toLowerCase();
            if (!e.action.toLowerCase().includes(q) && !e.target.toLowerCase().includes(q)) return false;
          }
          return true;
        })
        .slice(0, filter?.limit ?? 100);
    },
  };

  // -------------------------------------------------------------------------
  // panel 设置
  // -------------------------------------------------------------------------

  readonly panel: McnpApi['panel'] = {
    getSettings: async () => {
      this.store.currentUser();
      await latency();
      return { ...this.store.state.settings };
    },

    updateSettings: async (patch) => {
      this.store.requirePermission('user.manage');
      await latency();
      Object.assign(this.store.state.settings, patch);
      this.recordAudit('panel.settings', 'panel', 'SUCCESS', Object.keys(patch).join(', '));
      return { ...this.store.state.settings };
    },
  };
}
