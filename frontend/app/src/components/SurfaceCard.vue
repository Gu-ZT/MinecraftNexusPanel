<script setup lang="ts">
/*
 * SurfaceCard：通用内容卡片容器（docs/design/frontend-design.md §5）。
 * 只负责容器风格（实色底、描边、圆角、可选悬停上浮），业务内容走默认插槽。
 */
withDefaults(
  defineProps<{
    /** 悬停时轻微上浮，用于可点击/可聚焦的卡片 */
    hover?: boolean;
  }>(),
  { hover: false },
);
</script>

<template>
  <section class="surface-card" :class="{ 'surface-card--hover': hover }">
    <slot />
  </section>
</template>

<style scoped>
.surface-card {
  border: 1px solid var(--mcnp-border);
  border-radius: var(--mcnp-radius);
  background: var(--mcnp-surface);
  box-shadow: var(--mcnp-shadow);
  transition:
    transform 160ms ease-out,
    box-shadow 160ms ease-out,
    border-color 160ms ease-out;
}

.surface-card--hover:hover {
  border-color: var(--mcnp-border-subtle);
  box-shadow: var(--mcnp-shadow-hover);
  transform: translateY(-1px);
}
</style>
