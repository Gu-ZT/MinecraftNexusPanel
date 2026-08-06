import { ApiRequestError } from '@mcnp/api-client';
import type { InstanceState } from '@mcnp/api-client';

export function describeError(error: unknown, fallback: string): string {
  if (error instanceof ApiRequestError || error instanceof Error) {
    return `${fallback}: ${error.message}`;
  }
  return fallback;
}

export function canStartInstance(state: InstanceState | undefined): boolean {
  return state === 'CREATED' || state === 'STOPPED' || state === 'FAILED' || state === 'UNKNOWN';
}

export function canStopInstance(state: InstanceState | undefined): boolean {
  return state === 'RUNNING' || state === 'STARTING';
}

export function statusClass(status: string): string {
  return `status status-${status.toLocaleLowerCase().replaceAll('_', '-')}`;
}

export function formatDate(
  value: string | null | undefined,
  locale: string,
  fallback: string,
  includeDate = true,
): string {
  if (!value) {
    return fallback;
  }
  return new Intl.DateTimeFormat(locale, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    ...(includeDate ? { year: 'numeric', month: '2-digit', day: '2-digit' } : {}),
  }).format(new Date(value));
}

export function formatBytes(value: number, locale: string): string {
  if (value < 1024) {
    return `${value} B`;
  }
  const units = ['KiB', 'MiB', 'GiB', 'TiB'];
  let amount = value / 1024;
  let unit = units[0] ?? 'KiB';
  for (const nextUnit of units.slice(1)) {
    if (amount < 1024) {
      break;
    }
    amount /= 1024;
    unit = nextUnit;
  }
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(amount)} ${unit}`;
}
