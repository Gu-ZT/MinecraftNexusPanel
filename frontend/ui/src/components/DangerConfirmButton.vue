<script setup lang="ts">
/** 危险操作按钮：Popconfirm 二次确认后才触发（PLAN 第 9 节危险操作语义）。 */

import { Button, Popconfirm } from '@arco-design/web-vue';

withDefaults(
  defineProps<{
    confirmText: string;
    loading?: boolean;
    disabled?: boolean;
    size?: 'mini' | 'small' | 'medium' | 'large';
  }>(),
  { loading: false, disabled: false, size: 'medium' },
);

const emit = defineEmits<{ confirm: [] }>();
</script>

<template>
  <Popconfirm :content="confirmText" type="error" position="bottom" @ok="emit('confirm')">
    <Button status="danger" :loading="loading" :disabled="disabled" :size="size">
      <slot />
    </Button>
  </Popconfirm>
</template>
