<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useGitStore } from '../../stores/git';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { Project } from '../../types';

const props = defineProps<{
  project: Project;
}>();

const visible = defineModel<boolean>();
const { t } = useI18n();
const gitStore = useGitStore();

const selectedName = ref('');
const formName = ref('');
const formUrl = ref('');
const isEditing = ref(false);
const isSaving = ref(false);

const remoteRows = computed(() => {
  const source = gitStore.remotes[props.project.id] || [];
  const map = new Map<string, string>();
  for (const remote of source) {
    if (remote.remote_type === 'fetch' || !map.has(remote.name)) {
      map.set(remote.name, remote.url);
    }
  }
  return Array.from(map.entries()).map(([name, url]) => ({ name, url }));
});

watch(visible, async (v) => {
  if (v) {
    await gitStore.refreshRemotes(props.project.id, props.project.path);
    resetForm();
  }
});

function resetForm() {
  selectedName.value = '';
  formName.value = '';
  formUrl.value = '';
  isEditing.value = false;
}

function selectRow(name: string) {
  selectedName.value = name;
  const row = remoteRows.value.find(item => item.name === name);
  if (!row) return;
  formName.value = row.name;
  formUrl.value = row.url;
  isEditing.value = true;
}

async function handleSave() {
  const name = formName.value.trim();
  const url = formUrl.value.trim();
  if (!name || !url) return;

  isSaving.value = true;
  try {
    if (isEditing.value && selectedName.value) {
      if (name !== selectedName.value) {
        await gitStore.removeRemote(props.project.id, props.project.path, selectedName.value);
        await gitStore.addRemote(props.project.id, props.project.path, name, url);
      } else {
        await gitStore.updateRemoteUrl(props.project.id, props.project.path, name, url);
      }
      ElMessage.success(t('git.remoteUpdated'));
    } else {
      await gitStore.addRemote(props.project.id, props.project.path, name, url);
      ElMessage.success(t('git.remoteAdded'));
    }
    await gitStore.refreshRemotes(props.project.id, props.project.path);
    resetForm();
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  } finally {
    isSaving.value = false;
  }
}

async function handleRemove() {
  if (!selectedName.value) return;
  try {
    await ElMessageBox.confirm(
      t('git.remoteDeleteConfirm', { name: selectedName.value }),
      t('git.remoteDelete'),
      { confirmButtonText: t('common.confirm'), cancelButtonText: t('common.cancel'), type: 'warning' }
    );
    await gitStore.removeRemote(props.project.id, props.project.path, selectedName.value);
    ElMessage.success(t('git.remoteDeleted'));
    await gitStore.refreshRemotes(props.project.id, props.project.path);
    resetForm();
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(t('git.operationFailed', { error: String(e) }));
    }
  }
}
</script>

<template>
  <el-dialog
    v-model="visible"
    :title="t('git.repoSettings')"
    width="640px"
    :close-on-click-modal="false"
    @close="resetForm"
  >
    <div class="space-y-3">
      <div class="rounded border border-slate-200 dark:border-slate-600/40 overflow-hidden">
        <div class="grid grid-cols-[180px_1fr] text-[12px] font-medium bg-slate-100/80 dark:bg-slate-700/30 text-slate-600 dark:text-slate-300 px-3 py-1.5">
          <div>{{ t('git.remoteName') }}</div>
          <div>{{ t('git.remoteUrl') }}</div>
        </div>
        <div class="max-h-52 overflow-y-auto">
          <div
            v-for="row in remoteRows"
            :key="row.name"
            @click="selectRow(row.name)"
            class="grid grid-cols-[180px_1fr] px-3 py-1.5 text-[12px] cursor-pointer border-t border-slate-200/70 dark:border-slate-600/30"
            :class="selectedName === row.name ? 'bg-blue-500/10 dark:bg-blue-500/20' : 'hover:bg-slate-100/70 dark:hover:bg-slate-700/30'"
          >
            <div class="truncate text-slate-700 dark:text-slate-200">{{ row.name }}</div>
            <div class="truncate text-slate-500 dark:text-slate-400">{{ row.url }}</div>
          </div>
          <div v-if="remoteRows.length === 0" class="px-3 py-6 text-center text-[12px] text-slate-400">
            {{ t('git.noRemotes') }}
          </div>
        </div>
      </div>

      <div class="grid grid-cols-1 gap-2">
        <el-input v-model="formName" :placeholder="t('git.remoteNamePlaceholder')" />
        <el-input v-model="formUrl" :placeholder="t('git.remoteUrlPlaceholder')" />
      </div>

      <div class="flex items-center justify-between">
        <div class="text-[12px] text-slate-400">{{ t('git.multiRemoteHint') }}</div>
        <div class="flex items-center gap-2">
          <el-button @click="resetForm">{{ t('common.cancel') }}</el-button>
          <el-button type="danger" :disabled="!selectedName" @click="handleRemove">{{ t('git.remoteDelete') }}</el-button>
          <el-button type="primary" :loading="isSaving" :disabled="!formName.trim() || !formUrl.trim()" @click="handleSave">
            {{ isEditing ? t('git.remoteUpdate') : t('git.remoteAdd') }}
          </el-button>
        </div>
      </div>
    </div>
  </el-dialog>
</template>
