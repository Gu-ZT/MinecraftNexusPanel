<script setup lang="ts">
/** 登录页。Mock 账户：admin/admin123、operator/operator123、viewer/viewer123。 */

import { reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Message } from '@arco-design/web-vue';
import { ApiError } from '@mcnp/api-client';
import { useApi, usePlatform } from '@/composables';
import { useAuthStore } from '@/stores/auth';

const api = useApi();
const platform = usePlatform();
const auth = useAuthStore();
const route = useRoute();
const router = useRouter();

const form = reactive({ username: '', password: '' });
const loading = ref(false);

async function submit(): Promise<void> {
  if (!form.username || !form.password) {
    Message.warning('请输入用户名和密码');
    return;
  }
  loading.value = true;
  try {
    const result = await api.auth.login({
      username: form.username,
      password: form.password,
      deviceName: `${platform.appInfo().name} / 浏览器`,
    });
    auth.applyLogin(result, platform);
    Message.success(`欢迎回来，${result.user.displayName}`);
    void router.push((route.query.redirect as string) ?? '/');
  } catch (err) {
    Message.error(err instanceof ApiError ? err.message : '登录失败');
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="login-page">
    <div class="login-card mcnp-card">
      <h1 class="login-title">Minecraft Nexus Panel</h1>
      <p class="login-subtitle">多节点 Minecraft 服务器管理面板</p>
      <AForm :model="form" layout="vertical" @submit-success="submit">
        <AFormItem label="用户名">
          <AInput v-model="form.username" placeholder="admin" allow-clear />
        </AFormItem>
        <AFormItem label="密码">
          <AInputPassword v-model="form.password" placeholder="admin123" @press-enter="submit" />
        </AFormItem>
        <AButton type="primary" long html-type="submit" :loading="loading">登录</AButton>
      </AForm>
      <p class="login-hint">原型 Mock 账户：admin / operator / viewer，密码均为「用户名 + 123」</p>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
}

.login-card {
  width: 360px;
}

.login-title {
  margin: 0;
  font-size: 20px;
  text-align: center;
}

.login-subtitle {
  margin: var(--mcnp-space-1) 0 var(--mcnp-space-6);
  text-align: center;
  color: var(--mcnp-text-secondary);
  font-size: 13px;
}

.login-hint {
  margin-top: var(--mcnp-space-4);
  font-size: 12px;
  color: var(--mcnp-text-tertiary);
  text-align: center;
}
</style>
