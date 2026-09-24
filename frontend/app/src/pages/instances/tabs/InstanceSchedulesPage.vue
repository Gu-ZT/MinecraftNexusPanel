<script setup lang="ts">
/** 计划任务：Cron（带时区）与领域事件触发，执行历史抽屉。 */

import { computed, reactive, ref } from 'vue';
import { useRoute } from 'vue-router';
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError, type Schedule, type ScheduleEvent } from '@mcnp/api-client';
import { PermissionGate, formatTime } from '@mcnp/ui';
import { useApi } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const route = useRoute();
const queryClient = useQueryClient();
const auth = useAuthStore();
const authHasManage = computed(() => auth.has('schedule.manage'));
const instanceId = computed(() => route.params.id as string);

const { data: schedules, isLoading } = useQuery({
  queryKey: computed(() => ['schedules', instanceId.value]),
  queryFn: () => api.schedules.list(instanceId.value),
});

const editorVisible = ref(false);
const editingId = ref<string | null>(null);
const form = reactive({
  name: '',
  kind: 'CRON' as 'CRON' | 'EVENT',
  cron: '0 5 * * *',
  timezone: 'Asia/Shanghai',
  event: 'INSTANCE_STARTED' as ScheduleEvent,
  action: 'restart',
  enabled: true,
});

const EVENT_OPTIONS: { value: ScheduleEvent; label: string }[] = [
  { value: 'INSTANCE_STARTED', label: '实例启动完成' },
  { value: 'INSTANCE_STOPPED', label: '实例停止' },
  { value: 'PLAYER_COUNT_ABOVE', label: '玩家数超过阈值' },
  { value: 'EXIT_CODE_NONZERO', label: '异常退出（非零退出码）' },
];

function openEditor(schedule?: Schedule): void {
  editingId.value = schedule?.id ?? null;
  form.name = schedule?.name ?? '';
  form.kind = schedule?.trigger.kind ?? 'CRON';
  form.cron = schedule?.trigger.kind === 'CRON' ? schedule.trigger.cron : '0 5 * * *';
  form.timezone = schedule?.trigger.kind === 'CRON' ? schedule.trigger.timezone : 'Asia/Shanghai';
  form.event = schedule?.trigger.kind === 'EVENT' ? schedule.trigger.event : 'INSTANCE_STARTED';
  form.action = schedule?.action ?? 'restart';
  form.enabled = schedule?.enabled ?? true;
  editorVisible.value = true;
}

const saveMutation = useMutation({
  mutationFn: () => {
    const input = {
      name: form.name,
      trigger:
        form.kind === 'CRON'
          ? ({ kind: 'CRON', cron: form.cron, timezone: form.timezone } as const)
          : ({ kind: 'EVENT', event: form.event } as const),
      action: form.action,
      enabled: form.enabled,
    };
    return editingId.value
      ? api.schedules.update(instanceId.value, editingId.value, input)
      : api.schedules.create(instanceId.value, input);
  },
  onSuccess: () => {
    Message.success('已保存');
    editorVisible.value = false;
    void queryClient.invalidateQueries({ queryKey: ['schedules', instanceId.value] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '保存失败'),
});

const removeMutation = useMutation({
  mutationFn: (scheduleId: string) => api.schedules.remove(instanceId.value, scheduleId),
  onSuccess: () => {
    Message.success('已删除');
    void queryClient.invalidateQueries({ queryKey: ['schedules', instanceId.value] });
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '删除失败'),
});

const historyOf = ref<string | null>(null);
const { data: executions } = useQuery({
  queryKey: computed(() => ['schedule-executions', historyOf.value]),
  queryFn: () => api.schedules.executions(historyOf.value ?? ''),
  enabled: computed(() => historyOf.value !== null),
});

async function toggleEnabled(scheduleId: string, enabled: boolean): Promise<void> {
  try {
    await api.schedules.update(instanceId.value, scheduleId, { enabled });
    void queryClient.invalidateQueries({ queryKey: ['schedules', instanceId.value] });
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '更新失败');
  }
}

function triggerLabel(schedule: Schedule): string {
  return schedule.trigger.kind === 'CRON'
    ? `Cron ${schedule.trigger.cron}（${schedule.trigger.timezone}）`
    : `事件：${EVENT_OPTIONS.find((e) => e.value === (schedule.trigger as { event: ScheduleEvent }).event)?.label ?? ''}`;
}
</script>

<template>
  <div class="mcnp-card">
    <div class="toolbar">
      <PermissionGate when="schedule.manage">
        <AButton type="primary" @click="openEditor()">新建计划</AButton>
      </PermissionGate>
    </div>

    <ATable :data="schedules ?? []" :loading="isLoading" :pagination="false" row-key="id" :scroll="{ x: 1120 }">
      <template #columns>
        <ATableColumn title="名称" data-index="name" :width="160" />
        <ATableColumn title="触发条件" :width="200">
          <template #cell="{ record }"><span class="mono">{{ triggerLabel(record) }}</span></template>
        </ATableColumn>
        <ATableColumn title="动作" data-index="action" :width="160" />
        <ATableColumn title="启用" :width="80">
          <template #cell="{ record }">
            <ASwitch
              :model-value="record.enabled"
              :disabled="!authHasManage"
              @change="(v) => toggleEnabled(record.id, Boolean(v))"
            />
          </template>
        </ATableColumn>
        <ATableColumn title="上次执行" :width="160">
          <template #cell="{ record }">{{ formatTime(record.lastRunAt) }}</template>
        </ATableColumn>
        <ATableColumn title="下次执行" :width="160">
          <template #cell="{ record }">{{ formatTime(record.nextRunAt) }}</template>
        </ATableColumn>
        <ATableColumn title="操作" :width="180" fixed="right">
          <template #cell="{ record }">
            <ASpace>
              <AButton size="mini" @click="historyOf = record.id">历史</AButton>
              <PermissionGate when="schedule.manage">
                <AButton size="mini" @click="openEditor(record)">编辑</AButton>
                <AButton size="mini" status="danger" type="text" @click="removeMutation.mutate(record.id)">删除</AButton>
              </PermissionGate>
            </ASpace>
          </template>
        </ATableColumn>
      </template>
    </ATable>

    <AModal v-model:visible="editorVisible" :title="editingId ? '编辑计划' : '新建计划'" :ok-loading="saveMutation.isPending.value" @ok="saveMutation.mutate()">
      <AForm :model="form" layout="vertical">
        <AFormItem label="名称" required><AInput v-model="form.name" /></AFormItem>
        <AFormItem label="触发方式">
          <ARadioGroup v-model="form.kind">
            <ARadio value="CRON">Cron 定时</ARadio>
            <ARadio value="EVENT">实例事件</ARadio>
          </ARadioGroup>
        </AFormItem>
        <template v-if="form.kind === 'CRON'">
          <AFormItem label="Cron 表达式"><AInput v-model="form.cron" class="mono" /></AFormItem>
          <AFormItem label="时区"><AInput v-model="form.timezone" class="mono" /></AFormItem>
        </template>
        <AFormItem v-else label="事件">
          <ASelect v-model="form.event">
            <AOption v-for="opt in EVENT_OPTIONS" :key="opt.value" :value="opt.value">{{ opt.label }}</AOption>
          </ASelect>
        </AFormItem>
        <AFormItem label="动作"><AInput v-model="form.action" class="mono" placeholder="restart / backup / command:say hi" /></AFormItem>
        <AFormItem label="启用"><ASwitch v-model="form.enabled" /></AFormItem>
      </AForm>
    </AModal>

    <ADrawer :visible="historyOf !== null" title="执行历史" :width="420" @cancel="historyOf = null">
      <ATimeline>
        <ATimelineItem v-for="exec in executions ?? []" :key="exec.id">
          <div>
            <ATag :color="exec.result === 'SUCCESS' ? 'green' : exec.result === 'FAILED' ? 'red' : 'gray'" size="small">
              {{ exec.result }}
            </ATag>
            <span style="margin-left: 8px">{{ formatTime(exec.startedAt) }}</span>
          </div>
          <p v-if="exec.message" class="text-secondary" style="margin: 4px 0 0">{{ exec.message }}</p>
        </ATimelineItem>
      </ATimeline>
      <AEmpty v-if="(executions ?? []).length === 0" description="暂无执行记录" />
    </ADrawer>
  </div>
</template>

<style scoped>
.toolbar {
  margin-bottom: var(--mcnp-space-3);
}
</style>
