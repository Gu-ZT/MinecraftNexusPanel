<script setup lang="ts">
/** 已安装扩展列表（模组或插件）：版本列宽度保证「可更新」标签不折行。 */

import type { ExtensionInstall } from '@mcnp/api-client';
import { PermissionGate, formatRelative } from '@mcnp/ui';

defineProps<{
  installed: ExtensionInstall[];
  loading: boolean;
}>();

const emit = defineEmits<{ remove: [install: { id: string; name: string }] }>();
</script>

<template>
  <ATable :data="installed" :loading="loading" :pagination="false" row-key="id" :scroll="{ x: 940 }">
    <template #columns>
      <ATableColumn title="名称" data-index="name" :width="160" />
      <ATableColumn title="来源" :width="100">
        <template #cell="{ record }"><ATag size="small">{{ record.source }}</ATag></template>
      </ATableColumn>
      <ATableColumn title="版本" :width="220">
        <template #cell="{ record }">
          <span class="mono nowrap">{{ record.version }}</span>
          <ATag v-if="record.updateAvailable" size="small" color="orange" class="nowrap" style="margin-left: 6px">
            可更新 {{ record.updateAvailable }}
          </ATag>
        </template>
      </ATableColumn>
      <ATableColumn title="文件名" :width="260">
        <template #cell="{ record }"><span class="mono">{{ record.fileName }}</span></template>
      </ATableColumn>
      <ATableColumn title="安装时间" :width="110">
        <template #cell="{ record }">{{ formatRelative(record.installedAt) }}</template>
      </ATableColumn>
      <ATableColumn title="操作" :width="80" fixed="right">
        <template #cell="{ record }">
          <PermissionGate when="extension.manage">
            <AButton size="mini" status="danger" type="text" @click="emit('remove', record)">删除</AButton>
          </PermissionGate>
        </template>
      </ATableColumn>
    </template>
  </ATable>
</template>

<style scoped>
.nowrap {
  white-space: nowrap;
}
</style>
