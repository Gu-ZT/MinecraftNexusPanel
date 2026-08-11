<script setup lang="ts">
/*
 * SkeletonBlock：骨架屏占位块（docs/design/frontend-design.md §5）。
 * 尺寸由 props 控制；prefers-reduced-motion 下呼吸动画静止。
 */
withDefaults(
  defineProps<{
    width?: string;
    height?: string;
    radius?: string;
  }>(),
  { width: '100%', height: '1rem', radius: '6px' },
);
</script>

<template>
  <span class="skeleton-block" :style="{ width, height, borderRadius: radius }" aria-hidden="true" />
</template>

<style scoped>
.skeleton-block {
  display: inline-block;
  background: linear-gradient(
    90deg,
    var(--mcnp-surface-raised) 25%,
    var(--mcnp-surface-hover) 50%,
    var(--mcnp-surface-raised) 75%
  );
  background-size: 200% 100%;
  animation: skeleton-shimmer 1.4s ease-in-out infinite;
}

@keyframes skeleton-shimmer {
  from {
    background-position: 200% 0;
  }
  to {
    background-position: -200% 0;
  }
}

@media (prefers-reduced-motion: reduce) {
  .skeleton-block {
    animation: none;
  }
}
</style>
