/**
 * @mcnp/ui —— MCNP 设计令牌与领域共享组件（三端复用）。
 */

export { PermissionKey, type PermissionContext } from './permissions';
export * from './format';

export { default as PageHeader } from './components/PageHeader.vue';
export { default as InstanceStateTag } from './components/InstanceStateTag.vue';
export { default as CoreStatusBadge } from './components/CoreStatusBadge.vue';
export { default as LogTerminal } from './components/LogTerminal.vue';
export { default as TaskProgressCard } from './components/TaskProgressCard.vue';
export { default as PermissionGate } from './components/PermissionGate.vue';
export { default as MetricSparkline } from './components/MetricSparkline.vue';
export { default as DangerConfirmButton } from './components/DangerConfirmButton.vue';
