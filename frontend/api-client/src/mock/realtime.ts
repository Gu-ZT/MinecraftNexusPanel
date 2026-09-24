/**
 * MockRealtime：RealtimeClient 的内存实现。
 * 周期推送控制台行、指标、节点状态；任务事件由 MockStore 引擎直接广播。
 */

import type { RealtimeClient, RealtimeEventMap, RealtimeKind, RealtimeTopic } from '../realtime';
import type { MockStore } from './store';

const IDLE_LOG_LINES = [
  '[Server thread/INFO]: [Chunky] Task finished for world (processed 12481 chunks)',
  '[Server thread/INFO]: There are 17 of a max of 50 players online',
  '[Netty Server IO/INFO]: /203.0.113.55:52134 lost connection: Disconnected',
  '[Server thread/INFO]: Steve joined the game',
  '[Server thread/INFO]: Alex left the game',
  '[Server thread/WARN]: Can\'t keep up! Is the server overloaded? Running 2124ms or 42 ticks behind',
  '[Server thread/INFO]: Saving is already turned on',
  '[Server thread/INFO]: [spark] TPS from last 1m, 5m, 15m: 20.0, 19.9, 19.8',
];

export class MockRealtime implements RealtimeClient {
  private consoleSubs = new Map<string, number>();
  private metricsSubs = new Map<string, number>();
  private coreSubs = new Map<string, number>();
  private timers: ReturnType<typeof setInterval>[] = [];

  constructor(private readonly store: MockStore) {
    // 控制台流水：仅对"有订阅且 RUNNING"的实例产生日志，模拟真实 WS 的按需推送。
    this.timers.push(
      setInterval(() => {
        for (const [instanceId] of this.consoleSubs) {
          const instance = this.store.state.instances.find((i) => i.id === instanceId);
          if (!instance || instance.state !== 'RUNNING') continue;
          const line = IDLE_LOG_LINES[Math.floor(Math.random() * IDLE_LOG_LINES.length)] ?? '';
          this.store.emitter.emit('console', { instanceId, line, stream: 'stdout', at: Date.now() });
        }
      }, 900),
    );

    // 指标推送：CPU 小幅随机游走。
    this.timers.push(
      setInterval(() => {
        for (const [instanceId] of this.metricsSubs) {
          const runtime = this.store.state.runtimes.get(instanceId);
          if (!runtime || runtime.state !== 'RUNNING') continue;
          runtime.cpuUsage = clamp01(runtime.cpuUsage + (Math.random() - 0.5) * 0.06);
          this.store.emitter.emit('metrics', {
            instanceId,
            cpuUsage: runtime.cpuUsage,
            memoryBytes: runtime.memoryBytes,
            playerCount: runtime.playerCount,
            at: Date.now(),
          });
        }
      }, 2_000),
    );

    // 节点状态推送。
    this.timers.push(
      setInterval(() => {
        if (this.coreSubs.size === 0) return;
        for (const core of this.store.state.cores) {
          core.cpuUsage = clamp01(core.cpuUsage + (Math.random() - 0.5) * 0.04);
          this.store.emitter.emit('core-status', {
            coreId: core.id,
            status: core.status,
            cpuUsage: core.cpuUsage,
            memoryUsage: core.memoryUsage,
          });
        }
      }, 3_000),
    );
  }

  subscribe<K extends RealtimeKind>(
    topic: Extract<RealtimeTopic, { kind: K }>,
    handler: (event: RealtimeEventMap[K]) => void,
  ): () => void {
    const t = topic as RealtimeTopic;
    const match = (event: RealtimeEventMap[K]): boolean => {
      if (t.kind === 'console' || t.kind === 'metrics') {
        return (event as { instanceId: string }).instanceId === t.instanceId;
      }
      if (t.kind === 'instance-state' && t.instanceId) {
        return (event as { instanceId: string }).instanceId === t.instanceId;
      }
      if (t.kind === 'core-status' && t.coreId) {
        return (event as { coreId: string }).coreId === t.coreId;
      }
      if (t.kind === 'image-build-log') {
        return (event as { buildId: string }).buildId === t.buildId;
      }
      return true; // tasks / 未限定维度的主题
    };
    this.track(t, +1);
    const off = this.store.emitter.on(t.kind as K, match, handler);
    return () => {
      this.track(t, -1);
      off();
    };
  }

  private track(topic: RealtimeTopic, delta: number): void {
    const bump = (map: Map<string, number>, key: string) => {
      const next = (map.get(key) ?? 0) + delta;
      if (next <= 0) map.delete(key);
      else map.set(key, next);
    };
    if (topic.kind === 'console') bump(this.consoleSubs, topic.instanceId);
    if (topic.kind === 'metrics') bump(this.metricsSubs, topic.instanceId);
    if (topic.kind === 'core-status') bump(this.coreSubs, topic.coreId ?? '*');
  }

  close(): void {
    for (const timer of this.timers) clearInterval(timer);
    this.timers = [];
    this.consoleSubs.clear();
    this.metricsSubs.clear();
    this.coreSubs.clear();
  }
}

function clamp01(v: number): number {
  return Math.min(1, Math.max(0, v));
}
