/**
 * MCNP 领域类型（PLAN.md 第 6 节）。
 *
 * 与后端 v1 API/协议共享同一套领域模型；后端冻结 OpenAPI 后，
 * 本文件将由生成产物替换。TODO(M1): 切换为 OpenAPI 生成。
 */

// ---------------------------------------------------------------------------
// 实例状态机（PLAN.md 第 6 节状态图）
// ---------------------------------------------------------------------------

export type InstanceState = 'CREATED' | 'STARTING' | 'RUNNING' | 'STOPPING' | 'STOPPED' | 'FAILED';

// ---------------------------------------------------------------------------
// CoreNode
// ---------------------------------------------------------------------------

export type CoreStatus = 'ONLINE' | 'DEGRADED' | 'OFFLINE';

/** 一台运行 Core 的机器及其连接、能力和健康状态。 */
export interface CoreNode {
  id: string;
  name: string;
  /** Panel 连接 Core 的 host:port。 */
  address: string;
  status: CoreStatus;
  version: string;
  os: string;
  arch: string;
  /** 节点能力标识，如 docker、cpu-topology、mcdr。 */
  capabilities: string[];
  lastSeenAt: number | null;
  instanceCount: number;
  runningCount: number;
  /** 0..1 */
  cpuUsage: number;
  /** 0..1 */
  memoryUsage: number;
  createdAt: number;
}

// ---------------------------------------------------------------------------
// Instance / InstanceRuntime
// ---------------------------------------------------------------------------

export type RuntimeMode = 'HOST' | 'CONTAINER';
export type SupervisorMode = 'DIRECT' | 'MCDR';
export type ServerType = 'VANILLA' | 'PAPER' | 'VELOCITY' | 'FABRIC' | 'CUSTOM';

/** 容器路径映射：宿主机路径 ↔ 容器内路径。 */
export interface MountBinding {
  hostPath: string;
  containerPath: string;
}

/** MCDR 包装器设置（supervisorMode=MCDR 时有效，独立于启动命令编辑）。 */
export interface McdrSettings {
  /** 启动时检查 MCDR 与插件更新。 */
  checkUpdate: boolean;
  /** 配置文件变更后自动重载插件。 */
  autoReload: boolean;
  /** MCDR 语言包，如 zh_cn / en_us。 */
  language: string;
}

/** 一个 Minecraft 服务端实例，归属于且仅归属于一个 Core。 */
export interface Instance {
  id: string;
  coreId: string;
  name: string;
  serverType: ServerType;
  version: string;
  state: InstanceState;
  workDir: string;
  javaRuntimeId: string | null;
  runtimeMode: RuntimeMode;
  supervisorMode: SupervisorMode;
  /** 容器镜像（runtimeMode=CONTAINER 时有效）。 */
  containerImage: string | null;
  /** 容器端口映射，如 ["25565:25565"]。 */
  containerPorts: string[];
  containerEnv: Record<string, string>;
  /** 容器路径映射（宿主机 ↔ 容器）。 */
  containerMounts: MountBinding[];
  mcdrSettings: McdrSettings;
  /** 是否启用自动备份及备份产物落盘目录。 */
  backupEnabled: boolean;
  backupTargetDir: string | null;
  launchCommand: string;
  updateCommand: string | null;
  expiresAt: number | null;
  tags: string[];
  createdAt: number;
}

/** 实例进程的运行时信息：PID、状态、启动时间、退出码、资源使用量。 */
export interface InstanceRuntime {
  instanceId: string;
  pid: number | null;
  state: InstanceState;
  startedAt: number | null;
  exitCode: number | null;
  /** 0..1 */
  cpuUsage: number;
  memoryBytes: number;
  playerCount: number | null;
}

// ---------------------------------------------------------------------------
// ManagedRuntime（受管 Java/Node.js/Python）
// ---------------------------------------------------------------------------

export type ManagedRuntimeKind = 'JAVA' | 'NODE' | 'PYTHON';

export interface ManagedRuntime {
  id: string;
  coreId: string;
  kind: ManagedRuntimeKind;
  version: string;
  majorVersion: number;
  /** 受管工具链目录内路径，不修改系统 PATH。 */
  path: string;
  sha256: string | null;
  installedAt: number;
  sizeBytes: number;
}

// ---------------------------------------------------------------------------
// InstallTemplate
// ---------------------------------------------------------------------------

/** 一键搭建模板：服务端来源、所需环境、默认配置和启动命令的可版本化描述。 */
export interface InstallTemplate {
  id: string;
  name: string;
  serverType: ServerType;
  description: string;
  versions: string[];
  requiredRuntime: { kind: ManagedRuntimeKind; minMajor: number };
  supportsMcdr: boolean;
}

// ---------------------------------------------------------------------------
// 配置识别
// ---------------------------------------------------------------------------

export type ConfigFormat = 'PROPERTIES' | 'YAML' | 'JSON' | 'TOML' | 'RAW';

/** 识别后的配置文件：Schema、revision 与原始格式信息。 */
export interface ConfigDocument {
  id: string;
  instanceId: string;
  path: string;
  format: ConfigFormat;
  /** 无法识别时退回安全的原始文本编辑器。 */
  recognized: boolean;
  revision: number;
  updatedAt: number;
  schemaTitle: string | null;
}

/** 结构化配置表单字段（由 ConfigSchema/UI Schema 展开）。 */
export interface ConfigField {
  key: string;
  title: string;
  description: string | null;
  type: 'string' | 'number' | 'boolean' | 'enum';
  enumValues?: string[];
  value: unknown;
  defaultValue: unknown;
}

// ---------------------------------------------------------------------------
// 文件管理
// ---------------------------------------------------------------------------

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  sizeBytes: number;
  modifiedAt: number;
}

// ---------------------------------------------------------------------------
// 模组/插件
// ---------------------------------------------------------------------------

export type ExtensionSource = 'MODRINTH' | 'HANGAR' | 'SPIGET' | 'CUSTOM';

/** 扩展形态：模组与插件分开展示与管理。 */
export type ExtensionKind = 'MOD' | 'PLUGIN';

export interface ExtensionProject {
  kind: ExtensionKind;
  source: ExtensionSource;
  projectId: string;
  name: string;
  summary: string;
  downloads: number;
  updatedAt: number;
  url: string;
}

/** 本地安装记录：必须保留来源、项目 ID、版本、哈希。 */
export interface ExtensionInstall {
  id: string;
  instanceId: string;
  kind: ExtensionKind;
  source: ExtensionSource;
  projectId: string;
  name: string;
  version: string;
  sha256: string | null;
  fileName: string;
  installedAt: number;
  /** 可更新到的版本号，null 表示已是最新。 */
  updateAvailable: string | null;
}

// ---------------------------------------------------------------------------
// Docker 镜像
// ---------------------------------------------------------------------------

export interface ImageInfo {
  id: string;
  coreId: string;
  repoTags: string[];
  sizeBytes: number;
  createdAt: number;
}

export type ImageBuildStatus = 'RUNNING' | 'SUCCESS' | 'FAILED' | 'CANCELLED';

/** 镜像仓库：拉取时可指定目标仓库，不指定则按 priority 顺序查找。 */
export interface RegistryInfo {
  id: string;
  coreId: string;
  name: string;
  url: string;
  /** 查找顺序，越小越优先。 */
  priority: number;
}

export interface ImageBuild {
  id: string;
  coreId: string;
  tag: string;
  status: ImageBuildStatus;
  startedAt: number;
  finishedAt: number | null;
}

// ---------------------------------------------------------------------------
// CPU 拓扑与调度
// ---------------------------------------------------------------------------

export interface CpuTopology {
  coreId: string;
  logicalCpus: number;
  /** 性能核逻辑 CPU 编号。 */
  performanceCores: number[];
  /** 能效核逻辑 CPU 编号。 */
  efficiencyCores: number[];
  numaNodes: { id: number; cpus: number[] }[];
}

export type CpuPolicyMode = 'AUTO_PERFORMANCE' | 'MANUAL' | 'SHARED';

/**
 * 实例 CPU 亲和策略。
 * status 必须如实返回 requested/applied/degraded，不能把"偏好大核"伪装成硬保证（PLAN 第 9 节）。
 */
export interface CpuPolicy {
  instanceId: string;
  mode: CpuPolicyMode;
  cpuSet: number[];
  exclusive: boolean;
  numaNodeId: number | null;
  status: 'requested' | 'applied' | 'degraded';
}

// ---------------------------------------------------------------------------
// 计划任务
// ---------------------------------------------------------------------------

export type ScheduleEvent = 'INSTANCE_STARTED' | 'INSTANCE_STOPPED' | 'PLAYER_COUNT_ABOVE' | 'EXIT_CODE_NONZERO';

export type ScheduleTrigger =
  | { kind: 'CRON'; cron: string; timezone: string }
  | { kind: 'EVENT'; event: ScheduleEvent };

export interface Schedule {
  id: string;
  instanceId: string;
  name: string;
  enabled: boolean;
  trigger: ScheduleTrigger;
  /** 动作描述，如 command:say hello / start / stop / restart / backup。 */
  action: string;
  lastRunAt: number | null;
  nextRunAt: number | null;
}

export type ExecutionResult = 'SUCCESS' | 'FAILED' | 'SKIPPED';

/** 计划任务每次触发都有去重键和执行记录。 */
export interface ScheduleExecution {
  id: string;
  scheduleId: string;
  startedAt: number;
  finishedAt: number | null;
  result: ExecutionResult;
  message: string | null;
}

// ---------------------------------------------------------------------------
// Task（异步操作）
// ---------------------------------------------------------------------------

export type TaskKind =
  | 'CORE_REMOVE'
  | 'INSTANCE_CREATE'
  | 'INSTANCE_INSTALL'
  | 'INSTANCE_DELETE'
  | 'INSTANCE_START'
  | 'INSTANCE_STOP'
  | 'INSTANCE_KILL'
  | 'INSTANCE_RESTART'
  | 'RUNTIME_INSTALL'
  | 'RUNTIME_REMOVE'
  | 'IMAGE_PULL'
  | 'IMAGE_BUILD'
  | 'IMAGE_REMOVE'
  | 'EXTENSION_INSTALL'
  | 'EXTENSION_REMOVE'
  | 'BACKUP'
  | 'RESTORE';

export type TaskStatus = 'PENDING' | 'RUNNING' | 'SUCCESS' | 'FAILED' | 'CANCELLED';

/** 耗时或不可立即完成的操作返回 202 与 taskId（PLAN 第 4.2 节）。 */
export interface Task {
  id: string;
  kind: TaskKind;
  title: string;
  status: TaskStatus;
  /** 0..1；null 表示进度不确定。 */
  progress: number | null;
  message: string | null;
  coreId: string | null;
  instanceId: string | null;
  createdAt: number;
  finishedAt: number | null;
  cancellable: boolean;
}

// ---------------------------------------------------------------------------
// 用户 / 权限 / 会话 / 审计
// ---------------------------------------------------------------------------

export interface User {
  id: string;
  username: string;
  displayName: string;
  groupIds: string[];
  disabled: boolean;
  lastLoginAt: number | null;
  createdAt: number;
}

/**
 * 用户组授权范围：可列出 Core、实例或标签选择器（PLAN 第 4.4 节）。
 * 空数组表示不限制该维度；三组全空 = 全部实例可见（仅管理员组）。
 */
export interface InstanceScope {
  coreIds: string[];
  instanceIds: string[];
  tagSelectors: string[];
}

export interface Group {
  id: string;
  name: string;
  description: string;
  permissions: Permission[];
  instanceScopes: InstanceScope;
  builtIn: boolean;
}

export type SessionPlatform = 'browser' | 'tauri-desktop' | 'tauri-mobile' | 'unknown';

/** 浏览器或原生客户端登录会话。 */
export interface DeviceSession {
  id: string;
  userId: string;
  deviceName: string;
  platform: SessionPlatform;
  ip: string;
  createdAt: number;
  lastActiveAt: number;
  current: boolean;
}

export type AuditResult = 'SUCCESS' | 'DENIED' | 'FAILED';

/** 不可变的安全与运维操作记录；密码/Token/PSK 等已脱敏。 */
export interface AuditEvent {
  id: string;
  actorId: string;
  actorName: string;
  action: string;
  target: string;
  result: AuditResult;
  requestId: string;
  sourceIp: string;
  createdAt: number;
  detail: string | null;
}

// ---------------------------------------------------------------------------
// 鉴权
// ---------------------------------------------------------------------------

/** 登录令牌（结构对应 platform 的 AuthTokenBundle，本包不依赖 platform）。 */
export interface SessionToken {
  accessToken: string;
  refreshToken: string;
  expiresAt: number;
  deviceId: string;
}

export interface LoginResult {
  user: User;
  /** 聚合用户所在全部用户组后的权限点集合（服务端计算）。 */
  permissions: Permission[];
  token: SessionToken;
}

// ---------------------------------------------------------------------------
// Panel 设置
// ---------------------------------------------------------------------------

/** Panel 级默认设置：新建实例/启用备份时用于推导默认路径。 */
export interface PanelSettings {
  /** 默认实例根目录，如 /opt/servers/；新建实例工作目录推导为 {root}{name}/。 */
  defaultInstanceRoot: string;
  /** 默认备份根目录，如 /fs/backups/；启用备份时推导为 {root}{name}/。 */
  defaultBackupRoot: string;
}

// ---------------------------------------------------------------------------
// 权限点（PLAN.md 第 4.4 节）
// ---------------------------------------------------------------------------

export type Permission =
  | 'core.read'
  | 'core.manage'
  | 'environment.read'
  | 'environment.manage'
  | 'instance.read'
  | 'instance.create'
  | 'instance.delete'
  | 'instance.settings.basic'
  | 'instance.settings.launch'
  | 'instance.settings.path'
  | 'instance.settings.container'
  | 'instance.settings.cpu'
  | 'instance.control'
  | 'instance.console.read'
  | 'instance.console.write'
  | 'file.read'
  | 'file.write'
  | 'config.read'
  | 'config.write'
  | 'extension.read'
  | 'extension.manage'
  | 'image.read'
  | 'image.manage'
  | 'image.build'
  | 'schedule.read'
  | 'schedule.manage'
  | 'user.read'
  | 'user.manage'
  | 'audit.read';

/** 全部权限点清单（用于权限矩阵编辑器）。 */
export const ALL_PERMISSIONS: readonly Permission[] = [
  'core.read',
  'core.manage',
  'environment.read',
  'environment.manage',
  'instance.read',
  'instance.create',
  'instance.delete',
  'instance.settings.basic',
  'instance.settings.launch',
  'instance.settings.path',
  'instance.settings.container',
  'instance.settings.cpu',
  'instance.control',
  'instance.console.read',
  'instance.console.write',
  'file.read',
  'file.write',
  'config.read',
  'config.write',
  'extension.read',
  'extension.manage',
  'image.read',
  'image.manage',
  'image.build',
  'schedule.read',
  'schedule.manage',
  'user.read',
  'user.manage',
  'audit.read',
];
