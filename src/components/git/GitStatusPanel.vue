<script setup lang="ts">
import { computed, ref, onBeforeUnmount } from 'vue';
import { useGitStore } from '../../stores/git';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import { api } from '../../api';
import type { Project, GitFileStatus } from '../../types';

const props = defineProps<{
  project: Project;
}>();

const { t } = useI18n();
const gitStore = useGitStore();

const statusResult = computed(() => gitStore.getStatus(props.project.id));
const stagedFiles = computed(() => statusResult.value?.staged || []);
const unstagedFiles = computed(() => [
  ...(statusResult.value?.unstaged || []),
  ...(statusResult.value?.untracked || []),
]);
const conflictedFiles = computed(() => statusResult.value?.conflicted || []);

type ListType = 'staged' | 'unstaged';

const selectedFilePath = ref<string>('');
const selectedKeys = ref<Set<string>>(new Set());
const anchorKey = ref<string>('');
const activeList = ref<ListType>('unstaged');

const selectedStagedFile = computed(() => {
  const selected = selectedFilePath.value;
  if (!selected.endsWith(':s')) return null;
  const path = selected.slice(0, -2);
  return stagedFiles.value.find(file => file.path === path) || null;
});
const selectedUnstagedFile = computed(() => {
  const selected = selectedFilePath.value;
  if (!selected.endsWith(':u')) return null;
  const path = selected.slice(0, -2);
  return unstagedFiles.value.find(file => file.path === path) || null;
});

const selectedStagedFiles = computed(() =>
  stagedFiles.value.filter(file => selectedKeys.value.has(getKey(file.path, 'staged')))
);

const selectedUnstagedFiles = computed(() =>
  unstagedFiles.value.filter(file => selectedKeys.value.has(getKey(file.path, 'unstaged')))
);

// Context menu state
const ctxMenu = ref<{
  show: boolean;
  x: number;
  y: number;
  files: GitFileStatus[];
  isStaged: boolean;
}>({ show: false, x: 0, y: 0, files: [], isStaged: false });

function getKey(path: string, listType: ListType): string {
  return `${listType}:${path}`;
}

function getSuffix(listType: ListType): ':s' | ':u' {
  return listType === 'staged' ? ':s' : ':u';
}

function isSelected(path: string, listType: ListType): boolean {
  return selectedKeys.value.has(getKey(path, listType));
}

function selectSingle(file: GitFileStatus, listType: ListType) {
  const key = getKey(file.path, listType);
  selectedKeys.value = new Set([key]);
  anchorKey.value = key;
  activeList.value = listType;
  selectedFilePath.value = file.path + getSuffix(listType);
}

function selectRange(file: GitFileStatus, listType: ListType) {
  const list = listType === 'staged' ? stagedFiles.value : unstagedFiles.value;
  const targetKey = getKey(file.path, listType);
  const anchor = anchorKey.value && anchorKey.value.startsWith(listType + ':') ? anchorKey.value : targetKey;
  const anchorPath = anchor.slice(listType.length + 1);
  const start = list.findIndex(item => item.path === anchorPath);
  const end = list.findIndex(item => item.path === file.path);
  if (start === -1 || end === -1) {
    selectSingle(file, listType);
    return;
  }

  const [from, to] = start <= end ? [start, end] : [end, start];
  const next = new Set(selectedKeys.value);
  for (let i = from; i <= to; i += 1) {
    next.add(getKey(list[i].path, listType));
  }
  selectedKeys.value = next;
  anchorKey.value = targetKey;
  activeList.value = listType;
  selectedFilePath.value = file.path + getSuffix(listType);
}

function toggleSelect(file: GitFileStatus, listType: ListType) {
  const key = getKey(file.path, listType);
  const next = new Set(selectedKeys.value);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  selectedKeys.value = next;
  anchorKey.value = key;
  activeList.value = listType;
  selectedFilePath.value = file.path + getSuffix(listType);
}

function handleRowClick(e: MouseEvent, file: GitFileStatus, listType: ListType) {
  activeList.value = listType;
  if (e.shiftKey) {
    selectRange(file, listType);
  } else if (e.ctrlKey || e.metaKey) {
    toggleSelect(file, listType);
  } else {
    selectSingle(file, listType);
  }
  handleViewDiff(file, listType);
}

function handlePanelKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') {
    e.preventDefault();
    const next = new Set(selectedKeys.value);
    const list = activeList.value === 'staged' ? stagedFiles.value : unstagedFiles.value;
    for (const file of list) {
      next.add(getKey(file.path, activeList.value));
    }
    selectedKeys.value = next;
    if (list.length > 0) {
      const last = list[list.length - 1];
      anchorKey.value = getKey(last.path, activeList.value);
      selectedFilePath.value = last.path + getSuffix(activeList.value);
    }
  }
}

function setActiveList(listType: ListType) {
  activeList.value = listType;
}

// ─── Actions ─────────────────────────────────────────────────────────────────

async function handleStage(file: GitFileStatus) {
  try {
    await gitStore.stageFiles(props.project.id, props.project.path, [file.path]);
    selectedKeys.value.delete(getKey(file.path, 'unstaged'));
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function handleUnstage(file: GitFileStatus) {
  try {
    await gitStore.unstageFiles(props.project.id, props.project.path, [file.path]);
    selectedKeys.value.delete(getKey(file.path, 'staged'));
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function handleStageAll() {
  try {
    if (selectedUnstagedFiles.value.length > 0) {
      await gitStore.stageFiles(props.project.id, props.project.path, selectedUnstagedFiles.value.map(file => file.path));
    } else {
      await gitStore.stageAll(props.project.id, props.project.path);
    }
    selectedKeys.value = new Set([...selectedKeys.value].filter(key => !key.startsWith('unstaged:')));
  }
  catch (e: any) { ElMessage.error(t('git.operationFailed', { error: String(e) })); }
}

async function handleUnstageAll() {
  try {
    if (selectedStagedFiles.value.length > 0) {
      await gitStore.unstageFiles(props.project.id, props.project.path, selectedStagedFiles.value.map(file => file.path));
    } else {
      await gitStore.unstageAll(props.project.id, props.project.path);
    }
    selectedKeys.value = new Set([...selectedKeys.value].filter(key => !key.startsWith('staged:')));
  }
  catch (e: any) { ElMessage.error(t('git.operationFailed', { error: String(e) })); }
}

async function handleStageSelected() {
  if (!selectedUnstagedFile.value) return;
  await handleStage(selectedUnstagedFile.value);
}

async function handleUnstageSelected() {
  if (!selectedStagedFile.value) return;
  await handleUnstage(selectedStagedFile.value);
}

async function handleViewDiff(file: GitFileStatus, listType?: ListType) {
  const resolved = listType || (file.staged ? 'staged' : 'unstaged');
  selectedFilePath.value = file.path + getSuffix(resolved);
  try {
    await gitStore.getDiff(props.project.path, file.path, file.staged);
  } catch (e: any) {
    console.error('Failed to get diff:', e);
  }
}

// ─── Context Menu ────────────────────────────────────────────────────────────

function handleContextMenu(e: MouseEvent, file: GitFileStatus, isStaged: boolean) {
  e.preventDefault();
  e.stopPropagation();
  // Close any existing menu first
  ctxMenu.value.show = false;
  const listType: ListType = isStaged ? 'staged' : 'unstaged';
  activeList.value = listType;
  if (!isSelected(file.path, listType)) {
    selectSingle(file, listType);
  }
  const selectedList = listType === 'staged' ? selectedStagedFiles.value : selectedUnstagedFiles.value;
  const menuX = Math.min(e.clientX, window.innerWidth - 200);
  const menuY = Math.min(e.clientY, window.innerHeight - 300);
  ctxMenu.value = {
    show: true,
    x: menuX,
    y: menuY,
    files: selectedList.length > 0 ? selectedList : [file],
    isStaged,
  };
  requestAnimationFrame(() => {
    window.addEventListener('mousedown', handleCtxGlobalClick);
  });
}

function handleCtxGlobalClick(e: MouseEvent) {
  const menu = document.getElementById('git-status-ctx-menu');
  if (menu && !menu.contains(e.target as Node)) {
    closeCtxMenu();
  }
}

function closeCtxMenu() {
  ctxMenu.value.show = false;
  window.removeEventListener('mousedown', handleCtxGlobalClick);
}

onBeforeUnmount(() => {
  window.removeEventListener('mousedown', handleCtxGlobalClick);
});

async function ctxStage() {
  if (ctxMenu.value.files.length === 0) return;
  closeCtxMenu();
  try {
    await gitStore.stageFiles(props.project.id, props.project.path, ctxMenu.value.files.map(file => file.path));
    selectedKeys.value = new Set([...selectedKeys.value].filter(key => !key.startsWith('unstaged:')));
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function ctxUnstage() {
  if (ctxMenu.value.files.length === 0) return;
  closeCtxMenu();
  try {
    await gitStore.unstageFiles(props.project.id, props.project.path, ctxMenu.value.files.map(file => file.path));
    selectedKeys.value = new Set([...selectedKeys.value].filter(key => !key.startsWith('staged:')));
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function ctxDiscard() {
  if (ctxMenu.value.files.length === 0) return;
  closeCtxMenu();
  const files = ctxMenu.value.files;
  try {
    await ElMessageBox.confirm(t('git.discardConfirm'), t('git.discard'), {
      confirmButtonText: t('common.confirm'),
      cancelButtonText: t('common.cancel'),
      type: 'warning',
    });
    const untracked = files.filter(file => file.status === 'untracked').map(file => file.path);
    const tracked = files.filter(file => file.status !== 'untracked').map(file => file.path);
    if (tracked.length > 0) await gitStore.discardFiles(props.project.id, props.project.path, tracked);
    if (untracked.length > 0) await gitStore.discardUntracked(props.project.id, props.project.path, untracked);
    selectedKeys.value = new Set();
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function ctxStopTracking() {
  if (ctxMenu.value.files.length === 0) return;
  closeCtxMenu();
  try {
    await gitStore.rmCached(props.project.id, props.project.path, ctxMenu.value.files.map(file => file.path));
    selectedKeys.value = new Set();
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function ctxIgnore() {
  if (ctxMenu.value.files.length === 0) return;
  closeCtxMenu();
  const filePath = ctxMenu.value.files[0].path;
  try {
    ElMessage.info('Please manually add "' + filePath + '" to .gitignore');
  } catch (e: any) {
    ElMessage.error(String(e));
  }
}

async function ctxOpenInEditor() {
  if (ctxMenu.value.files.length === 0) return;
  closeCtxMenu();
  try {
    await api.openInEditor(props.project.path + '/' + ctxMenu.value.files[0].path);
  } catch (e: any) {
    ElMessage.error(String(e));
  }
}
</script>

<template>
  <div class="h-full flex flex-col overflow-hidden" tabindex="0" @keydown="handlePanelKeydown">

    <!-- Conflicted files -->
    <div v-if="conflictedFiles.length > 0" class="shrink-0 border-b border-red-500/15">
      <div class="section-header bg-red-500/4">
        <span class="section-title text-red-500 dark:text-red-400">{{ t('git.conflictedFiles') }}</span>
        <span class="section-badge bg-red-500/10 text-red-500">{{ conflictedFiles.length }}</span>
      </div>
      <div class="max-h-36 overflow-y-auto custom-scrollbar">
        <div v-for="file in conflictedFiles" :key="file.path"
          @click="handleRowClick($event, file, 'unstaged')"
          @contextmenu="handleContextMenu($event, file, false)"
          class="file-row"
          :class="{ 'file-row-active': isSelected(file.path, 'unstaged') }"
          @mouseenter="setActiveList('unstaged')">
          <span class="file-path">{{ file.path }}</span>
        </div>
      </div>
    </div>

    <!-- ===== Staged changes ===== -->
    <div class="flex-1 min-h-0 flex flex-col section-block">
      <div class="section-header section-header-light border-b border-slate-300/60">
        <span class="section-title text-slate-600 dark:text-slate-400">{{ t('git.stagedChanges') }}</span>
        <span class="flex-1" />
        <button v-if="selectedStagedFile" @click="handleUnstageSelected"
          class="section-action-chip" :title="t('git.unstage')">
          {{ t('git.unstage') }}
        </button>
        <button v-if="stagedFiles.length > 0" @click="handleUnstageAll"
          class="section-action-chip" :title="t('git.unstageAll')">
          {{ t('git.unstageAll') }}
        </button>
      </div>
      <div class="flex-1 overflow-y-auto custom-scrollbar">
        <div v-for="file in stagedFiles" :key="file.path + ':s'"
          @click="handleRowClick($event, file, 'staged')"
          @contextmenu="handleContextMenu($event, file, true)"
          class="file-row"
          :class="{ 'file-row-active': isSelected(file.path, 'staged') }"
          @mouseenter="setActiveList('staged')">
          <span class="file-path">{{ file.path }}</span>
          <button @click.stop="handleUnstage(file)"
            class="row-action-btn" :title="t('git.unstage')">
            −
          </button>
        </div>
        <div v-if="stagedFiles.length === 0" class="empty-hint">
          {{ t('git.noChanges') }}
        </div>
      </div>
    </div>

    <!-- ===== Unstaged changes ===== -->
    <div class="flex-1 min-h-0 flex flex-col section-block mt-2">
      <div class="section-header section-header-light border-b border-slate-300/60">
        <span class="section-title text-slate-600 dark:text-slate-400">{{ t('git.unstagedChanges') }}</span>
        <span class="flex-1" />
        <button v-if="selectedUnstagedFile" @click="handleStageSelected"
          class="section-action-chip" :title="t('git.stage')">
          {{ t('git.stage') }}
        </button>
        <button v-if="unstagedFiles.length > 0" @click="handleStageAll"
          class="section-action-chip" :title="t('git.stageAll')">
          {{ t('git.stageAll') }}
        </button>
      </div>
      <div class="flex-1 overflow-y-auto custom-scrollbar">
        <div v-for="file in unstagedFiles" :key="file.path + ':u'"
          @click="handleRowClick($event, file, 'unstaged')"
          @contextmenu="handleContextMenu($event, file, false)"
          class="file-row"
          :class="{ 'file-row-active': isSelected(file.path, 'unstaged') }"
          @mouseenter="setActiveList('unstaged')">
          <span class="file-path">{{ file.path }}</span>
          <button @click.stop="handleStage(file)"
            class="row-action-btn" :title="t('git.stage')">
            +
          </button>
        </div>
        <div v-if="unstagedFiles.length === 0" class="empty-hint">
          {{ t('git.noChanges') }}
        </div>
      </div>
    </div>

    <!-- File Context Menu -->
    <Teleport to="body">
      <Transition name="ctx-menu">
        <div v-if="ctxMenu.show && ctxMenu.files.length > 0"
          id="git-status-ctx-menu"
          class="fixed flex flex-col min-w-[220px] py-[3px] rounded-[2px] bg-[#f3f4f6] dark:bg-[#2f343c] shadow-[0_4px_10px_rgba(15,23,42,0.12)] dark:shadow-[0_8px_18px_rgba(2,6,23,0.52)] text-[12px]"
          :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px', zIndex: 99999, isolation: 'isolate' }">
          <div v-if="!ctxMenu.isStaged" @click="ctxStage" class="ctx-action">{{ t('git.stage') }}</div>
          <div v-if="ctxMenu.isStaged" @click="ctxUnstage" class="ctx-action">{{ t('git.unstage') }}</div>
          <div @click="ctxDiscard" class="ctx-action">{{ t('git.discard') }}</div>
          <div class="ctx-sep" />
          <div @click="ctxStopTracking" class="ctx-action">{{ t('git.stopTracking') }}</div>
          <div @click="ctxIgnore" class="ctx-action">{{ t('git.ignoreFile') }}</div>
          <div class="ctx-sep" />
          <div @click="ctxOpenInEditor" class="ctx-action">{{ t('git.openInEditor') }}</div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
/* ── Section header ────────────────────────────────────── */
.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  height: 28px;
  flex-shrink: 0;
}
.section-header-light {
  background: rgba(241, 245, 249, 0.8);
}
.dark .section-header-light {
  background: rgba(30, 41, 59, 0.58);
}
.section-title {
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.1px;
}
.dark .section-title {
  color: #cbd5e1;
}
.section-badge {
  font-size: 10px;
  min-width: 18px;
  text-align: center;
  padding: 0 6px;
  border-radius: 999px;
  line-height: 16px;
  font-weight: 500;
}
.section-action-chip {
  font-size: 11px;
  line-height: 1;
  padding: 0 6px;
  height: 18px;
  border: 1px solid rgba(148, 163, 184, 0.7);
  border-radius: 3px;
  background: #ffffff;
  color: #475569;
  cursor: pointer;
  transition: background-color 120ms ease, border-color 120ms ease;
}
.section-action-chip:hover {
  background: #f8fafc;
}
.dark .section-action-chip {
  border-color: rgba(100, 116, 139, 0.75);
  background: rgba(15, 23, 42, 0.6);
  color: #cbd5e1;
}
.dark .section-action-chip:hover {
  background: rgba(30, 41, 59, 0.8);
}

.section-block {
  border: 1px solid rgba(203, 213, 225, 0.8);
  background: #ffffff;
}
.dark .section-block {
  border-color: rgba(71, 85, 105, 0.6);
  background: rgba(15, 23, 42, 0.88);
}

/* ── File rows ─────────────────────────────────────────── */
.file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 26px;
  padding: 0 12px;
  cursor: pointer;
  transition: background-color 100ms ease;
}
.file-row:hover {
  background: rgba(241, 245, 249, 0.85);
}
.file-row-active {
  background: #f1f5f9;
}
.file-row-active:hover {
  background: #f1f5f9;
}
.dark .file-row:hover {
  background: rgba(51, 65, 85, 0.45);
}
.dark .file-row-active,
.dark .file-row-active:hover {
  background: rgba(51, 65, 85, 0.6);
}

.file-path {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  color: #0f172a;
}
.dark .file-path {
  color: #e2e8f0;
}
.row-action-btn {
  margin-left: auto;
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
  font-size: 12px;
  line-height: 1;
  font-weight: 500;
  flex-shrink: 0;
  cursor: pointer;
  transition: background-color 100ms ease, border-color 100ms ease;
  color: #475569;
  border: 1px solid rgba(148, 163, 184, 0.7);
  background: #ffffff;
}
.row-action-btn:hover {
  background: #f8fafc;
}
.dark .row-action-btn {
  color: #cbd5e1;
  border-color: rgba(100, 116, 139, 0.75);
  background: rgba(15, 23, 42, 0.6);
}
.dark .row-action-btn:hover {
  background: rgba(30, 41, 59, 0.8);
}
.empty-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px 0;
  color: #94a3b8;
  font-size: 11px;
  user-select: none;
}
.dark .empty-hint {
  color: #64748b;
}

/* ── Context menu ──────────────────────────────────────── */
:global(#git-status-ctx-menu .ctx-action) {
  display: block;
  width: 100%;
  min-height: 24px;
  padding: 0 14px;
  text-align: left;
  font-size: 12px;
  font-weight: 400;
  line-height: 24px;
  color: #1f2937;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: background-color 80ms ease;
}
:global(#git-status-ctx-menu .ctx-action:hover) {
  background: #e5e7eb;
}
:global(.dark #git-status-ctx-menu .ctx-action) {
  color: #e5e7eb;
}
:global(.dark #git-status-ctx-menu .ctx-action:hover) {
  background: #3d4450;
}
.ctx-sep {
  border-top: 1px solid #d1d5db;
  margin: 3px 0;
}
.dark .ctx-sep {
  border-top-color: #4b5563;
}

/* ── Transitions ───────────────────────────────────────── */
.ctx-menu-enter-active {
  transition: opacity 120ms ease-out, transform 120ms ease-out;
}
.ctx-menu-leave-active {
  transition: opacity 80ms ease-in, transform 80ms ease-in;
}
.ctx-menu-enter-from,
.ctx-menu-leave-to {
  opacity: 0;
  transform: scale(0.96);
}

/* ── Scrollbar ─────────────────────────────────────────── */
.custom-scrollbar::-webkit-scrollbar {
  width: 3px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 2px;
}
.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: #334155;
}
</style>
