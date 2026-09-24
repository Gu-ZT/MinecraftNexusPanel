/**
 * MCNP 实时事件 SDK（PLAN.md 第 4.2 节）。
 *
 * 实时日志、指标、节点状态和任务进度统一经 WebSocket 推送；本接口是
 * 前端订阅的唯一入口，当前由 MockRealtime 驱动原型。
 * TODO(M1): 实现 WebSocket 客户端（topic 订阅协议见 docs/api/websocket.md）。
 */

import type { CoreStatus, InstanceRuntime, InstanceState, Task } from './domain';

/** 订阅主题：kind 决定事件载荷类型，其余字段为过滤维度。 */
export type RealtimeTopic =
  | { kind: 'console'; instanceId: string }
  | { kind: 'instance-state'; instanceId?: string }
  | { kind: 'metrics'; instanceId: string }
  | { kind: 'core-status'; coreId?: string }
  | { kind: 'tasks' }
  | { kind: 'image-build-log'; buildId: string };

export interface RealtimeEventMap {
  console: { instanceId: string; line: string; stream: 'stdout' | 'stderr'; at: number };
  'instance-state': { instanceId: string; state: InstanceState; runtime: InstanceRuntime };
  metrics: {
    instanceId: string;
    cpuUsage: number;
    memoryBytes: number;
    playerCount: number | null;
    at: number;
  };
  'core-status': { coreId: string; status: CoreStatus; cpuUsage: number; memoryUsage: number };
  tasks: { task: Task };
  'image-build-log': { buildId: string; line: string; at: number };
}

export type RealtimeKind = keyof RealtimeEventMap;

/** 实时事件客户端；subscribe 返回取消订阅函数。 */
export interface RealtimeClient {
  subscribe<K extends RealtimeKind>(
    topic: Extract<RealtimeTopic, { kind: K }>,
    handler: (event: RealtimeEventMap[K]) => void,
  ): () => void;
  close(): void;
}
