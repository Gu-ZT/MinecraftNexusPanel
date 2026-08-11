<script setup lang="ts">
/*
 * EmptyState：居中空状态（docs/design/frontend-design.md §5）。
 * 使用纯 CSS 图形占位，避免引入图片资源；动作按钮放默认插槽。
 */
defineProps<{
  title: string;
  description?: string;
}>();
</script>

<template>
  <div class="empty-state" role="status">
    <div class="empty-state__art" aria-hidden="true">
      <i class="empty-state__art-dot empty-state__art-dot--a" />
      <i class="empty-state__art-dot empty-state__art-dot--b" />
      <i class="empty-state__art-dot empty-state__art-dot--c" />
    </div>
    <p class="empty-state__title">{{ title }}</p>
    <p v-if="description" class="empty-state__description">{{ description }}</p>
    <div v-if="$slots.default" class="empty-state__action">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.empty-state {
  display: grid;
  justify-items: center;
  gap: 0.5rem;
  padding: 2.5rem 1rem;
  color: var(--mcnp-text-muted);
  text-align: center;
}

.empty-state__art {
  position: relative;
  width: 3.5rem;
  height: 3.5rem;
  margin-bottom: 0.5rem;
  border: 1.5px dashed var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-surface-raised);
}

.empty-state__art-dot {
  position: absolute;
  width: 0.55rem;
  height: 0.55rem;
  border-radius: 50%;
}

.empty-state__art-dot--a {
  top: 0.7rem;
  left: 0.9rem;
  background: var(--mcnp-primary);
}

.empty-state__art-dot--b {
  top: 0.7rem;
  right: 0.9rem;
  background: var(--mcnp-accent);
}

.empty-state__art-dot--c {
  bottom: 0.7rem;
  left: 50%;
  background: var(--mcnp-success);
  transform: translateX(-50%);
}

.empty-state__title {
  margin: 0;
  color: var(--mcnp-text);
  font-size: 0.85rem;
  font-weight: 600;
}

.empty-state__description {
  margin: 0;
  max-width: 22rem;
  color: var(--mcnp-text-faint);
  font-size: 0.74rem;
  line-height: 1.5;
}

.empty-state__action {
  margin-top: 0.5rem;
}
</style>
