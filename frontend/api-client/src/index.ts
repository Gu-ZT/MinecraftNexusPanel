/**
 * @mcnp/api-client —— MCNP Web API 客户端（PLAN.md 第 4.2 节）。
 *
 * - domain：领域类型（对齐 PLAN 第 6 节）
 * - api：McnpApi 接口（/api/v1 资源分组）
 * - realtime：实时事件订阅接口
 * - mock：Mock 实现（当前原型唯一实现）
 *
 * TODO(M1): 由 OpenAPI 生成真实 HTTP 客户端与 WebSocket 客户端，实现同一接口。
 */

export * from './domain';
export type {
  Accepted,
  AuditFilter,
  CreateCoreInput,
  CreateGroupInput,
  CreateInstanceInput,
  CreateScheduleInput,
  CreateUserInput,
  InstanceFilter,
  InstanceSettingsPatch,
  McnpApi,
} from './api';
export type { RealtimeClient, RealtimeEventMap, RealtimeKind, RealtimeTopic } from './realtime';
export { ApiError } from './mock/store';
export { createMockBackend, type MockBackend } from './mock/backend';
