import type { ApiClientOptions } from './api-client-options';
import { createApiUrl } from './create-api-url';

type HttpMethod = 'DELETE' | 'GET' | 'PATCH' | 'POST' | 'PUT';

interface RequestOptions {
  method?: HttpMethod;
  body?: unknown;
  csrf?: boolean;
  headers?: Record<string, string>;
  ifMatch?: number | string;
  idempotent?: boolean;
  accept?: string;
  responseType?: 'arrayBuffer' | 'json';
}

interface ErrorBody {
  error?: {
    code?: string;
    message?: string;
    requestId?: string;
  };
}

export type ClientType = 'BROWSER' | 'NATIVE';
export type CoreStatus = 'ONLINE' | 'DEGRADED' | 'OFFLINE' | 'INCOMPATIBLE' | 'AUTH_FAILED' | 'UNKNOWN';
export type InstanceKind =
  | 'VANILLA'
  | 'PAPER'
  | 'VELOCITY'
  | 'FABRIC'
  | 'NEO_FORGE'
  | 'FORGE'
  | 'BUKKIT'
  | 'SPIGOT'
  | 'PURPUR'
  | 'PUFFERFISH'
  | 'FOLIA'
  | 'LEAF'
  | 'MOHIST'
  | 'MAGMA'
  | 'SPONGE'
  | 'ARCLIGHT'
  | 'YOUER'
  | 'ASYNC_YOUER'
  | 'SILKARD'
  | 'CAT_SERVER'
  | 'LINGSHU'
  | 'WATERFALL'
  | 'BUNGEE_CORD'
  | 'LIGHTFALL'
  | 'GEYSER'
  | 'BEDROCK_DEDICATED_SERVER'
  | 'POCKET_MINE_MP'
  | 'NUKKIT'
  | 'CLOUDBURST_NUKKIT'
  | 'CUSTOM'
  | 'UNKNOWN';
export type InstanceState = 'CREATED' | 'STARTING' | 'RUNNING' | 'STOPPING' | 'STOPPED' | 'FAILED' | 'UNKNOWN';
export type LogStream = 'stdout' | 'stderr' | 'system';
export type RuntimeKind = 'JAVA' | 'NODE_JS' | 'PYTHON';
export type RuntimeMode = 'HOST' | 'CONTAINER';
export type SupervisorMode = 'DIRECT' | 'MCDR';
export type RuntimeArchiveFormat = 'TAR_GZ' | 'ZIP';
export type DownloadPlatform = 'LINUX' | 'MACOS' | 'WINDOWS';
export type DownloadArchitecture = 'AARCH64' | 'X86_64';
export type BedrockManagementKind = 'DEDICATED_SERVER' | 'POCKET_MINE' | 'NUKKIT' | 'GEYSER';
export type BedrockTransport = 'RAKNET_UDP';
export type BedrockConfigurationFormat = 'PROPERTIES' | 'YAML' | 'UNKNOWN';
export type BedrockExtensionCompatibilityPolicy = 'UNSUPPORTED' | 'PLUGIN_MANIFEST';
export type BedrockPortCheckState = 'AVAILABLE' | 'IN_USE' | 'UNAVAILABLE';
export type BedrockPortSource = 'CONFIGURED' | 'DEFAULT';
export type BedrockBindAddressSource = 'CONFIGURED' | 'DEFAULT';
export type InstallRuntimeRequirement = 'JAVA' | 'NODE_JS' | 'PYTHON' | 'PHP' | 'NATIVE';
export type InstallTemplateFamily = 'JAVA_SERVER' | 'JAVA_PROXY' | 'BEDROCK_SERVER' | 'BEDROCK_PROXY';
export type ProxyTopology = 'NONE' | 'ONE_TO_MANY' | 'ONE_TO_ONE';
export type ExtensionKind = 'PLUGIN' | 'MOD';
export type ExtensionCompatibility = 'COMPATIBLE' | 'INCOMPATIBLE' | 'UNKNOWN';
export interface InstallTemplateExtensionLayout {
  kind: ExtensionKind;
  directories: string[];
}
export type RuntimeSource = 'MANAGED' | 'SYSTEM';
export type RuntimeValidation = 'VALID' | 'INVALID';
export type RuntimeTaskState = 'RUNNING' | 'SUCCEEDED' | 'FAILED';
export type InstallTemplateVersionKind = 'GAME' | 'LOADER' | 'SERVER';

export interface User {
  id: string;
  username: string;
  displayName: string;
  permissions: string[];
  resourceScopes?: string[];
}

export interface SessionTokens {
  id: string;
  accessToken: string | null;
  accessExpiresAt: string;
  refreshToken: string | null;
  refreshExpiresAt: string | null;
  csrfToken: string | null;
}

export interface LoginResponse {
  user: User;
  session: SessionTokens;
}

export interface Core {
  id: string;
  name: string;
  address: string;
  status: CoreStatus;
  latencyMs: number | null;
  lastSeenAt: string | null;
  version: string | null;
  protocolVersion: string | null;
  capabilities: string[];
  secretConfigured: boolean;
  secretUpdatedAt: string | null;
  skipCertificateVerification: boolean;
  certificateVerified: boolean | null;
  tlsCertificateSha256: string | null;
  tags: string[];
  revision: number;
}

export interface CorePage {
  items: Core[];
  nextCursor: string | null;
}

/** 操作系统报告的逻辑 CPU 性能类别。 */
export type CpuPerformanceClass = 'PERFORMANCE' | 'EFFICIENCY' | 'UNKNOWN';

/** Core 拓扑探测的来源和可信度。 */
export interface CpuTopologyDetection {
  source: string;
  confidence: string;
}

/** 单个逻辑 CPU 的拓扑和可调度状态。 */
export interface CpuLogicalProcessor {
  id: number;
  physicalCoreId: string | null;
  performanceClass: CpuPerformanceClass;
  online: boolean;
  isolated: boolean | null;
  numaNode: number | null;
}

/** 当前已确认可用于性能核或能效核策略的 CPU 集合。 */
export interface CpuAvailability {
  performanceCpuIds: number[];
  efficiencyCpuIds: number[];
}

/** Core 宿主机 CPU 拓扑只读快照。 */
export interface CpuTopology {
  architecture: string;
  logicalCpus: CpuLogicalProcessor[];
  physicalCoreCount: number | null;
  available: CpuAvailability;
  detection: CpuTopologyDetection;
}

export type CpuPolicyMode = 'AUTO' | 'PERFORMANCE' | 'EFFICIENCY' | 'CUSTOM';
export type CpuShareMode = 'SHARED' | 'EXCLUSIVE';

/** 实例请求的 CPU 选择和共享策略。 */
export interface CpuPolicy {
  mode: CpuPolicyMode;
  requestedCpuIds: number[];
  minCpus: number;
  maxCpus: number | null;
  preferPhysicalCores: boolean;
  numaNode: number | null;
  shareMode: CpuShareMode;
  strict: boolean;
}

/** Core 对 CPU policy 的只读候选解析结果，不代表 affinity 已应用。 */
export interface CpuPolicyResolution {
  requested: CpuPolicy;
  candidateCpuIds: number[];
  selectedCpuIds: number[];
  performanceClass: CpuPerformanceClass;
  conflicts: string[];
  degradedReason: string | null;
  reservationId: string | null;
}

/** Core 当前登记的 CPU 独占预留记录。登记不代表宿主机 affinity 已应用。 */
export interface CpuReservation {
  reservationId: string;
  instanceId: string;
  cpuIds: number[];
  createdAt: string;
}

/** Core CPU 独占预留列表。当前列表不跨 Core 重启持久化。 */
export interface CpuReservationPage {
  items: CpuReservation[];
}

/** 登记 CPU 独占预留所需的实例修订号和 policy。 */
export interface CpuReservationRequest {
  instanceId: string;
  revision: number;
  policy: CpuPolicy;
}

/** CPU 独占预留及 Core 实际选中的 policy。 */
export interface CpuReservationResult {
  reservation: CpuReservation;
  appliedPolicy: CpuPolicyResolution;
}

export interface ManagedRuntime {
  runtimeId: string | null;
  kind: RuntimeKind;
  source: RuntimeSource;
  distribution: string | null;
  executable: string;
  version: string | null;
  validation: RuntimeValidation;
}

export interface ManagedRuntimePage {
  items: ManagedRuntime[];
}

export interface DownloadManifest {
  url: string;
  sizeBytes: number;
  sha256: string;
  platform: DownloadPlatform;
  architecture: DownloadArchitecture;
}

export interface RuntimeInstallManifest {
  runtimeId: string;
  kind: RuntimeKind;
  distribution: string;
  version: string;
  archive: DownloadManifest;
  archiveFormat: RuntimeArchiveFormat;
  executablePath: string;
}

export interface RuntimeOperation {
  taskId: string;
  kind?: string;
  state?: RuntimeTaskState;
  progress?: number | null;
  runtime?: ManagedRuntime;
  error?: string;
}

export interface ProvisionPlan {
  templateId: string;
  minecraftVersion: string;
  build: string;
  instanceId: string;
  instanceName: string;
  instanceKind: InstanceKind;
  instanceDirectory: string;
  updateCommand?: string | null;
  expiresAt?: string | null;
  requiredRuntime: InstallRuntimeRequirement;
  runtimeId?: string | null;
  archive: DownloadManifest;
  archiveFormat: RuntimeArchiveFormat;
  executablePath: string;
  launchArguments?: string[];
  stopCommand: string;
  stopTimeoutSeconds: number;
}

export interface ProvisionResolution {
  resolvedPlan: ProvisionPlan;
  planHash: string;
}

export interface ProvisionOperation {
  taskId: string;
  instanceId?: string;
  instance?: Instance;
  kind?: string;
  state?: RuntimeTaskState;
  progress?: number | null;
  error?: string;
}

export interface BedrockManagementProfile {
  managementKind: BedrockManagementKind;
  transport: BedrockTransport;
  defaultBindAddress: string;
  defaultPort: number;
  configurationFiles: string[];
  configurationFormat: BedrockConfigurationFormat;
  extensionKind: ExtensionKind | null;
  extensionDirectories: string[];
  extensionCompatibilityPolicy: BedrockExtensionCompatibilityPolicy;
}

export interface BedrockPortCheck {
  instanceId: string;
  managementKind: BedrockManagementKind;
  transport: BedrockTransport;
  bindAddress: string;
  bindAddressSource: BedrockBindAddressSource;
  port: number;
  portSource: BedrockPortSource;
  state: BedrockPortCheckState;
  available: boolean;
  checkedAt: string;
  error: string | null;
}

export type ConfigFormat = 'PROPERTIES' | 'YAML' | 'JSON' | 'TOML' | 'PROVIDER_SPECIFIC';

export interface ConfigDocumentSummary {
  documentId: string;
  path: string;
  format: ConfigFormat;
  revision: string;
  contentHash: string;
  lossy: boolean;
}

export interface ConfigDocumentPage {
  documents: ConfigDocumentSummary[];
}

export interface ConfigDocument {
  documentId: string;
  path: string;
  format: ConfigFormat;
  schema: Record<string, unknown>;
  uiSchema: Record<string, unknown>;
  values: Record<string, unknown>;
  revision: string;
  contentHash: string;
  unmapped: string[];
  lossy: boolean;
}

/** Core 对实例配置关系执行一次校验后的诊断严重级别。 */
export type ConfigValidationSeverity = 'ERROR' | 'WARNING';

/** 一条可定位到主文件和关联文件的配置诊断。 */
export interface ConfigValidationIssue {
  code: string;
  severity: ConfigValidationSeverity;
  path: string;
  field: string | null;
  message: string;
  relatedPath: string | null;
  relatedField: string | null;
}

/** 实例配置校验的完整结果。 */
export interface ConfigValidationResult {
  valid: boolean;
  checkedDocuments: string[];
  issues: ConfigValidationIssue[];
}

export interface RawConfigResult {
  data: ArrayBuffer;
  etag: string;
  sha256: string;
}

export type FileKind = 'FILE' | 'DIRECTORY' | 'SYMLINK' | 'OTHER';
export type FileBytes = ArrayBuffer | Blob | Uint8Array;

export interface FileEntry {
  name: string;
  path: string;
  kind: FileKind;
  size: number;
  modifiedAt: string;
  sha256: string | null;
}

export interface FilePage {
  items: FileEntry[];
  nextCursor: string | null;
}

export interface ExtensionDirectoryPage {
  path: string;
  page: FilePage;
}

export interface ExtensionInstall {
  id: string;
  kind: ExtensionKind;
  path: string;
  sha256: string;
  source: string;
  projectId: string | null;
  version: string | null;
  installedAt: string;
}

export interface ExtensionProject {
  projectId: string;
  source: string;
  kind: ExtensionKind;
  name: string;
  summary: string;
  projectUrl: string;
  iconUrl: string | null;
  downloads: number;
  supportedMinecraftVersions: string[];
  supportedLoaders: string[];
  compatibility: ExtensionCompatibility;
}

export interface ExtensionSearchResult {
  source: string;
  items: ExtensionProject[];
  total: number;
  limit: number;
  offset: number;
}

export interface ExtensionDependency {
  projectId: string | null;
  versionId: string | null;
  fileName: string | null;
  dependencyType: string;
}

export interface ExtensionArtifact {
  fileName: string;
  downloadUrl: string;
  size: number;
  sha1: string | null;
  sha512: string;
  primary: boolean;
}

export interface ExtensionVersion {
  id: string;
  projectId: string;
  name: string;
  versionNumber: string;
  gameVersions: string[];
  loaders: string[];
  dependencies: ExtensionDependency[];
  artifacts: ExtensionArtifact[];
  downloads: number;
  compatibility: ExtensionCompatibility;
}

export interface ExtensionVersionResult {
  source: string;
  projectId: string;
  items: ExtensionVersion[];
}

export interface ExtensionPlanRequest {
  templateId: string;
  kind: ExtensionKind;
  projectId: string;
  versionId: string;
  minecraftVersion: string;
  loader?: string;
}

export interface ExtensionPlanItem {
  source: string;
  projectId: string;
  versionId: string;
  versionNumber: string;
  artifact: ExtensionArtifact;
  dependencies: ExtensionDependency[];
}

export interface ExtensionPlanResolution {
  templateId: string;
  kind: ExtensionKind;
  minecraftVersion: string;
  loader: string | null;
  items: ExtensionPlanItem[];
}

export interface ExtensionInstallRequest extends ExtensionPlanRequest {
  directory?: string;
  /** 目标实例接受的精确 Bedrock 插件 API 版本列表；缺省时只校验 manifest 结构。 */
  bedrockApiVersions?: string[];
}

export type ExtensionInstallTaskState = 'RUNNING' | 'SUCCEEDED' | 'FAILED';
export type ExtensionRollbackState = 'NOT_STARTED' | 'NOT_NEEDED' | 'SUCCEEDED' | 'PARTIAL';

export interface ExtensionInstallTask {
  taskId: string;
  coreId: string;
  instanceId: string;
  kind: 'EXTENSION_INSTALL' | 'EXTENSION_UPDATE';
  extensionKind: ExtensionKind;
  state: ExtensionInstallTaskState;
  rollbackState: ExtensionRollbackState;
  progress: {
    completed: number;
    total: number;
  };
  installations: ExtensionInstall[];
  acceptedAt: string;
  error?: string;
}

export interface InstanceExtensionScan {
  templateId: string;
  kind: ExtensionKind;
  directories: ExtensionDirectoryPage[];
  installations: ExtensionInstall[];
}

export interface FileReadResult {
  data: ArrayBuffer;
  etag: string;
  sha256: string;
  eof: boolean;
}

export interface FileDownloadStart {
  transferId: string;
  chunkSize: number;
  nextOffset: number;
  sizeBytes: number;
  sha256: string;
}

export interface FileDownloadChunk {
  data: ArrayBuffer;
  offset: number;
  nextOffset: number;
  sizeBytes: number;
  sha256: string;
  etag: string;
  eof: boolean;
}

export interface FileUploadStart {
  transferId: string;
  chunkSize: number;
  nextOffset: number;
  sizeBytes: number;
}

export interface FileUploadPart {
  transferId: string;
  nextOffset: number;
  sizeBytes: number;
}

export type FileBatchOperation =
  | { kind: 'MKDIR'; path: string; recursive?: boolean }
  | { kind: 'MOVE'; from: string; to: string; overwrite?: boolean }
  | {
      kind: 'WRITE';
      path: string;
      dataBase64: string;
      expectedSha256?: string;
    }
  | {
      kind: 'DELETE';
      path: string;
      recursive?: boolean;
      confirmation: 'DELETE';
    };

export type FileTaskState = 'RUNNING' | 'SUCCEEDED' | 'FAILED';
export interface FileBatchTaskProgress {
  completed: number;
  total: number;
}

export interface FileBatchTaskResult {
  index: number;
  state: 'SUCCEEDED' | 'FAILED';
  result?: {
    entry?: FileEntry;
    path?: string;
    deleted?: boolean;
  };
  error?: string;
}

export interface FileTask {
  taskId: string;
  kind: 'FILE_DELETE' | 'FILE_BATCH' | 'FILE_ARCHIVE_CREATE';
  state: FileTaskState;
  progress: number | null | FileBatchTaskProgress;
  path?: string;
  deleted?: boolean;
  failedIndex?: number;
  results?: FileBatchTaskResult[];
  archive?: FileEntry;
  error?: string;
}

export interface VersionMetadataProvider {
  id: string;
  name: string;
  url: string;
}

export interface InstallTemplate {
  id: string;
  name: string;
  instanceKind: InstanceKind;
  family: InstallTemplateFamily;
  requiredRuntime: InstallRuntimeRequirement;
  proxyTopology: ProxyTopology;
  extensionLayouts: InstallTemplateExtensionLayout[];
  metadataProviders: VersionMetadataProvider[];
}

export interface InstallTemplatePage {
  items: InstallTemplate[];
}

export interface InstallTemplateVersion {
  id: string;
  providerId: string;
  kind: InstallTemplateVersionKind;
  stable: boolean;
  metadataUrl: string | null;
}

export interface InstallTemplateVersionPage {
  items: InstallTemplateVersion[];
}

export interface ProxySubserver {
  id: string;
  name: string;
  targetInstanceId: string;
  host: string;
  port: number;
  enabled: boolean;
}

export interface ProxySubserverPage {
  items: ProxySubserver[];
}

export type ProxySubserverHealthStatus = 'DISABLED' | 'REACHABLE' | 'UNREACHABLE';
export type ProxySubserverProtocolStatus =
  | 'DISABLED'
  | 'UNAVAILABLE'
  | 'INVALID_RESPONSE'
  | 'RESPONDED';

export interface ProxySubserverHealth {
  subserverId: string;
  targetInstanceId: string;
  host: string;
  port: number;
  enabled: boolean;
  status: ProxySubserverHealthStatus;
  protocolStatus: ProxySubserverProtocolStatus;
  reachable: boolean | null;
  latencyMs: number | null;
  checkedAt: string;
  error: string | null;
}

export interface LaunchConfig {
  executable: string;
  args: string[];
  environment: Record<string, string>;
  stopCommand: string;
  stopTimeoutSeconds: number;
  runtimeMode?: RuntimeMode;
  supervisorMode?: SupervisorMode;
  mcdr?: McdrConfig | null;
}

export interface McdrConfig {
  executable: string;
  args: string[];
}

export interface InstanceRuntime {
  state: InstanceState;
  pid: number | null;
  startedAt: string | null;
  exitCode: number | null;
  players?: {
    online?: number;
    max?: number;
  };
  eventCursor?: string | null;
}

export interface Instance {
  id: string;
  coreId: string;
  name: string;
  kind: InstanceKind;
  directory: string;
  updateCommand: string | null;
  expiresAt: string | null;
  launch: LaunchConfig;
  runtime: InstanceRuntime;
  revision: number;
}

export interface InstanceUpdate {
  name?: string;
  kind?: InstanceKind;
  directory?: string;
  launch?: LaunchConfig;
  updateCommand?: string | null;
  expiresAt?: string | null;
}

export interface InstancePage {
  items: Instance[];
  nextCursor: string | null;
}

/** 实例生命周期审计动作。 */
export type InstanceAuditAction = 'START' | 'STOP' | 'KILL' | 'PROCESS_EXIT';

/** 实例生命周期审计结果。 */
export type InstanceAuditOutcome = 'ACCEPTED' | 'SUCCEEDED' | 'FAILED' | 'DEGRADED';

/** Core 记录的一条实例生命周期审计事实。 */
export interface InstanceAuditRecord {
  auditId: string;
  instanceId: string;
  taskId: string | null;
  action: InstanceAuditAction;
  outcome: InstanceAuditOutcome;
  runtimeMode: RuntimeMode;
  supervisorMode: SupervisorMode;
  reason: string | null;
  occurredAt: string;
}

/** 实例生命周期审计查询结果。当前按最新记录优先返回。 */
export interface InstanceAuditPage {
  items: InstanceAuditRecord[];
  nextCursor: string | null;
}

/** Panel 用户级 HTTP 审计的授权判定。 */
export type PanelAuditPermissionResult = 'ALLOWED' | 'DENIED' | 'NOT_REQUIRED';

/** Panel 持久化的一条请求审计事件；不包含请求体或凭据。 */
export interface PanelAuditEvent {
  id: string;
  occurredAt: string;
  userId: string | null;
  requestId: string;
  sourceIp: string | null;
  method: string;
  path: string;
  statusCode: number;
  permissionResult: PanelAuditPermissionResult;
}

/** Panel 用户级审计查询结果，按事件时间倒序返回。 */
export interface PanelAuditPage {
  items: PanelAuditEvent[];
}

export interface LogLine {
  cursor: string;
  occurredAt: string;
  stream: LogStream;
  line: string;
}

export interface LogPage {
  items: LogLine[];
  nextCursor: string | null;
  eventCursor: string;
}

export interface TaskAccepted {
  taskId: string;
  acceptedAt: string;
}

export interface CommandAccepted {
  acceptedAt: string;
}

export interface PanelApiClient {
  login(username: string, password: string, clientType: ClientType): Promise<LoginResponse>;
  getCurrentUser(): Promise<User>;
  logout(): Promise<void>;
  listCores(): Promise<CorePage>;
  listAuditEvents(limit?: number): Promise<PanelAuditPage>;
  getCpuTopology(coreId: string): Promise<CpuTopology>;
  resolveCpuPolicy(coreId: string, policy: CpuPolicy): Promise<CpuPolicyResolution>;
  listCpuReservations(coreId: string): Promise<CpuReservationPage>;
  reserveCpu(coreId: string, request: CpuReservationRequest): Promise<CpuReservationResult>;
  releaseCpu(coreId: string, reservationId: string): Promise<void>;
  listInstallTemplates(): Promise<InstallTemplatePage>;
  listInstallTemplateVersions(templateId: string): Promise<InstallTemplateVersionPage>;
  searchExtensionCatalog(
    query: string,
    type: ExtensionKind,
    source?: string,
    minecraftVersion?: string,
    loader?: string,
    limit?: number,
    offset?: number,
  ): Promise<ExtensionSearchResult>;
  getExtensionProjectVersions(
    source: string,
    projectId: string,
    minecraftVersion?: string,
    loader?: string,
  ): Promise<ExtensionVersionResult>;
  resolveExtensionPlan(
    coreId: string,
    instanceId: string,
    plan: ExtensionPlanRequest,
  ): Promise<ExtensionPlanResolution>;
  installExtensions(
    coreId: string,
    instanceId: string,
    request: ExtensionInstallRequest,
  ): Promise<TaskAccepted>;
  updateExtension(
    coreId: string,
    instanceId: string,
    extensionId: string,
    request: ExtensionInstallRequest,
    expectedSha256?: string,
  ): Promise<TaskAccepted>;
  getExtensionInstallTask(coreId: string, taskId: string): Promise<ExtensionInstallTask>;
  listManagedRuntimes(coreId: string): Promise<ManagedRuntimePage>;
  resolveProvisionPlan(coreId: string, plan: ProvisionPlan): Promise<ProvisionResolution>;
  executeProvision(coreId: string, plan: ProvisionPlan, planHash: string): Promise<ProvisionOperation>;
  getProvisionTask(coreId: string, taskId: string): Promise<ProvisionOperation>;
  installRuntime(
    coreId: string,
    manifest: RuntimeInstallManifest,
    setAsDefault?: boolean,
  ): Promise<RuntimeOperation>;
  getRuntimeInstallation(coreId: string, taskId: string): Promise<RuntimeOperation>;
  verifyRuntime(coreId: string, runtimeId: string): Promise<RuntimeOperation>;
  deleteRuntime(coreId: string, runtimeId: string): Promise<TaskAccepted>;
  getBedrockProfile(coreId: string, instanceId: string): Promise<BedrockManagementProfile>;
  checkBedrockPort(coreId: string, instanceId: string): Promise<BedrockPortCheck>;
  listInstanceExtensions(
    coreId: string,
    instanceId: string,
    templateId: string,
    kind: ExtensionKind,
  ): Promise<InstanceExtensionScan>;
  writeInstanceExtension(
    coreId: string,
    instanceId: string,
    templateId: string,
    kind: ExtensionKind,
    path: string,
    content: FileBytes,
    expectedSha256?: string,
  ): Promise<FileEntry>;
  deleteInstanceExtension(
    coreId: string,
    instanceId: string,
    templateId: string,
    kind: ExtensionKind,
    path: string,
  ): Promise<TaskAccepted>;
  listConfigDocuments(coreId: string, instanceId: string): Promise<ConfigDocumentPage>;
  scanConfigDocuments(coreId: string, instanceId: string): Promise<ConfigDocumentPage>;
  validateConfigDocuments(coreId: string, instanceId: string): Promise<ConfigValidationResult>;
  getConfigDocument(coreId: string, instanceId: string, documentId: string): Promise<ConfigDocument>;
  patchConfigDocument(
    coreId: string,
    instanceId: string,
    documentId: string,
    revision: string,
    patch: Record<string, unknown>,
    allowLossy?: boolean,
  ): Promise<ConfigDocument>;
  readRawConfig(
    coreId: string,
    instanceId: string,
    documentId: string,
  ): Promise<RawConfigResult>;
  writeRawConfig(
    coreId: string,
    instanceId: string,
    documentId: string,
    content: FileBytes,
    expectedSha256?: string,
  ): Promise<ConfigDocument>;
  listInstanceFiles(
    coreId: string,
    instanceId: string,
    path?: string,
    cursor?: string,
    limit?: number,
  ): Promise<FilePage>;
  readInstanceFile(
    coreId: string,
    instanceId: string,
    path: string,
    offset?: number,
    length?: number,
  ): Promise<FileReadResult>;
  writeInstanceFile(
    coreId: string,
    instanceId: string,
    path: string,
    content: FileBytes,
    expectedSha256?: string,
  ): Promise<FileEntry>;
  createInstanceDirectory(
    coreId: string,
    instanceId: string,
    path: string,
    recursive?: boolean,
  ): Promise<FileEntry>;
  moveInstanceFile(
    coreId: string,
    instanceId: string,
    from: string,
    to: string,
    overwrite?: boolean,
  ): Promise<FileEntry>;
  beginFileUpload(
    coreId: string,
    instanceId: string,
    path: string,
    sizeBytes: number,
    sha256: string,
    expectedSha256?: string,
  ): Promise<FileUploadStart>;
  beginFileDownload(
    coreId: string,
    instanceId: string,
    path: string,
  ): Promise<FileDownloadStart>;
  uploadFilePart(
    coreId: string,
    transferId: string,
    partNumber: number,
    content: FileBytes,
    sha256: string,
  ): Promise<FileUploadPart>;
  downloadFilePart(
    coreId: string,
    transferId: string,
    partNumber: number,
  ): Promise<FileDownloadChunk>;
  completeFileUpload(coreId: string, transferId: string): Promise<FileEntry>;
  abortFileUpload(coreId: string, transferId: string): Promise<void>;
  completeFileDownload(coreId: string, transferId: string): Promise<void>;
  abortFileDownload(coreId: string, transferId: string): Promise<void>;
  batchInstanceFiles(
    coreId: string,
    instanceId: string,
    operations: FileBatchOperation[],
  ): Promise<TaskAccepted>;
  createFileArchive(
    coreId: string,
    instanceId: string,
    paths: string[],
    outputPath: string,
  ): Promise<TaskAccepted>;
  deleteInstanceFile(
    coreId: string,
    instanceId: string,
    path: string,
    recursive?: boolean,
  ): Promise<TaskAccepted>;
  getFileTask(coreId: string, taskId: string): Promise<FileTask>;
  listProxySubservers(coreId: string, proxyInstanceId: string): Promise<ProxySubserverPage>;
  upsertProxySubserver(
    coreId: string,
    proxyInstanceId: string,
    subserver: ProxySubserver,
  ): Promise<ProxySubserver>;
  deleteProxySubserver(coreId: string, proxyInstanceId: string, subserverId: string): Promise<void>;
  checkProxySubserver(
    coreId: string,
    proxyInstanceId: string,
    subserverId: string,
  ): Promise<ProxySubserverHealth>;
  listInstances(coreId: string): Promise<InstancePage>;
  updateInstance(coreId: string, instanceId: string, update: InstanceUpdate, revision: number): Promise<Instance>;
  listInstanceAudit(coreId: string, instanceId: string, limit?: number): Promise<InstanceAuditPage>;
  getInstanceLogs(coreId: string, instanceId: string): Promise<LogPage>;
  startInstance(coreId: string, instanceId: string): Promise<TaskAccepted>;
  stopInstance(coreId: string, instanceId: string): Promise<TaskAccepted>;
  killInstance(coreId: string, instanceId: string): Promise<TaskAccepted>;
  resetInstance(coreId: string, instanceId: string): Promise<Instance>;
  sendInstanceCommand(coreId: string, instanceId: string, command: string): Promise<CommandAccepted>;
}

export class ApiRequestError extends Error {
  readonly status: number;
  readonly code: string;
  readonly requestId?: string;

  constructor(status: number, code: string, message: string, requestId?: string) {
    super(message);
    this.name = 'ApiRequestError';
    this.status = status;
    this.code = code;
    if (requestId !== undefined) {
      this.requestId = requestId;
    }
  }
}

export function createPanelApiClient(options: ApiClientOptions): PanelApiClient {
  async function request<T>(path: string, requestOptions: RequestOptions = {}): Promise<T> {
    const method = requestOptions.method ?? 'GET';
    const headers = new Headers({ Accept: requestOptions.accept ?? 'application/json' });
    const accessToken = options.getAccessToken?.();
    const csrfToken = requestOptions.csrf ? options.getCsrfToken?.() : undefined;
    if (accessToken) {
      headers.set('Authorization', `Bearer ${accessToken}`);
    }
    if (csrfToken) {
      headers.set('X-CSRF-Token', csrfToken);
    }
    if (requestOptions.headers) {
      for (const [name, value] of Object.entries(requestOptions.headers)) {
        headers.set(name, value);
      }
    }
    if (requestOptions.idempotent) {
      headers.set('Idempotency-Key', createRequestId());
    }
    if (requestOptions.ifMatch !== undefined) {
      const value = String(requestOptions.ifMatch);
      headers.set('If-Match', value.startsWith('"') ? value : `"${value}"`);
    }
    if (requestOptions.body !== undefined) {
      headers.set(
        'Content-Type',
        isBinaryBody(requestOptions.body) ? 'application/octet-stream' : 'application/json',
      );
    }

    const init: RequestInit = {
      method,
      headers,
      credentials: 'same-origin',
    };
    if (requestOptions.body !== undefined) {
      init.body = isBinaryBody(requestOptions.body)
        ? (requestOptions.body as BodyInit)
        : JSON.stringify(requestOptions.body);
    }

    const response = await fetch(createApiUrl(options.baseUrl, path), init);
    if (!response.ok) {
      throw await createRequestError(response);
    }
    if (response.status === 204) {
      return undefined as T;
    }

    if (requestOptions.responseType === 'arrayBuffer') {
      return {
        data: await response.arrayBuffer(),
        etag: response.headers.get('ETag'),
        contentSha256: response.headers.get('Content-SHA256'),
        offset: parseResponseNumber(response, 'x-mcnp-file-transfer-offset'),
        nextOffset: parseResponseNumber(response, 'x-mcnp-file-transfer-next-offset'),
        sizeBytes: parseResponseNumber(response, 'x-mcnp-file-transfer-size'),
        eof: response.headers.get('x-mcnp-file-eof') === 'true',
      } as T;
    }

    return response.json() as Promise<T>;
  }

  return {
    login(username, password, clientType) {
      return request<LoginResponse>('/api/v1/auth/login', {
        method: 'POST',
        body: { username, password, clientType },
      });
    },
    getCurrentUser() {
      return request<User>('/api/v1/auth/me');
    },
    logout() {
      return request<void>('/api/v1/auth/logout', { method: 'POST', csrf: true });
    },
    listCores() {
      return request<CorePage>('/api/v1/cores?limit=50');
    },
    listAuditEvents(limit = 100) {
      const query = new URLSearchParams({ limit: String(limit) });
      return request<PanelAuditPage>(`/api/v1/audit-events?${query.toString()}`);
    },
    getCpuTopology(coreId) {
      return request<CpuTopology>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/cpu-topology`,
      );
    },
    resolveCpuPolicy(coreId, policy) {
      return request<CpuPolicyResolution>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/cpu-policies:resolve`,
        { method: 'POST', body: policy },
      );
    },
    listCpuReservations(coreId) {
      return request<CpuReservationPage>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/cpu-reservations`,
      );
    },
    reserveCpu(coreId, reservationRequest) {
      return request<CpuReservationResult>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/cpu-reservations`,
        { method: 'POST', body: reservationRequest, csrf: true, idempotent: true },
      );
    },
    releaseCpu(coreId, reservationId) {
      return request<void>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/cpu-reservations/${encodeURIComponent(reservationId)}`,
        { method: 'DELETE', csrf: true, idempotent: true },
      );
    },
    listInstallTemplates() {
      return request<InstallTemplatePage>('/api/v1/install-templates');
    },
    listInstallTemplateVersions(templateId) {
      return request<InstallTemplateVersionPage>(
        `/api/v1/install-templates/${encodeURIComponent(templateId)}/versions`,
      );
    },
    searchExtensionCatalog(
      query,
      type,
      source = 'modrinth',
      minecraftVersion,
      loader,
      limit = 20,
      offset = 0,
    ) {
      const params = new URLSearchParams({
        query,
        type,
        source,
        limit: String(limit),
        offset: String(offset),
      });
      if (minecraftVersion !== undefined) {
        params.set('minecraftVersion', minecraftVersion);
      }
      if (loader !== undefined) {
        params.set('loader', loader);
      }
      return request<ExtensionSearchResult>(
        `/api/v1/extension-catalog/search?${params.toString()}`,
      );
    },
    getExtensionProjectVersions(source, projectId, minecraftVersion, loader) {
      const params = new URLSearchParams();
      if (minecraftVersion !== undefined) {
        params.set('minecraftVersion', minecraftVersion);
      }
      if (loader !== undefined) {
        params.set('loader', loader);
      }
      const query = params.toString();
      return request<ExtensionVersionResult>(
        `/api/v1/extension-catalog/projects/${encodeURIComponent(source)}/${encodeURIComponent(projectId)}${query ? `?${query}` : ''}`,
      );
    },
    resolveExtensionPlan(coreId, instanceId, plan) {
      return request<ExtensionPlanResolution>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/extension-plans:resolve`,
        { method: 'POST', body: plan, csrf: true },
      );
    },
    installExtensions(coreId, instanceId, installRequest) {
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/extensions`,
        { method: 'POST', body: installRequest, csrf: true, idempotent: true },
      );
    },
    updateExtension(coreId, instanceId, extensionId, updateRequest, expectedSha256) {
      const requestOptions: RequestOptions = {
        method: 'POST',
        body: updateRequest,
        csrf: true,
        idempotent: true,
      };
      if (expectedSha256 !== undefined) {
        requestOptions.ifMatch = expectedSha256;
      }
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/extensions/${encodeURIComponent(extensionId)}/actions/update`,
        requestOptions,
      );
    },
    getExtensionInstallTask(coreId, taskId) {
      return request<ExtensionInstallTask>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/extension-tasks/${encodeURIComponent(taskId)}`,
      );
    },
    listManagedRuntimes(coreId) {
      return request<ManagedRuntimePage>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/environments`,
      );
    },
    resolveProvisionPlan(coreId, plan) {
      return request<ProvisionResolution>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/provision-plans:resolve`,
        { method: 'POST', body: plan, csrf: true },
      );
    },
    executeProvision(coreId, plan, planHash) {
      return request<ProvisionOperation>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instance-provisions`,
        {
          method: 'POST',
          body: { resolvedPlan: plan, planHash },
          csrf: true,
          idempotent: true,
        },
      );
    },
    getProvisionTask(coreId, taskId) {
      return request<ProvisionOperation>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instance-provisions/${encodeURIComponent(taskId)}`,
      );
    },
    installRuntime(coreId, manifest, setAsDefault = false) {
      return request<RuntimeOperation>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/runtime-installations`,
        {
          method: 'POST',
          body: { manifest, setAsDefault },
          csrf: true,
          idempotent: true,
        },
      );
    },
    getRuntimeInstallation(coreId, taskId) {
      return request<RuntimeOperation>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/runtime-installations/${encodeURIComponent(taskId)}`,
      );
    },
    verifyRuntime(coreId, runtimeId) {
      return request<RuntimeOperation>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/runtimes/${encodeURIComponent(runtimeId)}/actions/verify`,
        { method: 'POST', csrf: true, idempotent: true },
      );
    },
    deleteRuntime(coreId, runtimeId) {
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/runtimes/${encodeURIComponent(runtimeId)}`,
        { method: 'DELETE', csrf: true, idempotent: true },
      );
    },
    getBedrockProfile(coreId, instanceId) {
      return request<BedrockManagementProfile>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/bedrock-profile`,
      );
    },
    checkBedrockPort(coreId, instanceId) {
      return request<BedrockPortCheck>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/bedrock-profile/actions/check-port`,
        { method: 'POST', csrf: true },
      );
    },
    listInstanceExtensions(coreId, instanceId, templateId, kind) {
      const query = new URLSearchParams({ templateId, kind });
      return request<InstanceExtensionScan>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/extensions?${query.toString()}`,
      );
    },
    writeInstanceExtension(coreId, instanceId, templateId, kind, path, content, expectedSha256) {
      const query = new URLSearchParams({ templateId, kind, path });
      const requestOptions: RequestOptions = {
        method: 'PUT',
        body: content,
        csrf: true,
        idempotent: true,
      };
      if (expectedSha256 !== undefined) {
        requestOptions.ifMatch = expectedSha256;
      }
      return request<FileEntry>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/extensions?${query.toString()}`,
        requestOptions,
      );
    },
    deleteInstanceExtension(coreId, instanceId, templateId, kind, path) {
      const query = new URLSearchParams({
        templateId,
        kind,
        path,
        confirmation: 'DELETE',
      });
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/extensions?${query.toString()}`,
        { method: 'DELETE', csrf: true, idempotent: true },
      );
    },
    listConfigDocuments(coreId, instanceId) {
      return request<ConfigDocumentPage>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/config-documents`,
      );
    },
    scanConfigDocuments(coreId, instanceId) {
      return request<ConfigDocumentPage>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/config-documents:scan`,
        { method: 'POST' },
      );
    },
    validateConfigDocuments(coreId, instanceId) {
      return request<ConfigValidationResult>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/config-documents:validate`,
        { method: 'POST' },
      );
    },
    getConfigDocument(coreId, instanceId, documentId) {
      return request<ConfigDocument>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/config-documents/${encodeURIComponent(documentId)}`,
      );
    },
    patchConfigDocument(coreId, instanceId, documentId, revision, patch, allowLossy = false) {
      return request<ConfigDocument>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/config-documents/${encodeURIComponent(documentId)}/values`,
        {
          method: 'PATCH',
          body: { revision, patch, allowLossy },
          csrf: true,
          idempotent: true,
        },
      );
    },
    async readRawConfig(coreId, instanceId, documentId) {
      const response = await request<BinaryFileResponse>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/config-documents/${encodeURIComponent(documentId)}/raw`,
        { accept: 'text/plain', responseType: 'arrayBuffer' },
      );
      if (response.etag === null) {
        throw new Error('Configuration response did not include an ETag');
      }
      return {
        data: response.data,
        etag: response.etag,
        sha256: response.etag.replace(/^"|"$/g, ''),
      };
    },
    writeRawConfig(coreId, instanceId, documentId, content, expectedSha256) {
      const requestOptions: RequestOptions = {
        method: 'PUT',
        body: content,
        csrf: true,
        idempotent: true,
      };
      if (expectedSha256 !== undefined) {
        requestOptions.ifMatch = expectedSha256;
      }
      return request<ConfigDocument>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/config-documents/${encodeURIComponent(documentId)}/raw`,
        requestOptions,
      );
    },
    listInstanceFiles(coreId, instanceId, path = '', cursor, limit) {
      const query = new URLSearchParams({ path });
      if (cursor !== undefined) {
        query.set('cursor', cursor);
      }
      if (limit !== undefined) {
        query.set('limit', String(limit));
      }
      return request<FilePage>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/files?${query.toString()}`,
      );
    },
    async readInstanceFile(coreId, instanceId, path, offset = 0, length = 32 * 1024) {
      const query = new URLSearchParams({ path, offset: String(offset), length: String(length) });
      const response = await request<BinaryFileResponse>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/file-content?${query.toString()}`,
        { accept: 'application/octet-stream', responseType: 'arrayBuffer' },
      );
      const etag = response.etag;
      if (etag === null) {
        throw new Error('File response did not include an ETag');
      }
      const sha256 = etag.replace(/^"|"$/g, '');
      return { data: response.data, etag, sha256, eof: response.eof };
    },
    writeInstanceFile(coreId, instanceId, path, content, expectedSha256) {
      const query = new URLSearchParams({ path });
      const requestOptions: RequestOptions = {
        method: 'PUT',
        body: content,
        csrf: true,
        idempotent: true,
      };
      if (expectedSha256 !== undefined) {
        requestOptions.ifMatch = expectedSha256;
      }
      return request<FileEntry>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/file-content?${query.toString()}`,
        requestOptions,
      );
    },
    createInstanceDirectory(coreId, instanceId, path, recursive = false) {
      return request<FileEntry>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/directories`,
        {
          method: 'POST',
          body: { path, recursive },
          csrf: true,
          idempotent: true,
        },
      );
    },
    moveInstanceFile(coreId, instanceId, from, to, overwrite = false) {
      return request<FileEntry>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/file-actions/move`,
        {
          method: 'POST',
          body: { from, to, overwrite },
          csrf: true,
          idempotent: true,
        },
      );
    },
    beginFileUpload(coreId, instanceId, path, sizeBytes, sha256, expectedSha256) {
      return request<FileUploadStart>(
        '/api/v1/cores/' +
          encodeURIComponent(coreId) +
          '/instances/' +
          encodeURIComponent(instanceId) +
          '/uploads',
        {
          method: 'POST',
          body: { path, sizeBytes, sha256, expectedSha256 },
          csrf: true,
          idempotent: true,
        },
      );
    },
    beginFileDownload(coreId, instanceId, path) {
      return request<FileDownloadStart>(
        '/api/v1/cores/' +
          encodeURIComponent(coreId) +
          '/instances/' +
          encodeURIComponent(instanceId) +
          '/downloads',
        {
          method: 'POST',
          body: { path },
          csrf: true,
          idempotent: true,
        },
      );
    },
    uploadFilePart(coreId, transferId, partNumber, content, sha256) {
      return request<FileUploadPart>(
        '/api/v1/cores/' +
          encodeURIComponent(coreId) +
          '/uploads/' +
          encodeURIComponent(transferId) +
          '/parts/' +
          encodeURIComponent(String(partNumber)),
        {
          method: 'PUT',
          body: content,
          headers: { 'Content-SHA256': sha256 },
          csrf: true,
          idempotent: true,
        },
      );
    },
    async downloadFilePart(coreId, transferId, partNumber) {
      const response = await request<BinaryFileResponse>(
        '/api/v1/cores/' +
          encodeURIComponent(coreId) +
          '/downloads/' +
          encodeURIComponent(transferId) +
          '/parts/' +
          encodeURIComponent(String(partNumber)),
        { accept: 'application/octet-stream', responseType: 'arrayBuffer' },
      );
      if (
        response.etag === null ||
        response.contentSha256 === null ||
        response.offset === null ||
        response.nextOffset === null ||
        response.sizeBytes === null
      ) {
        throw new Error('File download response did not include transfer metadata');
      }
      return {
        data: response.data,
        offset: response.offset,
        nextOffset: response.nextOffset,
        sizeBytes: response.sizeBytes,
        sha256: response.contentSha256,
        etag: response.etag,
        eof: response.eof,
      };
    },
    completeFileUpload(coreId, transferId) {
      return request<FileEntry>(
        '/api/v1/cores/' +
          encodeURIComponent(coreId) +
          '/uploads/' +
          encodeURIComponent(transferId) +
          '/complete',
        { method: 'POST', csrf: true, idempotent: true },
      );
    },
    abortFileUpload(coreId, transferId) {
      return request<void>(
        '/api/v1/cores/' +
          encodeURIComponent(coreId) +
          '/uploads/' +
          encodeURIComponent(transferId),
        { method: 'DELETE', csrf: true, idempotent: true },
      );
    },
    completeFileDownload(coreId, transferId) {
      return request<void>(
        '/api/v1/cores/' +
          encodeURIComponent(coreId) +
          '/downloads/' +
          encodeURIComponent(transferId) +
          '/complete',
        { method: 'POST', csrf: true, idempotent: true },
      );
    },
    abortFileDownload(coreId, transferId) {
      return request<void>(
        '/api/v1/cores/' +
          encodeURIComponent(coreId) +
          '/downloads/' +
          encodeURIComponent(transferId),
        { method: 'DELETE', csrf: true, idempotent: true },
      );
    },
    batchInstanceFiles(coreId, instanceId, operations) {
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/file-actions/batch`,
        {
          method: 'POST',
          body: { operations },
          csrf: true,
          idempotent: true,
        },
      );
    },
    createFileArchive(coreId, instanceId, paths, outputPath) {
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/archives`,
        {
          method: 'POST',
          body: { paths, outputPath },
          csrf: true,
          idempotent: true,
        },
      );
    },
    deleteInstanceFile(coreId, instanceId, path, recursive = false) {
      const query = new URLSearchParams({ path, confirmation: 'DELETE' });
      if (recursive) {
        query.set('recursive', 'true');
      }
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/files?${query.toString()}`,
        { method: 'DELETE', csrf: true, idempotent: true },
      );
    },
    getFileTask(coreId, taskId) {
      return request<FileTask>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/file-tasks/${encodeURIComponent(taskId)}`,
      );
    },
    listProxySubservers(coreId, proxyInstanceId) {
      return request<ProxySubserverPage>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(proxyInstanceId)}/proxy-subservers`,
      );
    },
    upsertProxySubserver(coreId, proxyInstanceId, subserver) {
      return request<ProxySubserver>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(proxyInstanceId)}/proxy-subservers`,
        { method: 'POST', body: subserver, csrf: true, idempotent: true },
      );
    },
    deleteProxySubserver(coreId, proxyInstanceId, subserverId) {
      return request<void>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(proxyInstanceId)}/proxy-subservers/${encodeURIComponent(subserverId)}`,
        { method: 'DELETE', csrf: true, idempotent: true },
      );
    },
    checkProxySubserver(coreId, proxyInstanceId, subserverId) {
      return request<ProxySubserverHealth>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(proxyInstanceId)}/proxy-subservers/${encodeURIComponent(subserverId)}/actions/check`,
        { method: 'POST', csrf: true },
      );
    },
    listInstances(coreId) {
      return request<InstancePage>(`/api/v1/cores/${encodeURIComponent(coreId)}/instances?limit=50`);
    },
    updateInstance(coreId, instanceId, update, revision) {
      return request<Instance>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}`,
        { method: 'PATCH', body: update, csrf: true, ifMatch: revision },
      );
    },
    listInstanceAudit(coreId, instanceId, limit = 200) {
      const query = new URLSearchParams({ limit: String(limit) });
      return request<InstanceAuditPage>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/audit?${query.toString()}`,
      );
    },
    getInstanceLogs(coreId, instanceId) {
      return request<LogPage>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/logs?limit=200`,
      );
    },
    startInstance(coreId, instanceId) {
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/actions/start`,
        { method: 'POST', csrf: true, idempotent: true },
      );
    },
    stopInstance(coreId, instanceId) {
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/actions/stop`,
        { method: 'POST', csrf: true, idempotent: true },
      );
    },
    killInstance(coreId, instanceId) {
      return request<TaskAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/actions/kill`,
        { method: 'POST', csrf: true, idempotent: true, body: { confirmation: 'KILL' } },
      );
    },
    resetInstance(coreId, instanceId) {
      return request<Instance>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/actions/reset`,
        { method: 'POST', csrf: true, idempotent: true, body: { confirmation: 'RESET' } },
      );
    },
    sendInstanceCommand(coreId, instanceId, command) {
      return request<CommandAccepted>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/commands`,
        { method: 'POST', csrf: true, idempotent: true, body: { command } },
      );
    },
  };
}

interface BinaryFileResponse {
  data: ArrayBuffer;
  etag: string | null;
  contentSha256: string | null;
  offset: number | null;
  nextOffset: number | null;
  sizeBytes: number | null;
  eof: boolean;
}

function parseResponseNumber(response: Response, name: string): number | null {
  const value = response.headers.get(name);
  if (value === null) {
    return null;
  }
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : null;
}

function isBinaryBody(value: unknown): boolean {
  return (
    (typeof Blob !== 'undefined' && value instanceof Blob) ||
    (typeof ArrayBuffer !== 'undefined' && value instanceof ArrayBuffer) ||
    (typeof ArrayBuffer !== 'undefined' && ArrayBuffer.isView(value))
  );
}

async function createRequestError(response: Response): Promise<ApiRequestError> {
  const body = await readErrorBody(response);
  const code = body.error?.code ?? `HTTP_${response.status}`;
  const message = body.error?.message ?? response.statusText;

  return new ApiRequestError(response.status, code, message, body.error?.requestId);
}

async function readErrorBody(response: Response): Promise<ErrorBody> {
  try {
    return (await response.json()) as ErrorBody;
  } catch {
    return {};
  }
}

function createRequestId(): string {
  if (globalThis.crypto?.randomUUID) {
    return globalThis.crypto.randomUUID();
  }

  return `${Date.now().toString(16)}-${Math.random().toString(16).slice(2)}`;
}
