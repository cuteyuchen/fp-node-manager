<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { useProjectStore } from '../../stores/project';
import { useGitStore } from '../../stores/git';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { useSplitPane } from '../../composables/useSplitPane';
import GitToolbar from './GitToolbar.vue';
import GitStatusPanel from './GitStatusPanel.vue';
import GitCommitArea from './GitCommitArea.vue';
import GitDiffView from './GitDiffView.vue';
import GitHistory from './GitHistory.vue';
import GitBranchDialog from './GitBranchDialog.vue';
import GitCommitFileList from './GitCommitFileList.vue';

const { t } = useI18n();
const projectStore = useProjectStore();
const gitStore = useGitStore();

const activeTab = ref<'changes' | 'history'>('changes');
const showBranchDialog = ref(false);

const activeProject = computed(() =>
  projectStore.projects.find(p => p.id === projectStore.activeProjectId)
);

const isGitRepo = computed(() => {
  if (!activeProject.value) return false;
  return gitStore.isGitRepo[activeProject.value.id] || false;
});

// Draggable split panes for changes tab
const leftPane = useSplitPane({ initial: 280, min: 180, max: 500, direction: 'horizontal' });
const commitPane = useSplitPane({ initial: 180, min: 120, max: 400, direction: 'vertical', reverse: true });
// For the staged/unstaged vertical split inside status panel, we use a percentage-based approach
const stagedRatio = ref(50); // percentage of staged area height
let stagedDragStart = 0;
let stagedRatioStart = 0;
const isDraggingStagedSplit = ref(false);
const statusPanelRef = ref<HTMLElement | null>(null);

function onStagedSplitMouseDown(e: MouseEvent) {
  e.preventDefault();
  isDraggingStagedSplit.value = true;
  stagedDragStart = e.clientY;
  stagedRatioStart = stagedRatio.value;
  document.addEventListener('mousemove', onStagedSplitMouseMove);
  document.addEventListener('mouseup', onStagedSplitMouseUp);
  document.body.style.cursor = 'row-resize';
  document.body.style.userSelect = 'none';
}

function onStagedSplitMouseMove(e: MouseEvent) {
  if (!statusPanelRef.value) return;
  const panelHeight = statusPanelRef.value.clientHeight;
  if (panelHeight <= 0) return;
  const delta = e.clientY - stagedDragStart;
  const deltaPercent = (delta / panelHeight) * 100;
  const newRatio = Math.min(85, Math.max(15, stagedRatioStart + deltaPercent));
  stagedRatio.value = newRatio;
}

function onStagedSplitMouseUp() {
  isDraggingStagedSplit.value = false;
  document.removeEventListener('mousemove', onStagedSplitMouseMove);
  document.removeEventListener('mouseup', onStagedSplitMouseUp);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
}

// Watch project changes — refresh and clear stale diff
watch(activeProject, async (newProject, oldProject) => {
  if (oldProject?.id !== newProject?.id) {
    gitStore.clearDiff();
  }
  if (newProject) {
    const isRepo = await gitStore.checkGitRepo(newProject.id, newProject.path);
    if (isRepo) {
      await gitStore.refreshSummaryAndStatus(newProject.id, newProject.path);
    }
  }
}, { immediate: true });

// Clear diff when switching tabs; lazy-load history
watch(activeTab, (tab) => {
  gitStore.clearDiff();
  if (tab === 'history' && activeProject.value) {
    gitStore.refreshHistory(activeProject.value.id, activeProject.value.path);
  }
});

async function handleInitRepo() {
  if (!activeProject.value) return;
  try {
    await gitStore.initRepo(activeProject.value.id, activeProject.value.path);
    ElMessage.success(t('git.initSuccess'));
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function handleRefresh() {
  if (!activeProject.value) return;
  await gitStore.refreshSummaryAndStatus(activeProject.value.id, activeProject.value.path);
  if (activeTab.value === 'history') {
    await gitStore.refreshHistory(activeProject.value.id, activeProject.value.path);
  }
}

const tabs = computed(() => [
  { value: 'changes' as const, label: t('git.fileStatus') },
  { value: 'history' as const, label: t('git.commitHistory') },
]);
</script>

<template>
  <div class="h-full flex flex-col overflow-hidden">
    <!-- Not a git repo -->
    <div v-if="!isGitRepo" class="flex-1 flex flex-col items-center justify-center gap-3">
      <div class="i-mdi-source-branch text-4xl text-slate-300 dark:text-slate-600" />
      <p class="text-sm text-slate-500 dark:text-slate-400">{{ t('git.notGitRepo') }}</p>
      <button @click="handleInitRepo"
        class="px-4 py-1.5 rounded-md text-xs font-medium bg-blue-500 hover:bg-blue-600 text-white transition-colors cursor-pointer">
        {{ t('git.initRepo') }}
      </button>
    </div>

    <!-- Git view -->
    <template v-else-if="activeProject">
      <!-- Toolbar -->
      <GitToolbar
        :project="activeProject"
        @open-branch-dialog="showBranchDialog = true"
        @refresh="handleRefresh"
      />

      <!-- Tab bar -->
      <div class="flex border-b border-slate-200/40 dark:border-slate-700/30 shrink-0 px-2">
        <button
          v-for="tab in tabs"
          :key="tab.value"
          @click="activeTab = tab.value"
          class="px-3 py-1.5 text-[11px] font-medium transition-colors border-b-2 -mb-px cursor-pointer"
          :class="activeTab === tab.value
            ? 'border-blue-500 text-blue-600 dark:text-blue-400'
            : 'border-transparent text-slate-500 dark:text-slate-300 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-100/60 dark:hover:bg-slate-700/40'"
        >
          {{ tab.label }}
        </button>
      </div>

      <!-- ===== CHANGES TAB: SourceTree-style layout ===== -->
      <div v-if="activeTab === 'changes'" class="flex-1 flex flex-col min-h-0">
        <!-- Top area: status panel (left) + diff view (right) -->
        <div class="flex-1 flex min-h-0">
          <!-- Left: file status panel (staged top / unstaged bottom) -->
          <div class="flex flex-col shrink-0 border-r border-slate-200/40 dark:border-slate-700/30" :style="{ width: leftPane.size.value + 'px' }">
            <div ref="statusPanelRef" class="flex-1 flex flex-col min-h-0 overflow-hidden">
              <GitStatusPanel
                :project="activeProject"
                :staged-ratio="stagedRatio"
                @staged-split-mousedown="onStagedSplitMouseDown"
              />
            </div>
          </div>

          <!-- Vertical drag handle: left ↔ right -->
          <div
            class="w-1 hover:w-1.5 shrink-0 cursor-col-resize transition-all group relative"
            :class="leftPane.isDragging.value ? 'bg-blue-500/40 w-1.5' : 'hover:bg-blue-500/20'"
            @mousedown="leftPane.onMouseDown"
          >
            <div class="absolute inset-y-0 -left-1 -right-1" />
          </div>

          <!-- Right: diff view -->
          <div class="flex-1 min-w-0">
            <GitDiffView :project="activeProject" />
          </div>
        </div>

        <!-- Horizontal drag handle: workspace ↔ commit -->
        <div
          class="h-1 hover:h-1.5 shrink-0 cursor-row-resize transition-all"
          :class="commitPane.isDragging.value ? 'bg-blue-500/40 h-1.5' : 'hover:bg-blue-500/20'"
          @mousedown="commitPane.onMouseDown"
        >
          <div class="absolute inset-x-0 -top-1 -bottom-1 relative" />
        </div>

        <!-- Bottom: commit area spanning full width -->
        <div class="shrink-0 border-t border-slate-200/40 dark:border-slate-700/30" :style="{ height: commitPane.size.value + 'px' }">
          <GitCommitArea :project="activeProject" class="h-full" />
        </div>
      </div>

      <!-- ===== HISTORY TAB: three-column layout ===== -->
      <div v-else class="flex-1 flex min-h-0">
        <!-- Left panel: commit list -->
        <div class="w-[260px] flex flex-col border-r border-slate-200/40 dark:border-slate-700/30 shrink-0">
          <GitHistory :project="activeProject" />
        </div>

        <!-- Middle panel: commit file list -->
        <div class="w-[220px] flex flex-col border-r border-slate-200/40 dark:border-slate-700/30 shrink-0">
          <GitCommitFileList :project="activeProject" />
        </div>

        <!-- Right panel: diff view -->
        <div class="flex-1 min-w-0">
          <GitDiffView :project="activeProject" />
        </div>
      </div>

      <!-- Branch dialog -->
      <GitBranchDialog
        v-model="showBranchDialog"
        :project="activeProject"
      />
    </template>
  </div>
</template>
