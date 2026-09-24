<script setup lang="ts">
/** 页面头部：标题、副标题与右侧操作区。 */
defineProps<{
  title: string;
  subtitle?: string;
}>();
</script>

<template>
  <div class="mcnp-page-header">
    <div class="mcnp-page-header__text">
      <h2 class="mcnp-page-header__title">{{ title }}</h2>
      <p v-if="subtitle" class="mcnp-page-header__subtitle">{{ subtitle }}</p>
    </div>
    <div v-if="$slots.extra" class="mcnp-page-header__extra">
      <slot name="extra" />
    </div>
  </div>
</template>

<style scoped>
.mcnp-page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--mcnp-space-4);
  margin-bottom: var(--mcnp-space-4);
}

.mcnp-page-header__title {
  margin: 0;
  font-size: 20px;
  color: var(--mcnp-text-primary);
}

.mcnp-page-header__subtitle {
  margin: var(--mcnp-space-1) 0 0;
  font-size: 13px;
  color: var(--mcnp-text-secondary);
}
</style>
