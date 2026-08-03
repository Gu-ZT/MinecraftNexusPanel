import type { ApiClientOptions } from './api-client-options';
import { createApiUrl } from './create-api-url';

type HttpMethod = 'DELETE' | 'GET' | 'PATCH' | 'POST';

interface RequestOptions {
  method?: HttpMethod;
  body?: unknown;
  csrf?: boolean;
  ifMatch?: number;
  idempotent?: boolean;
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
  kind: RuntimeKind;
  source: RuntimeSource;
  executable: string;
  version: string | null;
  validation: RuntimeValidation;
}

export interface ManagedRuntimePage {
  items: ManagedRuntime[];
}

export interface BedrockManagementProfile {
  managementKind: BedrockManagementKind;
  transport: BedrockTransport;
  defaultPort: number;
  configurationFiles: string[];
  extensionKind: ExtensionKind | null;
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
  getBedrockProfile(coreId: string, instanceId: string): Promise<BedrockManagementProfile>;
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
    const headers = new Headers({ Accept: 'application/json' });
    const accessToken = options.getAccessToken?.();
    const csrfToken = requestOptions.csrf ? options.getCsrfToken?.() : undefined;
    if (accessToken) {
      headers.set('Authorization', `Bearer ${accessToken}`);
    }
    if (csrfToken) {
      headers.set('X-CSRF-Token', csrfToken);
    }
    if (requestOptions.idempotent) {
      headers.set('Idempotency-Key', createRequestId());
    }
    if (requestOptions.ifMatch !== undefined) {
      headers.set('If-Match', `"${requestOptions.ifMatch}"`);
    }
    if (requestOptions.body !== undefined) {
      headers.set('Content-Type', 'application/json');
    }

    const init: RequestInit = {
      method,
      headers,
      credentials: 'same-origin',
    };
    if (requestOptions.body !== undefined) {
      init.body = JSON.stringify(requestOptions.body);
    }

    const response = await fetch(createApiUrl(options.baseUrl, path), init);
    if (!response.ok) {
      throw await createRequestError(response);
    }
    if (response.status === 204) {
      return undefined as T;
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
    getBedrockProfile(coreId, instanceId) {
      return request<BedrockManagementProfile>(
        `/api/v1/cores/${encodeURIComponent(coreId)}/instances/${encodeURIComponent(instanceId)}/bedrock-profile`,
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
