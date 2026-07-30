<script setup lang="ts">
defineProps<{
  title: string;
  description?: string;
}>();
</script>

<template>
  <section class="empty-state" aria-live="polite">
    <div class="status-mark" aria-hidden="true"></div>
    <h1>{{ title }}</h1>
    <p v-if="description">{{ description }}</p>
  </section>
</template>

<style scoped>
.empty-state {
  display: grid;
  justify-items: center;
  max-width: 36rem;
  padding: 3rem 1.5rem;
  text-align: center;
}

.status-mark {
  width: 2.75rem;
  height: 2.75rem;
  margin-bottom: 1.25rem;
  border: 1px solid #a8b0aa;
  border-radius: 6px;
  background: #2f7d4a;
}

h1 {
  margin: 0;
  color: #18201b;
  font-size: 1.25rem;
  font-weight: 650;
  letter-spacing: 0;
}

p {
  margin: 0.65rem 0 0;
  color: #637068;
  line-height: 1.6;
}
</style>
