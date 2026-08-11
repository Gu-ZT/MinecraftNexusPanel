<script setup lang="ts">
/*
 * PageHeader：页面头部（docs/design/frontend-design.md §4、§5）。
 * 眉题 + 标题 + 说明，右侧动作通过默认插槽放入。
 */
defineProps<{
  /** 眉题，主色小号加粗 */
  eyebrow?: string;
  title: string;
  /** 辅助说明 */
  hint?: string;
}>();
</script>

<template>
  <header class="page-header">
    <div class="page-header__heading">
      <p v-if="eyebrow" class="page-eyebrow">{{ eyebrow }}</p>
      <h1>{{ title }}</h1>
      <p v-if="hint" class="page-header__hint">{{ hint }}</p>
    </div>
    <div v-if="$slots.default" class="page-header__actions">
      <slot />
    </div>
  </header>
</template>

<style scoped>
.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1.1rem;
}

.page-header__heading {
  display: grid;
  gap: 0.3rem;
}

.page-header h1 {
  margin: 0;
  color: var(--mcnp-text);
  font-size: 1.35rem;
  font-weight: 650;
  line-height: 1.3;
}

.page-header__hint {
  margin: 0;
  max-width: 36rem;
  color: var(--mcnp-text-faint);
  font-size: 0.75rem;
  line-height: 1.5;
}

.page-header__actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 0.5rem;
}

@media (max-width: 44rem) {
  .page-header {
    flex-direction: column;
  }

  .page-header__actions {
    width: 100%;
  }
}
</style>
