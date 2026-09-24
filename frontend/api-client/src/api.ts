/**
 * MCNP Web API 接口层（PLAN.md 第 4.2 节，基础路径 /api/v1）。
 *
 * McnpApi 是前端访问后端的唯一入口。当前由 Mock 实现驱动原型；
 * M1 起由 OpenAPI 生成的真实客户端实现同一接口。
 */

import type {
  AuditEvent,
  ConfigDocument,
  ConfigField,
  CoreNode,
  CpuPolicy,
  CpuTopology,
  DeviceSession,
  ExtensionInstall,
  ExtensionProject,
  FileEntry,
  Group,
  ImageBuild,
  ImageInfo,
  InstallTemplate,
  Instance,
  InstanceRuntime,
  LoginResult,
  ManagedRuntime,
  ManagedRuntimeKind,
  McdrSettings,
  MountBinding,
  PanelSettings,
  Permission,
  RegistryInfo,
  Schedule,
  ScheduleExecution,
  ScheduleTrigger,
  ServerType,
  Task,
  User,
} from './domain';

/** 异步操作受理结果：对应 HTTP 202 Accepted。 */
export interface Accepted {
  taskId: string;
}

export interface InstanceFilter {
  coreId?: string;
  state?: Instance['state'];
  tag?: string;
  keyword?: string;
}

export interface CreateInstanceInput {
  coreId: string;
  name: string;
  /** 一键搭建模板；null 表示空白实例。 */
  templateId: string | null;
  serverType: ServerType;
  version: string;
  javaRuntimeId: string | null;
  workDir: string;
  supervisorMode: 'DIRECT' | 'MCDR';
  expiresAt: number | null;
}

/** 实例设置补丁；各字段对应独立权限点（PLAN 第 4.4 节）。 */
export interface InstanceSettingsPatch {
  // instance.settings.basic
  name?: string;
  serverType?: ServerType;
  expiresAt?: number | null;
  tags?: string[];
  backupEnabled?: boolean;
  backupTargetDir?: string | null;
  // instance.settings.launch
  launchCommand?: string;
  updateCommand?: string | null;
  javaRuntimeId?: string | null;
  supervisorMode?: 'DIRECT' | 'MCDR';
  /** MCDR 包装器设置（部分字段更新）。 */
  mcdrSettings?: Partial<McdrSettings>;
  // instance.settings.path
  workDir?: string;
  // instance.settings.container
  runtimeMode?: 'HOST' | 'CONTAINER';
  containerImage?: string | null;
  containerPorts?: string[];
  containerEnv?: Record<string, string>;
  containerMounts?: MountBinding[];
}

export interface CreateCoreInput {
  name: string;
  address: string;
  /** 节点连接密钥（PSK）；仅传输与信封加密落库，永不明文回显。 */
  psk: string;
}

export interface CreateScheduleInput {
  name: string;
  trigger: ScheduleTrigger;
  action: string;
  enabled: boolean;
}

export interface CreateUserInput {
  username: string;
  displayName: string;
  password: string;
  groupIds: string[];
}

export interface CreateGroupInput {
  name: string;
  description: string;
  permissions: Permission[];
  instanceScopes: Group['instanceScopes'];
}

export interface AuditFilter {
  actorId?: string;
  result?: AuditEvent['result'];
  keyword?: string;
  limit?: number;
}

/** MCNP Panel Web API 客户端接口。 */
export interface McnpApi {
  readonly auth: {
    /** 是否为首次运行（无任何用户，需要初始化管理员）。 */
    setupRequired(): Promise<boolean>;
    setupAdmin(input: { username: string; displayName: string; password: string }): Promise<LoginResult>;
    login(input: { username: string; password: string; deviceName: string }): Promise<LoginResult>;
    logout(): Promise<void>;
    /** 恢复当前会话；无有效会话时返回 null。 */
    me(): Promise<{ user: User; permissions: Permission[] } | null>;
    changePassword(oldPassword: string, newPassword: string): Promise<void>;
    listSessions(): Promise<DeviceSession[]>;
    revokeSession(sessionId: string): Promise<void>;
  };

  readonly cores: {
    list(): Promise<CoreNode[]>;
    get(coreId: string): Promise<CoreNode>;
    create(input: CreateCoreInput): Promise<CoreNode>;
    remove(coreId: string): Promise<Accepted>;
    /** 连通性测试：重建连接并返回延迟（毫秒）。 */
    testConnection(coreId: string): Promise<{ latencyMs: number }>;
    cpuTopology(coreId: string): Promise<CpuTopology>;
  };

  readonly instances: {
    list(filter?: InstanceFilter): Promise<Instance[]>;
    get(instanceId: string): Promise<Instance>;
    create(input: CreateInstanceInput): Promise<Accepted & { instanceId: string }>;
    delete(instanceId: string): Promise<Accepted>;
    start(instanceId: string): Promise<Accepted>;
    stop(instanceId: string): Promise<Accepted>;
    kill(instanceId: string): Promise<Accepted>;
    restart(instanceId: string): Promise<Accepted>;
    runtime(instanceId: string): Promise<InstanceRuntime>;
    sendCommand(instanceId: string, command: string): Promise<void>;
    updateSettings(instanceId: string, patch: InstanceSettingsPatch): Promise<Instance>;
    cpuPolicy(instanceId: string): Promise<CpuPolicy>;
    updateCpuPolicy(instanceId: string, policy: Omit<CpuPolicy, 'instanceId' | 'status'>): Promise<CpuPolicy>;
  };

  readonly environments: {
    list(coreId: string): Promise<ManagedRuntime[]>;
    /** 某节点上可安装的版本清单（来源清单驱动）。 */
    availableVersions(coreId: string, kind: ManagedRuntimeKind): Promise<string[]>;
    install(coreId: string, kind: ManagedRuntimeKind, version: string): Promise<Accepted>;
    remove(runtimeId: string): Promise<Accepted>;
  };

  readonly templates: {
    list(): Promise<InstallTemplate[]>;
  };

  readonly files: {
    list(instanceId: string, path: string): Promise<FileEntry[]>;
    read(instanceId: string, path: string): Promise<string>;
    write(instanceId: string, path: string, content: string): Promise<void>;
    mkdir(instanceId: string, path: string): Promise<void>;
    remove(instanceId: string, path: string): Promise<void>;
    rename(instanceId: string, from: string, to: string): Promise<void>;
  };

  readonly configs: {
    list(instanceId: string): Promise<ConfigDocument[]>;
    fields(instanceId: string, configId: string): Promise<ConfigField[]>;
    /** round-trip 补丁：携带 revision 做乐观并发控制。 */
    updateFields(
      instanceId: string,
      configId: string,
      revision: number,
      changes: Record<string, unknown>,
    ): Promise<{ revision: number }>;
    readRaw(instanceId: string, configId: string): Promise<string>;
    writeRaw(instanceId: string, configId: string, revision: number, content: string): Promise<{ revision: number }>;
  };

  readonly extensions: {
    search(instanceId: string, query: string): Promise<ExtensionProject[]>;
    installed(instanceId: string): Promise<ExtensionInstall[]>;
    install(instanceId: string, project: ExtensionProject): Promise<Accepted>;
    remove(instanceId: string, installId: string): Promise<Accepted>;
  };

  readonly images: {
    list(coreId: string): Promise<ImageInfo[]>;
    /** 拉取镜像；registryId 为空时按仓库 priority 顺序查找。 */
    pull(coreId: string, reference: string, registryId?: string): Promise<Accepted>;
    remove(coreId: string, imageId: string): Promise<Accepted>;
    builds(coreId: string): Promise<ImageBuild[]>;
    build(coreId: string, tag: string, dockerfile: string): Promise<Accepted & { buildId: string }>;
    registries(coreId: string): Promise<RegistryInfo[]>;
    addRegistry(coreId: string, input: { name: string; url: string }): Promise<RegistryInfo>;
    removeRegistry(coreId: string, registryId: string): Promise<void>;
  };

  readonly schedules: {
    list(instanceId: string): Promise<Schedule[]>;
    create(instanceId: string, input: CreateScheduleInput): Promise<Schedule>;
    update(instanceId: string, scheduleId: string, patch: Partial<CreateScheduleInput>): Promise<Schedule>;
    remove(instanceId: string, scheduleId: string): Promise<void>;
    executions(scheduleId: string): Promise<ScheduleExecution[]>;
  };

  readonly tasks: {
    list(): Promise<Task[]>;
    cancel(taskId: string): Promise<void>;
  };

  readonly users: {
    list(): Promise<User[]>;
    create(input: CreateUserInput): Promise<User>;
    update(userId: string, patch: { displayName?: string; groupIds?: string[]; disabled?: boolean }): Promise<User>;
    resetPassword(userId: string, newPassword: string): Promise<void>;
    remove(userId: string): Promise<void>;
    groups(): Promise<Group[]>;
    createGroup(input: CreateGroupInput): Promise<Group>;
    updateGroup(groupId: string, patch: Partial<CreateGroupInput>): Promise<Group>;
    removeGroup(groupId: string): Promise<void>;
  };

  readonly audit: {
    list(filter?: AuditFilter): Promise<AuditEvent[]>;
  };

  /** Panel 级默认设置：读取不限权限（创建向导需要），修改要求 user.manage。 */
  readonly panel: {
    getSettings(): Promise<PanelSettings>;
    updateSettings(patch: Partial<PanelSettings>): Promise<PanelSettings>;
  };
}
