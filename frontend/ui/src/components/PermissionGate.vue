<script setup lang="ts">
/** 权限渲染闸门：当前用户持有全部指定权限点时才渲染插槽内容。 */

import { computed, inject } from 'vue';
import type { Permission } from '@mcnp/api-client';
import { PermissionKey } from '../permissions';

const props = defineProps<{ when: Permission | Permission[] }>();

const ctx = inject(PermissionKey);

const allowed = computed(() => {
  if (!ctx) return false;
  const required = Array.isArray(props.when) ? props.when : [props.when];
  return ctx.hasAll(required);
});
</script>

<template>
  <slot v-if="allowed" />
</template>
