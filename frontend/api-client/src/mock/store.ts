/**
 * Mock 后端状态仓库：持有内存数据、任务推进引擎与实例状态机模拟。
 * 仅服务原型开发，行为对齐 PLAN.md 第 6 节状态机与第 4.2 节 202 语义。
 */

import { buildSeed, type MockState } from './seed';
import { MockEmitter } from './emitter';
import type {
  Instance,
  InstanceRuntime,
  InstanceState,
  Permission,
  Task,
  TaskKind,
  User,
} from '../domain';

/** Mock API 错误：shape 对齐未来真实错误模型 { status, code, message }。 */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export function latency(): Promise<void> {
  const ms = 100 + Math.random() * 300;
  return new Promise((resolve) => setTimeout(resolve, ms));
}

interface RunningTask {
  task: Task;
  durationMs: number;
  onDone: (() => void) | null;
}

export class MockStore {
  readonly state: MockState = buildSeed();
  readonly emitter = new MockEmitter();

  /** 当前登录用户 ID；null 表示未登录。 */
  currentUserId: string | null = null;

  private seq = 1000;
  private runningTasks = new Map<string, RunningTask>();
  private engineTimer: ReturnType<typeof setInterval> | null = null;

  nextId(prefix: string): string {
    this.seq += 1;
    return `${prefix}-${this.seq}`;
  }

  // -------------------------------------------------------------------------
  // 会话与权限（PLAN 4.4：服务端过滤，前端隐藏只是配合）
  // -------------------------------------------------------------------------

  currentUser(): User {
    const user = this.state.users.find((u) => u.id === this.currentUserId);
    if (!user) throw new ApiError(401, 'UNAUTHORIZED', '未登录或会话已过期');
    return user;
  }

  permissionsOf(user: User): Permission[] {
    const set = new Set<Permission>();
    for (const group of this.state.groups) {
      if (user.groupIds.includes(group.id)) {
        for (const p of group.permissions) set.add(p);
      }
    }
    return [...set];
  }

  /** 校验当前用户是否持有权限点；不持有则抛 403。 */
  requirePermission(permission: Permission): User {
    const user = this.currentUser();
    if (!this.permissionsOf(user).includes(permission)) {
      throw new ApiError(403, 'FORBIDDEN', `缺少权限 ${permission}`);
    }
    return user;
  }

  /** 实例对当前用户是否可见（按用户组 instanceScopes 过滤）。 */
  canSeeInstance(instance: Instance, user: User): boolean {
    const groups = this.state.groups.filter((g) => user.groupIds.includes(g.id));
    return groups.some((g) => {
      const scope = g.instanceScopes;
      if (scope.coreIds.length === 0 && scope.instanceIds.length === 0 && scope.tagSelectors.length === 0) {
        return true; // 空范围 = 不限制（管理员组）
      }
      if (scope.coreIds.includes(instance.coreId)) return true;
      if (scope.instanceIds.includes(instance.id)) return true;
      return instance.tags.some((t) => scope.tagSelectors.includes(t));
    });
  }

  requireVisibleInstance(instanceId: string): Instance {
    const user = this.currentUser();
    const instance = this.state.instances.find((i) => i.id === instanceId);
    if (!instance || !this.canSeeInstance(instance, user)) {
      throw new ApiError(404, 'INSTANCE_NOT_FOUND', '实例不存在或不在授权范围内');
    }
    return instance;
  }

  runtimeOf(instanceId: string): InstanceRuntime {
    const runtime = this.state.runtimes.get(instanceId);
    if (!runtime) throw new ApiError(404, 'RUNTIME_NOT_FOUND', `实例 ${instanceId} 缺少运行时记录`);
    return runtime;
  }

  // -------------------------------------------------------------------------
  // 实例状态机模拟（PLAN 第 6 节：状态变更只能由 Core 事件产生）
  // -------------------------------------------------------------------------

  /** 立即变更状态并广播事件。 */
  setInstanceState(instanceId: string, state: InstanceState, patch?: Partial<InstanceRuntime>): void {
    const instance = this.state.instances.find((i) => i.id === instanceId);
    if (!instance) throw new ApiError(404, 'INSTANCE_NOT_FOUND', `实例 ${instanceId} 不存在`);
    instance.state = state;
    const runtime = this.runtimeOf(instanceId);
    runtime.state = state;
    if (patch) Object.assign(runtime, patch);
    // 维护节点级运行计数
    const core = this.state.cores.find((c) => c.id === instance.coreId);
    if (core) {
      core.runningCount = this.state.instances.filter((i) => i.coreId === core.id && i.state === 'RUNNING').length;
    }
    this.emitter.emit('instance-state', { instanceId, state, runtime: { ...runtime } });
  }

  /** 延迟推进状态（模拟 STARTING→RUNNING 等异步流转），期间输出控制台日志。 */
  scheduleTransition(instanceId: string, target: InstanceState, delayMs: number, logs: string[], patch?: Partial<InstanceRuntime>): void {
    const step = Math.max(200, Math.floor(delayMs / (logs.length + 1)));
    logs.forEach((line, idx) => {
      setTimeout(() => {
        this.emitter.emit('console', { instanceId, line, stream: 'stdout', at: Date.now() });
      }, step * (idx + 1));
    });
    setTimeout(() => {
      try {
        this.setInstanceState(instanceId, target, patch);
      } catch (err) {
        console.warn(`[mock] 状态推进失败：实例 ${instanceId} 可能已被删除`, err);
      }
    }, delayMs);
  }

  // -------------------------------------------------------------------------
  // 任务引擎：RUNNING 任务随时间推进进度并广播 tasks 事件
  // -------------------------------------------------------------------------

  startTask(input: {
    kind: TaskKind;
    title: string;
    coreId?: string | null;
    instanceId?: string | null;
    durationMs: number;
    cancellable?: boolean;
    onDone?: () => void;
  }): Task {
    const task: Task = {
      id: this.nextId('task'),
      kind: input.kind,
      title: input.title,
      status: 'RUNNING',
      progress: 0,
      message: null,
      coreId: input.coreId ?? null,
      instanceId: input.instanceId ?? null,
      createdAt: Date.now(),
      finishedAt: null,
      cancellable: input.cancellable ?? true,
    };
    this.state.tasks.unshift(task);
    this.runningTasks.set(task.id, { task, durationMs: input.durationMs, onDone: input.onDone ?? null });
    this.emitter.emit('tasks', { task: { ...task } });
    this.ensureEngine();
    return task;
  }

  cancelTask(taskId: string): void {
    const running = this.runningTasks.get(taskId);
    if (!running) throw new ApiError(409, 'TASK_NOT_CANCELLABLE', '任务已结束，无法取消');
    this.runningTasks.delete(taskId);
    running.task.status = 'CANCELLED';
    running.task.finishedAt = Date.now();
    this.emitter.emit('tasks', { task: { ...running.task } });
  }

  private ensureEngine(): void {
    if (this.engineTimer) return;
    const TICK = 400;
    this.engineTimer = setInterval(() => {
      for (const [id, running] of this.runningTasks) {
        const next = (running.task.progress ?? 0) + TICK / running.durationMs;
        if (next >= 1) {
          this.runningTasks.delete(id);
          running.task.progress = 1;
          running.task.status = 'SUCCESS';
          running.task.finishedAt = Date.now();
          running.task.cancellable = false;
          try {
            running.onDone?.();
          } catch (err) {
            console.warn(`[mock] 任务 ${id} 完成回调失败`, err);
            running.task.status = 'FAILED';
            running.task.message = '完成回调失败（Mock 内部错误）';
          }
        } else {
          running.task.progress = Math.round(next * 100) / 100;
        }
        this.emitter.emit('tasks', { task: { ...running.task } });
      }
      if (this.runningTasks.size === 0 && this.engineTimer) {
        clearInterval(this.engineTimer);
        this.engineTimer = null;
      }
    }, TICK);
  }

  dispose(): void {
    if (this.engineTimer) {
      clearInterval(this.engineTimer);
      this.engineTimer = null;
    }
    this.runningTasks.clear();
    this.emitter.clear();
  }
}
