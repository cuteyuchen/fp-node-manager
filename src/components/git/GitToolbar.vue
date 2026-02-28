<script setup lang="ts">
import { computed } from 'vue';
import { useGitStore } from '../../stores/git';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { Project } from '../../types';

const props = defineProps<{
  project: Project;
}>();

const emit = defineEmits<{
  (e: 'open-branch-dialog', mode: 'create' | 'delete', branch?: string): void;
  (e: 'open-stash'): void;
  (e: 'refresh'): void;
  (e: 'open-remote-settings'): void;
}>();

const { t } = useI18n();
const gitStore = useGitStore();

const currentBranch = computed(() => gitStore.getCurrentBranch(props.project.id));

const currentBranchInfo = computed(() => {
  const branches = gitStore.getBranches(props.project.id);
  return branches.find(b => b.is_current);
});

const isLoading = computed(() => gitStore.operationLoading);

function showPersistentError(error: unknown) {
  ElMessage({
    type: 'error',
    message: t('git.operationFailed', { error: String(error) }),
    duration: 0,
    showClose: true,
  });
}

async function handleFetch() {
  try {
    await gitStore.fetch(props.project.id, props.project.path);
    ElMessage.success(t('git.fetchSuccess'));
  } catch (e: any) {
    showPersistentError(e);
  }
}

async function handlePull() {
  try {
    await gitStore.pull(props.project.id, props.project.path);
    ElMessage.success(t('git.pullSuccess'));
  } catch (e: any) {
    showPersistentError(e);
  }
}

async function handlePush() {
  try {
    await gitStore.push(props.project.id, props.project.path);
    ElMessage.success(t('git.pushSuccess'));
  } catch (e: any) {
    showPersistentError(e);
  }
}
</script>

<template>
  <div class="git-toolbar">
    <!-- Current branch display -->
    <div class="branch-chip">
      <div class="i-mdi-source-branch text-xs text-blue-500" />
      <span class="text-[11px] font-medium text-slate-700 dark:text-slate-300 max-w-[120px] truncate">{{ currentBranch || 'HEAD' }}</span>
      <template v-if="currentBranchInfo">
        <span v-if="currentBranchInfo.ahead > 0" class="text-[9px] px-1 py-0.5 rounded-sm bg-green-500/10 text-green-600 dark:text-green-400 font-medium">
          +{{ currentBranchInfo.ahead }}
        </span>
        <span v-if="currentBranchInfo.behind > 0" class="text-[9px] px-1 py-0.5 rounded-sm bg-orange-500/10 text-orange-600 dark:text-orange-400 font-medium">
          -{{ currentBranchInfo.behind }}
        </span>
      </template>
    </div>

    <!-- Action buttons -->
    <button @click="handleFetch" :disabled="isLoading"
      class="toolbar-action" :title="t('git.fetch')">
      <div class="i-mdi-cloud-download-outline action-icon" />
      <span class="action-label">{{ t('git.fetch') }}</span>
    </button>

    <button @click="handlePull" :disabled="isLoading"
      class="toolbar-action" :title="t('git.pull')">
      <div class="i-mdi-arrow-down-bold action-icon" />
      <span class="action-label">{{ t('git.pull') }}</span>
      <span v-if="currentBranchInfo && currentBranchInfo.behind > 0"
        class="action-badge">
        {{ currentBranchInfo.behind }}
      </span>
    </button>

    <button @click="handlePush" :disabled="isLoading"
      class="toolbar-action" :title="t('git.push')">
      <div class="i-mdi-arrow-up-bold action-icon" />
      <span class="action-label">{{ t('git.push') }}</span>
      <span v-if="currentBranchInfo && currentBranchInfo.ahead > 0"
        class="action-badge">
        {{ currentBranchInfo.ahead }}
      </span>
    </button>

    <div class="toolbar-sep" />

    <button @click="emit('open-branch-dialog', 'create')" :disabled="isLoading"
      class="toolbar-action" :title="t('git.newBranch')">
      <div class="i-mdi-source-branch-plus action-icon" />
      <span class="action-label">{{ t('git.branch') }}</span>
    </button>

    <button @click="emit('open-stash')" :disabled="isLoading"
      class="toolbar-action" :title="t('git.stash')">
      <div class="i-mdi-package-down action-icon" />
      <span class="action-label">{{ t('git.stash') }}</span>
    </button>

    <div class="flex-1" />

    <!-- Refresh -->
    <button @click="emit('refresh')" :disabled="isLoading"
      class="toolbar-action" :title="t('git.refresh')">
      <div class="i-mdi-refresh action-icon" :class="{ 'animate-spin': isLoading }" />
      <span class="action-label">{{ t('git.refresh') }}</span>
    </button>

    <button @click="emit('open-remote-settings')" :disabled="isLoading"
      class="toolbar-action" :title="t('git.repoSettings')">
      <div class="i-mdi-cog-outline action-icon" />
      <span class="action-label">{{ t('git.settings') }}</span>
    </button>

    <!-- Loading indicator -->
    <div v-if="isLoading" class="flex items-center gap-1 text-blue-500 text-[11px]">
      <div class="i-mdi-loading animate-spin text-xs" />
    </div>
  </div>
</template>

<style scoped>
.git-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px 5px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.8);
  background: rgba(255, 255, 255, 0.78);
  backdrop-filter: blur(4px);
}

.dark .git-toolbar {
  border-bottom-color: rgba(71, 85, 105, 0.45);
  background: rgba(30, 41, 59, 0.72);
}

.branch-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-right: 10px;
  padding: 3px 10px;
  border-radius: 6px;
  border: 1px solid rgba(203, 213, 225, 0.85);
  background: rgba(248, 250, 252, 0.95);
}

.dark .branch-chip {
  border-color: rgba(71, 85, 105, 0.55);
  background: rgba(30, 41, 59, 0.6);
}

.toolbar-action {
  position: relative;
  width: 42px;
  height: 38px;
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  color: #2563eb;
  cursor: pointer;
  transition: background-color 120ms ease, color 120ms ease;
}

.toolbar-action:hover {
  background: rgba(37, 99, 235, 0.08);
}

.dark .toolbar-action:hover {
  background: rgba(59, 130, 246, 0.14);
}

.toolbar-action:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.action-icon {
  font-size: 16px;
  line-height: 1;
}

.action-label {
  font-size: 11px;
  line-height: 1;
  color: #111827;
}

.dark .action-label {
  color: #d1d5db;
}

.action-badge {
  position: absolute;
  top: 1px;
  right: 2px;
  min-width: 15px;
  height: 15px;
  padding: 0 4px;
  border-radius: 999px;
  background: #2563eb;
  color: #ffffff;
  font-size: 9px;
  font-weight: 700;
  line-height: 15px;
  text-align: center;
}

.toolbar-sep {
  width: 1px;
  height: 22px;
  margin: 0 4px;
  background: rgba(148, 163, 184, 0.5);
}

.dark .toolbar-sep {
  background: rgba(148, 163, 184, 0.45);
}
</style>
