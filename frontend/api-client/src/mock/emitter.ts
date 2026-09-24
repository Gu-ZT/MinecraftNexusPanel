/**
 * Mock 层内部使用的类型化事件总线。
 * 仅服务 MockBackend，不是公开 API。
 */

import type { RealtimeEventMap, RealtimeKind } from '../realtime';

/** 内部存储形态：kind 之外的类型信息在订阅边界完成校验。 */
interface Subscription {
  kind: RealtimeKind;
  match: (event: unknown) => boolean;
  handler: (event: unknown) => void;
}

/** 简单多播事件总线：按 kind 分发，订阅方自带过滤。 */
export class MockEmitter {
  private subs: Subscription[] = [];

  on<K extends RealtimeKind>(
    kind: K,
    match: (event: RealtimeEventMap[K]) => boolean,
    handler: (event: RealtimeEventMap[K]) => void,
  ): () => void {
    const sub: Subscription = {
      kind,
      match: match as (event: unknown) => boolean,
      handler: handler as (event: unknown) => void,
    };
    this.subs.push(sub);
    return () => {
      this.subs = this.subs.filter((s) => s !== sub);
    };
  }

  emit<K extends RealtimeKind>(kind: K, event: RealtimeEventMap[K]): void {
    for (const sub of this.subs) {
      if (sub.kind !== kind) continue;
      if (sub.match(event)) sub.handler(event);
    }
  }

  clear(): void {
    this.subs = [];
  }
}
