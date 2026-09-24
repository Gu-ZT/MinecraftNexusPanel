<script setup lang="ts">
/** 一键搭建向导：模板 → 版本与环境 → 基本设置 → 确认（M2 链路原型）。 */

import { computed, reactive, ref } from 'vue';
import { useRouter } from 'vue-router';
import { useMutation, useQuery } from '@tanstack/vue-query';
import { Message } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { PageHeader } from '@mcnp/ui';
import { useApi } from '@/composables';

const api = useApi();
const router = useRouter();

const step = ref(0);

const form = reactive({
  coreId: '',
  templateId: '',
  version: '',
  javaRuntimeId: '',
  supervisorMode: 'DIRECT' as 'DIRECT' | 'MCDR',
  name: '',
  workDir: '',
  expiresInDays: 0,
});

const { data: cores } = useQuery({ queryKey: ['cores'], queryFn: () => api.cores.list() });
const { data: templates } = useQuery({ queryKey: ['templates'], queryFn: () => api.templates.list() });

const template = computed(() => templates.value?.find((t) => t.id === form.templateId) ?? null);

// 选定节点后加载其受管 Java 运行时
const { data: runtimes } = useQuery({
  queryKey: computed(() => ['runtimes', form.coreId]),
  queryFn: () => api.environments.list(form.coreId),
  enabled: computed(() => form.coreId !== ''),
});

const javaRuntimes = computed(() =>
  (runtimes.value ?? []).filter(
    (r) => r.kind === 'JAVA' && (!template.value || r.majorVersion >= template.value.requiredRuntime.minMajor),
  ),
);

function pickTemplate(templateId: string): void {
  form.templateId = templateId;
  form.version = '';
  const t = templates.value?.find((x) => x.id === templateId);
  if (t && !form.name) form.name = t.name;
  step.value = 1;
}

const canNext = computed(() => {
  if (step.value === 1) return form.coreId !== '' && form.version !== '' && form.javaRuntimeId !== '';
  if (step.value === 2) return form.name.trim() !== '' && form.workDir.trim() !== '';
  return true;
});

const createMutation = useMutation({
  mutationFn: () => {
    const t = template.value;
    return api.instances.create({
      coreId: form.coreId,
      name: form.name,
      templateId: form.templateId || null,
      serverType: t?.serverType ?? 'CUSTOM',
      version: form.version,
      javaRuntimeId: form.javaRuntimeId || null,
      workDir: form.workDir,
      supervisorMode: form.supervisorMode,
      expiresAt: form.expiresInDays > 0 ? Date.now() + form.expiresInDays * 86_400_000 : null,
    });
  },
  onSuccess: ({ instanceId }) => {
    Message.success('创建任务已提交，可在任务中心查看进度');
    void router.push(`/instances/${instanceId}/console`);
  },
  onError: (err) => Message.error(err instanceof ApiError ? err.message : '创建失败'),
});
</script>

<template>
  <div>
    <PageHeader title="一键搭建" subtitle="由安装模板完成服务端下载、校验与默认配置（可审计异步任务）" />

    <div class="mcnp-card wizard">
      <ASteps :current="step + 1" style="margin-bottom: 24px">
        <AStep>选择模板</AStep>
        <AStep>版本与环境</AStep>
        <AStep>基本设置</AStep>
        <AStep>确认</AStep>
      </ASteps>

      <!-- 步骤 1：模板 -->
      <div v-if="step === 0" class="template-grid">
        <div
          v-for="t in templates ?? []"
          :key="t.id"
          class="template-card"
          :class="{ 'template-card--active': form.templateId === t.id }"
          @click="pickTemplate(t.id)"
        >
          <div class="template-card__name">{{ t.name }}</div>
          <div class="template-card__desc">{{ t.description }}</div>
          <div class="template-card__meta">需要 Java {{ t.requiredRuntime.minMajor }}+{{ t.supportsMcdr ? ' · 支持 MCDR' : '' }}</div>
        </div>
      </div>

      <!-- 步骤 2：版本与环境 -->
      <AForm v-else-if="step === 1" :model="form" layout="vertical" class="wizard-form">
        <AFormItem label="目标节点" required>
          <ASelect v-model="form.coreId" placeholder="选择 Core 节点">
            <AOption v-for="core in cores ?? []" :key="core.id" :value="core.id" :disabled="core.status === 'OFFLINE'">
              {{ core.name }}（{{ core.address }}）
            </AOption>
          </ASelect>
        </AFormItem>
        <AFormItem label="服务端版本" required>
          <ASelect v-model="form.version" placeholder="选择版本">
            <AOption v-for="v in template?.versions ?? []" :key="v" :value="v">{{ v }}</AOption>
          </ASelect>
        </AFormItem>
        <AFormItem label="Java 运行时" required extra="仅列出满足模板最低版本要求的受管运行时">
          <ASelect v-model="form.javaRuntimeId" placeholder="选择 Java 版本" :disabled="!form.coreId">
            <AOption v-for="rt in javaRuntimes" :key="rt.id" :value="rt.id">Java {{ rt.version }}（{{ rt.path }}）</AOption>
          </ASelect>
        </AFormItem>
        <AFormItem v-if="template?.supportsMcdr" label="进程包装">
          <ARadioGroup v-model="form.supervisorMode">
            <ARadio value="DIRECT">直接启动</ARadio>
            <ARadio value="MCDR">MCDR 包装</ARadio>
          </ARadioGroup>
        </AFormItem>
      </AForm>

      <!-- 步骤 3：基本设置 -->
      <AForm v-else-if="step === 2" :model="form" layout="vertical" class="wizard-form">
        <AFormItem label="实例名称" required><AInput v-model="form.name" /></AFormItem>
        <AFormItem label="工作目录" required><AInput v-model="form.workDir" placeholder="/opt/mc/servers/my-server" /></AFormItem>
        <AFormItem label="到期时间" extra="0 表示永不到期（社区版可留空）">
          <AInputNumber v-model="form.expiresInDays" :min="0" placeholder="天数" /> 天后到期
        </AFormItem>
      </AForm>

      <!-- 步骤 4：确认 -->
      <div v-else class="wizard-confirm">
        <ADescriptions :column="1" bordered>
          <ADescriptionsItem label="模板">{{ template?.name ?? '空白实例' }}</ADescriptionsItem>
          <ADescriptionsItem label="版本">{{ form.version }}</ADescriptionsItem>
          <ADescriptionsItem label="节点">{{ cores?.find((c) => c.id === form.coreId)?.name }}</ADescriptionsItem>
          <ADescriptionsItem label="实例名称">{{ form.name }}</ADescriptionsItem>
          <ADescriptionsItem label="工作目录"><span class="mono">{{ form.workDir }}</span></ADescriptionsItem>
          <ADescriptionsItem label="进程包装">{{ form.supervisorMode }}</ADescriptionsItem>
        </ADescriptions>
      </div>

      <div class="wizard-actions">
        <AButton v-if="step > 0" @click="step -= 1">上一步</AButton>
        <AButton v-if="step < 3" type="primary" :disabled="step > 0 && !canNext" @click="step += 1">下一步</AButton>
        <AButton v-else type="primary" :loading="createMutation.isPending.value" @click="createMutation.mutate()">
          创建实例
        </AButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.wizard {
  max-width: 760px;
}

.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: var(--mcnp-space-3);
}

.template-card {
  padding: var(--mcnp-space-3);
  border: 1px solid rgb(255 255 255 / 10%);
  border-radius: var(--mcnp-radius-m);
  cursor: pointer;
  transition: border-color 0.15s;
}

.template-card:hover,
.template-card--active {
  border-color: var(--mcnp-color-primary);
}

.template-card__name {
  font-weight: 600;
  margin-bottom: var(--mcnp-space-1);
}

.template-card__desc {
  font-size: 12px;
  color: var(--mcnp-text-secondary);
  min-height: 36px;
}

.template-card__meta {
  font-size: 12px;
  color: var(--mcnp-text-tertiary);
  margin-top: var(--mcnp-space-2);
}

.wizard-form {
  max-width: 460px;
}

.wizard-actions {
  display: flex;
  gap: var(--mcnp-space-3);
  margin-top: var(--mcnp-space-6);
}
</style>
