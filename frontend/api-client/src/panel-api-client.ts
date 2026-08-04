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
export type RuntimeArchiveFormat = 'TAR_GZ' | 'ZIP';
export type DownloadPlatform = 'LINUX' | 'MACOS' | 'WINDOWS';
export type DownloadArchitecture = 'AARCH64' | 'X86_64';
export type BedrockManagementKind = 'DEDICATED_SERVER' | 'POCKET_MINE' | 'NUKKIT' | 'GEYSER';
export type BedrockTransport = 'RAKNET_UDP';
export type InstallRuntimeRequirement = 'JAVA' | 'NODE_JS' | 'PYTHON' | 'PHP' | 'NATIVE';
export type InstallTemplateFamily = 'JAVA_SERVER' | 'JAVA_PROXY' | 'BEDROCK_SERVER' | 'BEDROCK_PROXY';
export type ProxyTopology = 'NONE' | 'ONE_TO_MANY' | 'ONE_TO_ONE';
export type ExtensionKind = 'PLUGIN' | 'MOD';
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
  defaultPort: number;
  configurationFiles: string[];
  extensionKind: ExtensionKind | null;
  extensionDirectories: string[];
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

export interface InstanceExtensionScan {
  templateId: string;
  kind: ExtensionKind;
  directories: ExtensionDirectoryPage[];
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

export interface LaunchConfig {
  executable: string;
  args: string[];
  environment: Record<string, string>;
  stopCommand: string;
  stopTimeoutSeconds: number;
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
  listInstallTemplates(): Promise<InstallTemplatePage>;
  listInstallTemplateVersions(templateId: string): Promise<InstallTemplateVersionPage>;
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
  listInstances(coreId: string): Promise<InstancePage>;
  updateInstance(coreId: string, instanceId: string, update: InstanceUpdate, revision: number): Promise<Instance>;
  getInstanceLogs(coreId: string, instanceId: string): Promise<LogPage>;
  startInstance(coreId: string, instanceId: string): Promise<TaskAccepted>;
  stopInstance(coreId: string, instanceId: string): Promise<TaskAccepted>;
  killInstance(coreId: string, instanceId: string): Promise<TaskAccepted>;
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
    listInstallTemplates() {
      return request<InstallTemplatePage>('/api/v1/install-templates');
    },
    listInstallTemplateVersions(templateId) {
      return request<InstallTemplateVersionPage>(
        `/api/v1/install-templates/${encodeURIComponent(templateId)}/versions`,
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
    listInstanceExtensions(coreId, instanceId, templateId, kind) {
      const query = new URLSearchParams({ templateId, kind });
      return request<InstanceExtensionScan>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/extensions?${query.toString()}`,
      );
    },
    writeInstanceExtension(coreId, instanceId, templateId, kind, path, content) {
      const query = new URLSearchParams({ templateId, kind, path });
      return request<FileEntry>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/extensions?${query.toString()}`,
        { method: 'PUT', body: content, csrf: true, idempotent: true },
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
    beginFileUpload(coreId, instanceId, path, sizeBytes, sha256) {
      return request<FileUploadStart>(
        '/api/v1/cores/' +
          encodeURIComponent(coreId) +
          '/instances/' +
          encodeURIComponent(instanceId) +
          '/uploads',
        {
          method: 'POST',
          body: { path, sizeBytes, sha256 },
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
    listInstances(coreId) {
      return request<InstancePage>(`/api/v1/cores/${encodeURIComponent(coreId)}/instances?limit=50`);
    },
    updateInstance(coreId, instanceId, update, revision) {
      return request<Instance>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}`,
        { method: 'PATCH', body: update, csrf: true, ifMatch: revision },
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
