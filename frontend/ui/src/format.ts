/** 通用格式化工具。 */

import dayjs from 'dayjs';

export function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  const idx = Math.min(units.length - 1, Math.floor(Math.log2(bytes) / 10));
  const value = bytes / 2 ** (10 * idx);
  return `${value >= 100 ? Math.round(value) : value.toFixed(1)} ${units[idx]}`;
}

export function formatPercent(ratio: number): string {
  return `${Math.round(ratio * 100)}%`;
}

export function formatTime(at: number | null): string {
  if (!at) return '—';
  return dayjs(at).format('YYYY-MM-DD HH:mm:ss');
}

export function formatRelative(at: number | null): string {
  if (!at) return '—';
  const diff = Date.now() - at;
  if (diff < 0) {
    // 未来时间（如到期时间）
    const ahead = -diff;
    if (ahead < 60_000) return '即将到期';
    if (ahead < 3_600_000) return `${Math.ceil(ahead / 60_000)} 分钟后`;
    if (ahead < 86_400_000) return `${Math.ceil(ahead / 3_600_000)} 小时后`;
    return `${Math.ceil(ahead / 86_400_000)} 天后`;
  }
  if (diff < 60_000) return '刚刚';
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`;
  return `${Math.floor(diff / 86_400_000)} 天前`;
}

export function formatDuration(ms: number): string {
  if (ms < 1_000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1_000).toFixed(1)}s`;
  return `${Math.floor(ms / 60_000)}m${Math.round((ms % 60_000) / 1_000)}s`;
}
