<script setup lang="ts">
/**
 * 管理员初始化页：仅在 Panel 无任何用户时可达（M1 链路第一步）。
 * Mock 种子数据自带用户，正常流程不会进入本页。
 */

import { reactive, ref } from 'vue';
import { useRouter } from 'vue-router';
import { Message } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { useApi, usePlatform } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const platform = usePlatform();
const auth = useAuthStore();
const router = useRouter();

const form = reactive({ username: '', displayName: '', password: '', confirm: '' });
const loading = ref(false);

async function submit(): Promise<void> {
  if (form.password !== form.confirm) {
    Message.warning('两次输入的密码不一致');
    return;
  }
  loading.value = true;
  try {
    const result = await api.auth.setupAdmin({
      username: form.username,
      displayName: form.displayName || form.username,
      password: form.password,
    });
    auth.applyLogin(result, platform);
    Message.success('初始化完成');
    void router.push('/');
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '初始化失败');
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="setup-page">
    <div class="setup-card mcnp-card">
      <h1 class="setup-title">初始化管理员</h1>
      <p class="setup-subtitle">首次运行需要创建管理员账户</p>
      <AForm :model="form" layout="vertical" @submit-success="submit">
        <AFormItem label="用户名"><AInput v-model="form.username" /></AFormItem>
        <AFormItem label="显示名"><AInput v-model="form.displayName" /></AFormItem>
        <AFormItem label="密码"><AInputPassword v-model="form.password" /></AFormItem>
        <AFormItem label="确认密码"><AInputPassword v-model="form.confirm" /></AFormItem>
        <AButton type="primary" long html-type="submit" :loading="loading">创建并登录</AButton>
      </AForm>
    </div>
  </div>
</template>

<style scoped>
.setup-page {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
}

.setup-card {
  width: 360px;
}

.setup-title {
  margin: 0;
  font-size: 20px;
  text-align: center;
}

.setup-subtitle {
  margin: var(--mcnp-space-1) 0 var(--mcnp-space-6);
  text-align: center;
  color: var(--mcnp-text-secondary);
  font-size: 13px;
}
</style>
