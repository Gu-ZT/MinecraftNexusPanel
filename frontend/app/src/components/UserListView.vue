<script setup lang="ts">
import {
  Button as AButton,
  Checkbox as ACheckbox,
  Empty as AEmpty,
  Input as AInput,
  InputPassword as AInputPassword,
  Modal as AModal,
  Popconfirm as APopconfirm,
  Spin as ASpin,
} from '@arco-design/web-vue';
import { IconDelete, IconPlus, IconRefresh, IconSafe, IconUserGroup } from '@arco-design/web-vue/es/icon';
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';

import type { PanelApiClient, User } from '@mcnp/api-client';

import { describeError } from '../utils/presentation';

const props = defineProps<{
  client: PanelApiClient;
  currentUser: User;
}>();

const { t } = useI18n();
const users = ref<User[]>([]);
const loading = ref(false);
const createVisible = ref(false);
const createPending = ref(false);
const actionPending = ref('');
const username = ref('');
const displayName = ref('');
const password = ref('');
const grantAuditRead = ref(false);
const errorMessage = ref('');
const noticeMessage = ref('');

onMounted(() => void loadUsers());

async function loadUsers(): Promise<void> {
  loading.value = true;
  errorMessage.value = '';
  try {
    users.value = (await props.client.listUsers()).items;
  } catch (error) {
    errorMessage.value = describeError(error, t('users.loadError'));
  } finally {
    loading.value = false;
  }
}

function openCreate(): void {
  username.value = '';
  displayName.value = '';
  password.value = '';
  grantAuditRead.value = false;
  errorMessage.value = '';
  createVisible.value = true;
}

async function createUser(): Promise<void> {
  createPending.value = true;
  errorMessage.value = '';
  try {
    await props.client.createUser({
      username: username.value.trim(),
      displayName: displayName.value.trim(),
      password: password.value,
      permissions: grantAuditRead.value ? ['audit.read'] : [],
    });
    createVisible.value = false;
    noticeMessage.value = t('users.created');
    await loadUsers();
  } catch (error) {
    errorMessage.value = describeError(error, t('users.createError'));
  } finally {
    createPending.value = false;
  }
}

async function setAuditRead(
  user: User,
  enabled: boolean | (string | number | boolean)[],
): Promise<void> {
  if (typeof enabled !== 'boolean') {
    return;
  }
  actionPending.value = `permission:${user.id}`;
  errorMessage.value = '';
  try {
    const permissions = enabled ? ['audit.read'] : [];
    const updated = await props.client.updateUser(user.id, { permissions });
    users.value = users.value.map((item) => (item.id === updated.id ? updated : item));
    noticeMessage.value = t('users.updated');
  } catch (error) {
    errorMessage.value = describeError(error, t('users.updateError'));
  } finally {
    actionPending.value = '';
  }
}

async function deleteUser(user: User): Promise<void> {
  actionPending.value = `delete:${user.id}`;
  errorMessage.value = '';
  try {
    await props.client.deleteUser(user.id);
    users.value = users.value.filter((item) => item.id !== user.id);
    noticeMessage.value = t('users.deleted');
  } catch (error) {
    errorMessage.value = describeError(error, t('users.deleteError'));
  } finally {
    actionPending.value = '';
  }
}
</script>

<template>
  <main class="console-page">
    <header class="page-heading page-heading--toolbar">
      <div>
        <p class="page-eyebrow"><IconUserGroup /> {{ t('users.eyebrow') }}</p>
        <h1>{{ t('users.title') }}</h1>
      </div>
      <div class="user-page-actions">
        <a-button :loading="loading" @click="loadUsers">
          <template #icon><IconRefresh /></template>
          {{ t('common.refresh') }}
        </a-button>
        <a-button type="primary" @click="openCreate">
          <template #icon><IconPlus /></template>
          {{ t('users.create') }}
        </a-button>
      </div>
    </header>

    <p v-if="errorMessage" class="form-error" role="alert">{{ errorMessage }}</p>
    <p v-else-if="noticeMessage" class="notice" role="status">{{ noticeMessage }}</p>

    <section class="data-panel user-panel">
      <a-spin :loading="loading">
        <div v-if="users.length" class="data-table-wrap">
          <table class="data-table user-table">
            <thead>
              <tr>
                <th>{{ t('users.account') }}</th>
                <th>{{ t('users.displayName') }}</th>
                <th>{{ t('users.auditRead') }}</th>
                <th>{{ t('common.actions') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="user in users" :key="user.id">
                <td><strong>{{ user.username }}</strong><small>{{ user.id }}</small></td>
                <td>{{ user.displayName }}</td>
                <td>
                  <a-checkbox
                    :model-value="user.permissions.includes('audit.read')"
                    :disabled="user.id === currentUser.id || actionPending === `permission:${user.id}`"
                    @change="(enabled) => setAuditRead(user, enabled)"
                  >
                    <IconSafe /> {{ t('users.auditRead') }}
                  </a-checkbox>
                </td>
                <td>
                  <a-popconfirm
                    :content="t('users.deleteConfirm', { name: user.username })"
                    :ok-text="t('common.delete')"
                    :cancel-text="t('common.cancel')"
                    @ok="deleteUser(user)"
                  >
                    <a-button
                      type="text"
                      status="danger"
                      :loading="actionPending === `delete:${user.id}`"
                      :disabled="user.id === currentUser.id"
                      :aria-label="t('common.delete')"
                    >
                      <template #icon><IconDelete /></template>
                    </a-button>
                  </a-popconfirm>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <a-empty v-else :description="t('users.empty')" />
      </a-spin>
    </section>

    <a-modal v-model:visible="createVisible" :title="t('users.createTitle')" :footer="false" unmount-on-close>
      <form class="user-form" @submit.prevent="createUser">
        <label><span>{{ t('auth.username') }}</span><a-input v-model="username" allow-clear /></label>
        <label><span>{{ t('users.displayName') }}</span><a-input v-model="displayName" allow-clear /></label>
        <label><span>{{ t('auth.password') }}</span><a-input-password v-model="password" allow-clear /></label>
        <a-checkbox v-model="grantAuditRead"><IconSafe /> {{ t('users.auditRead') }}</a-checkbox>
        <div class="user-form__actions">
          <a-button @click="createVisible = false">{{ t('common.cancel') }}</a-button>
          <a-button
            type="primary"
            html-type="submit"
            :loading="createPending"
            :disabled="!username.trim() || !displayName.trim() || password.length < 12"
          >
            {{ t('users.create') }}
          </a-button>
        </div>
      </form>
    </a-modal>
  </main>
</template>

<style scoped>
.user-page-actions,
.user-form__actions {
  display: flex;
  gap: 0.6rem;
}

.user-panel {
  margin-top: 0.8rem;
}

.user-table td:first-child {
  display: grid;
  min-width: 13rem;
  gap: 0.2rem;
}

.user-table td:last-child {
  width: 4rem;
  text-align: right;
}

.user-table strong {
  color: var(--mcnp-text);
}

.user-table small {
  color: var(--mcnp-text-faint);
  overflow-wrap: anywhere;
}

.user-form {
  display: grid;
  gap: 1rem;
}

.user-form label {
  display: grid;
  gap: 0.4rem;
  color: var(--mcnp-text-muted);
  font-size: 0.78rem;
  font-weight: 600;
}

.user-form__actions {
  justify-content: flex-end;
  padding-top: 0.25rem;
}

@media (max-width: 40rem) {
  .page-heading--toolbar {
    align-items: flex-start;
  }

  .user-page-actions {
    width: 100%;
  }

  .user-page-actions > * {
    flex: 1;
  }
}
</style>
