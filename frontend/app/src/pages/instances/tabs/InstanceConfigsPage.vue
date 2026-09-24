<script setup lang="ts">
/** 实例配置：识别后的结构化表单 + revision 乐观锁；未识别配置退回原始文本编辑。 */

import { computed, ref } from 'vue';
import { useRoute } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError, type ConfigDocument, type ConfigField } from '@mcnp/api-client';
import { formatRelative } from '@mcnp/ui';
import { useApi } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const route = useRoute();
const queryClient = useQueryClient();
const auth = useAuthStore();
const instanceId = computed(() => route.params.id as string);

const { data: docs, isLoading } = useQuery({
  queryKey: computed(() => ['configs', instanceId.value]),
  queryFn: () => api.configs.list(instanceId.value),
});

const activeDoc = ref<ConfigDocument | null>(null);
const activeFields = ref<ConfigField[]>([]);
const rawContent = ref('');
const rawMode = ref(false);

/** Arco Form 必需的 model：以当前字段键值构造。 */
const formModel = computed(() => Object.fromEntries(activeFields.value.map((f) => [f.key, f.value])));

async function openDoc(doc: ConfigDocument, forceRaw = false): Promise<void> {
  activeDoc.value = doc;
  rawMode.value = forceRaw || !doc.recognized;
  if (rawMode.value) {
    rawContent.value = await api.configs.readRaw(instanceId.value, doc.id);
  } else {
    activeFields.value = (await api.configs.fields(instanceId.value, doc.id)).map((f) => ({ ...f }));
  }
}

const saveMutation = useMutation({
  mutationFn: async () => {
    const doc = activeDoc.value;
    if (!doc) throw new ApiError(400, 'NO_DOC', '未选择配置');
    if (rawMode.value) {
      return api.configs.writeRaw(instanceId.value, doc.id, doc.revision, rawContent.value);
    }
    const changes: Record<string, unknown> = {};
    for (const field of activeFields.value) changes[field.key] = field.value;
    return api.configs.updateFields(instanceId.value, doc.id, doc.revision, changes);
  },
  onSuccess: () => {
    Message.success('配置已保存');
    const doc = activeDoc.value;
    if (doc) void openDoc(doc, rawMode.value);
    void queryClient.invalidateQueries({ queryKey: ['configs', instanceId.value] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '保存失败'),
});
</script>

<template>
  <div class="config-page">
    <div class="mcnp-card config-list">
      <ATable :data="docs ?? []" :loading="isLoading" :pagination="false" row-key="id" :scroll="{ x: 640 }">
        <template #columns>
          <ATableColumn title="文件" :width="260">
            <template #cell="{ record }"><span class="mono">{{ record.path }}</span></template>
          </ATableColumn>
          <ATableColumn title="格式" :width="110">
            <template #cell="{ record }">
              <ATag size="small">{{ record.format }}</ATag>
              <ATag v-if="!record.recognized" size="small" color="orange">未识别</ATag>
            </template>
          </ATableColumn>
          <ATableColumn title="更新" :width="110">
            <template #cell="{ record }">{{ formatRelative(record.updatedAt) }}</template>
          </ATableColumn>
          <ATableColumn title="操作" :width="160" fixed="right">
            <template #cell="{ record }">
              <ASpace>
                <AButton v-if="record.recognized" size="mini" @click="openDoc(record)">表单编辑</AButton>
                <AButton size="mini" @click="openDoc(record, true)">原始文本</AButton>
              </ASpace>
            </template>
          </ATableColumn>
        </template>
      </ATable>
    </div>

    <AModal
      :visible="activeDoc !== null"
      :title="activeDoc ? `${activeDoc.schemaTitle ?? activeDoc.path}（revision ${activeDoc.revision}）` : ''"
      width="680px"
      :ok-loading="saveMutation.isPending.value"
      :ok-button-props="{ disabled: !auth.has('config.write') }"
      @ok="saveMutation.mutate()"
      @cancel="activeDoc = null"
    >
      <template v-if="activeDoc && !rawMode">
        <AForm :model="formModel" layout="vertical">
          <AFormItem v-for="field in activeFields" :key="field.key" :label="field.title" :extra="field.description ?? undefined">
            <AInput v-if="field.type === 'string'" v-model="field.value as string" :disabled="!auth.has('config.write')" />
            <AInputNumber v-else-if="field.type === 'number'" v-model="field.value as number" :disabled="!auth.has('config.write')" />
            <ASwitch v-else-if="field.type === 'boolean'" v-model="field.value as boolean" :disabled="!auth.has('config.write')" />
            <ASelect v-else v-model="field.value as string" :disabled="!auth.has('config.write')">
              <AOption v-for="v in field.enumValues ?? []" :key="v" :value="v">{{ v }}</AOption>
            </ASelect>
          </AFormItem>
        </AForm>
      </template>
      <ATextarea
        v-else
        v-model="rawContent"
        class="mono"
        :auto-size="{ minRows: 14, maxRows: 22 }"
        :disabled="!auth.has('config.write')"
      />
    </AModal>
  </div>
</template>

<style scoped>
.config-page {
  display: flex;
  flex-direction: column;
  gap: var(--mcnp-space-4);
}
</style>
