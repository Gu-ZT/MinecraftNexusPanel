<script setup lang="ts">
/** 扩展内容源搜索：输入框 + 结果列表（安装按钮带权限闸门）。 */

import type { ExtensionProject } from '@mcnp/api-client';
import { PermissionGate } from '@mcnp/ui';
import { usePlatform } from '@/composables';

defineProps<{
  query: string;
  placeholder: string;
  results: ExtensionProject[];
  searching: boolean;
  installPending: boolean;
  isInstalled: (project: ExtensionProject) => boolean;
}>();

const emit = defineEmits<{
  'update:query': [value: string];
  search: [];
  install: [project: ExtensionProject];
}>();

const platform = usePlatform();
</script>

<template>
  <div>
    <div class="search-bar">
      <AInputSearch
        :model-value="query"
        :placeholder="placeholder"
        allow-clear
        @update:model-value="(v: string) => emit('update:query', v)"
        @search="() => emit('search')"
      />
    </div>
    <ASpin :loading="searching" style="width: 100%">
      <AList :data="results" :bordered="false">
        <AListItem v-for="project in results" :key="`${project.source}:${project.projectId}`">
          <AListItemMeta :title="project.name" :description="project.summary" />
          <template #extra>
            <ASpace>
              <ATag size="small">{{ project.source }}</ATag>
              <span class="text-secondary">{{ project.downloads.toLocaleString() }} 下载</span>
              <AButton size="mini" type="text" @click="platform.openExternal(project.url)">来源页</AButton>
              <PermissionGate when="extension.manage">
                <AButton
                  size="mini"
                  type="primary"
                  :disabled="isInstalled(project)"
                  :loading="installPending"
                  @click="emit('install', project)"
                >
                  {{ isInstalled(project) ? '已安装' : '安装' }}
                </AButton>
              </PermissionGate>
            </ASpace>
          </template>
        </AListItem>
      </AList>
    </ASpin>
  </div>
</template>

<style scoped>
.search-bar {
  margin-bottom: var(--mcnp-space-3);
  max-width: 420px;
}
</style>
