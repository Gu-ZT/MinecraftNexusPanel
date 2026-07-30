<script setup lang="ts">
import { McnpEmptyState } from '@mcnp/ui';
</script>

<template>
  <div class="workspace">
    <header class="topbar">
      <strong>MCNP</strong>
      <span>Core 管理</span>
    </header>
    <main>
      <McnpEmptyState title="尚未连接 Core" description="添加 Core 后，实例会显示在这里。" />
    </main>
  </div>
</template>

<style scoped>
.workspace {
  min-height: 100vh;
}

.topbar {
  display: flex;
  align-items: center;
  min-height: 3.5rem;
  padding: 0 1.25rem;
  border-bottom: 1px solid #d7dcd8;
  background: #ffffff;
}

.topbar strong {
  color: #206b3a;
  font-size: 1rem;
  letter-spacing: 0;
}

.topbar span {
  margin-left: 1rem;
  color: #637068;
  font-size: 0.875rem;
}

main {
  display: grid;
  min-height: calc(100vh - 3.5rem);
  place-items: center;
}

</style>
