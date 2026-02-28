<script setup lang="ts">
import { computed, ref, inject, nextTick, onBeforeUnmount } from 'vue';
import { useGitStore } from '../../stores/git';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox, ElLoading } from 'element-plus';
import type { Project } from '../../types';

const props = defineProps<{
  project: Project;
}>();

const emit = defineEmits<{
  (e: 'open-branch-dialog', mode: 'create' | 'delete', branch?: string): void;
  (e: 'open-stash'): void;
}>();

const { t } = useI18n();
const gitStore = useGitStore();
const scrollToBranch = inject<(name: string) => void>('scrollToBranch', () => {});

const expandedSections = ref<Record<string, boolean>>({
  branches: true,
  remoteBranches: false,
  tags: false,
  remotes: false,
  stashes: false,
});

// Context menu state
const contextMenu = ref<{
  show: boolean;
  x: number;
  y: number;
  name: string;
  type: 'local' | 'remote' | 'tag' | 'remoteRepo';
  isCurrent: boolean;
}>({ show: false, x: 0, y: 0, name: '', type: 'local', isCurrent: false });

const localBranches = computed(() => gitStore.getLocalBranches(props.project.id));
const remoteBranches = computed(() => gitStore.getRemoteBranches(props.project.id));
const tagList = computed(() => gitStore.tags[props.project.id] || []);
const remoteList = computed(() => gitStore.remotes[props.project.id] || []);
const stashList = computed(() => gitStore.stashes[props.project.id] || []);

function showPersistentError(error: unknown) {
  ElMessage({
    type: 'error',
    message: t('git.operationFailed', { error: String(error) }),
    duration: 0,
    showClose: true,
  });
}

function toggleSection(section: string) {
  expandedSections.value[section] = !expandedSections.value[section];
}

// Left click: navigate to branch/tag position in log graph
function handleItemClick(name: string) {
  scrollToBranch(name);
}

// Right click: show context menu
function handleContextMenu(e: MouseEvent, name: string, type: 'local' | 'remote' | 'tag' | 'remoteRepo', isCurrent: boolean = false) {
  e.preventDefault();
  e.stopPropagation();
  // Close any existing menu first
  contextMenu.value.show = false;
  const menuX = Math.min(e.clientX, window.innerWidth - 200);
  const menuY = Math.min(e.clientY, window.innerHeight - 260);
  nextTick(() => {
    contextMenu.value = { show: true, x: menuX, y: menuY, name, type, isCurrent };
    // Use a tiny delay to avoid the event that opens the menu from also closing it
    requestAnimationFrame(() => {
      window.addEventListener('mousedown', handleGlobalClick);
    });
  });
}

function handleGlobalClick(e: MouseEvent) {
  const menu = document.getElementById('git-sidebar-ctx-menu');
  if (menu && !menu.contains(e.target as Node)) {
    closeContextMenu();
  }
}

function closeContextMenu() {
  contextMenu.value.show = false;
  window.removeEventListener('mousedown', handleGlobalClick);
}

onBeforeUnmount(() => {
  window.removeEventListener('mousedown', handleGlobalClick);
});

// ── Branch operations ────────────────────────────────────────────────────

async function handleCheckout() {
  closeContextMenu();
  const name = contextMenu.value.name;
  const loading = ElLoading.service({
    target: document.querySelector('.git-sidebar') as HTMLElement || undefined,
    text: t('git.switchBranch') + '...',
    background: 'rgba(0,0,0,0.25)',
  });
  try {
    if (contextMenu.value.type === 'remote') {
      const parts = name.split('/');
      const localName = parts.slice(1).join('/');
      try {
        await gitStore.checkout(props.project.id, props.project.path, localName);
        ElMessage.success(t('git.switchSuccess', { name: localName }));
      } catch {
        await gitStore.createBranch(props.project.id, props.project.path, localName, name);
        ElMessage.success(t('git.switchSuccess', { name: localName }));
      }
    } else {
      await gitStore.checkout(props.project.id, props.project.path, name);
      ElMessage.success(t('git.switchSuccess', { name }));
    }
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  } finally {
    loading.close();
  }
}

async function handlePullBranch() {
  closeContextMenu();
  try {
    await gitStore.pull(props.project.id, props.project.path, undefined, contextMenu.value.name);
    ElMessage.success(t('git.pullSuccess'));
  } catch (e: any) {
    showPersistentError(e);
  }
}

async function handleMergeBranch() {
  closeContextMenu();
  try {
    await gitStore.merge(props.project.id, props.project.path, contextMenu.value.name);
    ElMessage.success(t('git.mergeSuccess'));
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function handleRebaseBranch() {
  closeContextMenu();
  try {
    await gitStore.rebase(props.project.id, props.project.path, contextMenu.value.name);
    ElMessage.success(t('git.rebaseSuccess'));
  } catch (e: any) {
    ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function handleDeleteBranch() {
  closeContextMenu();
  const name = contextMenu.value.name;
  try {
    await ElMessageBox.confirm(
      t('git.deleteBranchConfirm', { name }),
      t('git.deleteBranch'),
      { confirmButtonText: t('common.confirm'), cancelButtonText: t('common.cancel'), type: 'warning' }
    );
    await gitStore.deleteBranch(props.project.id, props.project.path, name);
    ElMessage.success(t('git.deleteBranchSuccess', { name }));
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function handleDeleteTag() {
  closeContextMenu();
  const name = contextMenu.value.name;
  try {
    await ElMessageBox.confirm(
      t('git.deleteTagConfirm', { name }),
      t('git.deleteTag'),
      { confirmButtonText: t('common.confirm'), cancelButtonText: t('common.cancel'), type: 'warning' }
    );
    await gitStore.deleteTag(props.project.id, props.project.path, name);
    ElMessage.success(t('git.deleteTagSuccess', { name }));
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function handleFetchRemote() {
  closeContextMenu();
  try {
    await gitStore.fetch(props.project.id, props.project.path, contextMenu.value.name);
    ElMessage.success(t('git.fetchSuccess'));
  } catch (e: any) {
    showPersistentError(e);
  }
}

// Current branch operations
async function handleFetchCurrent() {
  closeContextMenu();
  try {
    await gitStore.fetch(props.project.id, props.project.path);
    ElMessage.success(t('git.fetchSuccess'));
  } catch (e: any) {
    showPersistentError(e);
  }
}

async function handlePullCurrent() {
  closeContextMenu();
  try {
    await gitStore.pull(props.project.id, props.project.path);
    ElMessage.success(t('git.pullSuccess'));
  } catch (e: any) {
    showPersistentError(e);
  }
}

async function handlePushCurrent() {
  closeContextMenu();
  try {
    await gitStore.push(props.project.id, props.project.path);
    ElMessage.success(t('git.pushSuccess'));
  } catch (e: any) {
    showPersistentError(e);
  }
}

async function handleRenameBranch() {
  closeContextMenu();
  const name = contextMenu.value.name;
  try {
    const result = await ElMessageBox.prompt(
      t('git.renameBranchPrompt', { name }),
      t('git.renameBranch'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        inputValue: name,
        inputPattern: /^[^\s~^:?*\[\\]+$/,
        inputErrorMessage: t('git.branchNameInvalid'),
      }
    );
    const newName = (result as any).value ?? result;
    if (newName && newName !== name) {
      await gitStore.renameBranch(props.project.id, props.project.path, name, newName);
      ElMessage.success(t('git.renameBranchSuccess', { old: name, name: newName }));
    }
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}
</script>

<template>
  <div @contextmenu.prevent class="git-sidebar flex flex-col border-r border-slate-200/60 dark:border-slate-700/20 bg-gradient-to-b from-white/80 to-slate-50/60 dark:from-[#0f172a]/80 dark:to-[#0c1222]/60 overflow-y-auto custom-scrollbar text-xs select-none">

    <!-- Local Branches -->
    <div class="border-b border-slate-200/40 dark:border-slate-700/15">
      <div @click="toggleSection('branches')"
        class="flex items-center justify-between px-3 py-1.5 hover:bg-slate-100/70 dark:hover:bg-slate-800/25 cursor-pointer transition-colors duration-150">
        <div class="flex items-center gap-1.5 font-medium text-slate-600 dark:text-slate-400 text-[11px]">
          <div class="i-mdi-chevron-right text-xs transition-transform duration-200" :class="{ 'rotate-90': expandedSections.branches }" />
          <div class="i-mdi-source-branch text-xs text-blue-500" />
          {{ t('git.localBranches') }}
        </div>
        <span class="text-[9px] text-slate-400 bg-slate-100/80 dark:bg-slate-800/60 px-1.5 py-0.5 rounded-full leading-none font-medium">{{ localBranches.length }}</span>
      </div>
      <div v-if="expandedSections.branches" class="pb-1 pt-0.5">
        <div v-for="branch in localBranches" :key="branch.name"
          @click="handleItemClick(branch.name)"
          @contextmenu.prevent.stop="handleContextMenu($event, branch.name, 'local', branch.is_current)"
          class="flex items-center justify-between px-2 py-[3px] ml-4 mr-1.5 rounded hover:bg-blue-50/60 dark:hover:bg-blue-900/10 cursor-pointer transition-colors duration-150"
          :class="{ 'text-blue-600 dark:text-blue-400 font-medium bg-blue-50/40 dark:bg-blue-900/8': branch.is_current }">
          <div class="flex items-center gap-1.5 truncate flex-1 min-w-0">
            <div v-if="branch.is_current" class="w-1.5 h-1.5 rounded-full bg-blue-500 ring-2 ring-blue-500/20 flex-shrink-0" />
            <div v-else class="w-1.5 h-1.5 rounded-full bg-slate-300 dark:bg-slate-600 flex-shrink-0" />
            <span class="truncate text-[11px]">{{ branch.name }}</span>
            <span v-if="branch.ahead > 0" class="text-[8px] text-green-600 dark:text-green-400 font-mono flex-shrink-0">+{{ branch.ahead }}</span>
            <span v-if="branch.behind > 0" class="text-[8px] text-orange-500 font-mono flex-shrink-0">-{{ branch.behind }}</span>
          </div>
        </div>
        <div @click="emit('open-branch-dialog', 'create')"
          class="flex items-center gap-1.5 px-2 py-[3px] ml-4 mr-1.5 rounded hover:bg-slate-100/80 dark:hover:bg-slate-800/30 cursor-pointer text-slate-400 hover:text-blue-500 transition-colors duration-150">
          <div class="i-mdi-plus text-[10px]" />
          <span class="text-[10px]">{{ t('git.newBranch') }}</span>
        </div>
      </div>
    </div>

    <!-- Remote Branches -->
    <div class="border-b border-slate-200/40 dark:border-slate-700/15">
      <div @click="toggleSection('remoteBranches')"
        class="flex items-center justify-between px-3 py-1.5 hover:bg-slate-100/70 dark:hover:bg-slate-800/25 cursor-pointer transition-colors duration-150">
        <div class="flex items-center gap-1.5 font-medium text-slate-600 dark:text-slate-400 text-[11px]">
          <div class="i-mdi-chevron-right text-xs transition-transform duration-200" :class="{ 'rotate-90': expandedSections.remoteBranches }" />
          <div class="i-mdi-cloud-outline text-xs text-purple-500" />
          {{ t('git.remoteBranches') }}
        </div>
        <span class="text-[9px] text-slate-400 bg-slate-100/80 dark:bg-slate-800/60 px-1.5 py-0.5 rounded-full leading-none font-medium">{{ remoteBranches.length }}</span>
      </div>
      <div v-if="expandedSections.remoteBranches" class="pb-1 pt-0.5">
        <div v-for="branch in remoteBranches" :key="branch.name"
          @click="handleItemClick(branch.name)"
          @contextmenu.prevent.stop="handleContextMenu($event, branch.name, 'remote')"
          class="flex items-center gap-1.5 px-2 py-[3px] ml-4 mr-1.5 rounded hover:bg-purple-50/60 dark:hover:bg-purple-900/10 cursor-pointer transition-colors duration-150">
          <div class="w-1.5 h-1.5 rounded-full bg-purple-400/50 dark:bg-purple-500/40 flex-shrink-0" />
          <span class="truncate text-[11px] text-slate-600 dark:text-slate-400">{{ branch.name }}</span>
        </div>
        <div v-if="remoteBranches.length === 0" class="px-3 py-2 ml-4 text-slate-400/60 italic text-[10px]">—</div>
      </div>
    </div>

    <!-- Tags -->
    <div class="border-b border-slate-200/40 dark:border-slate-700/15">
      <div @click="toggleSection('tags')"
        class="flex items-center justify-between px-3 py-1.5 hover:bg-slate-100/70 dark:hover:bg-slate-800/25 cursor-pointer transition-colors duration-150">
        <div class="flex items-center gap-1.5 font-medium text-slate-600 dark:text-slate-400 text-[11px]">
          <div class="i-mdi-chevron-right text-xs transition-transform duration-200" :class="{ 'rotate-90': expandedSections.tags }" />
          <div class="i-mdi-tag-outline text-xs text-amber-500" />
          {{ t('git.tags') }}
        </div>
        <span class="text-[9px] text-slate-400 bg-slate-100/80 dark:bg-slate-800/60 px-1.5 py-0.5 rounded-full leading-none font-medium">{{ tagList.length }}</span>
      </div>
      <div v-if="expandedSections.tags" class="pb-1 pt-0.5">
        <div v-for="tag in tagList" :key="tag.name"
          class="flex items-center gap-1.5 px-2 py-[3px] ml-4 mr-1.5 rounded hover:bg-amber-50/60 dark:hover:bg-amber-900/10 cursor-pointer transition-colors duration-150"
          @click="handleItemClick('tag: ' + tag.name)"
          @contextmenu.prevent.stop="handleContextMenu($event, tag.name, 'tag')">
          <div class="w-1.5 h-1.5 rounded-sm bg-amber-400/60 dark:bg-amber-500/40 flex-shrink-0" />
          <span class="truncate text-[11px] text-slate-600 dark:text-slate-400">{{ tag.name }}</span>
          <span class="text-[8px] text-slate-400/60 font-mono ml-auto flex-shrink-0">{{ tag.hash }}</span>
        </div>
        <div v-if="tagList.length === 0" class="px-3 py-2 ml-4 text-slate-400/60 italic text-[10px]">—</div>
      </div>
    </div>

    <!-- Remotes -->
    <div class="border-b border-slate-200/40 dark:border-slate-700/15">
      <div @click="toggleSection('remotes')"
        class="flex items-center justify-between px-3 py-1.5 hover:bg-slate-100/70 dark:hover:bg-slate-800/25 cursor-pointer transition-colors duration-150">
        <div class="flex items-center gap-1.5 font-medium text-slate-600 dark:text-slate-400 text-[11px]">
          <div class="i-mdi-chevron-right text-xs transition-transform duration-200" :class="{ 'rotate-90': expandedSections.remotes }" />
          <div class="i-mdi-server-network text-xs text-teal-500" />
          {{ t('git.remotes') }}
        </div>
        <span class="text-[9px] text-slate-400 bg-slate-100/80 dark:bg-slate-800/60 px-1.5 py-0.5 rounded-full leading-none font-medium">{{ new Set(remoteList.map(r => r.name)).size }}</span>
      </div>
      <div v-if="expandedSections.remotes" class="pb-1 pt-0.5">
        <div v-for="remote in remoteList.filter(r => r.remote_type === 'fetch')" :key="remote.name"
          @contextmenu.prevent.stop="handleContextMenu($event, remote.name, 'remoteRepo')"
          class="flex items-center gap-1.5 px-2 py-[3px] ml-4 mr-1.5 rounded hover:bg-teal-50/60 dark:hover:bg-teal-900/10 cursor-pointer transition-colors duration-150">
          <div class="w-1.5 h-1.5 rounded-full bg-teal-400/50 dark:bg-teal-500/40 flex-shrink-0" />
          <span class="font-medium text-[11px] text-slate-600 dark:text-slate-400">{{ remote.name }}</span>
          <span class="text-slate-400/50 truncate text-[9px] ml-auto">{{ remote.url }}</span>
        </div>
        <div v-if="remoteList.length === 0" class="px-3 py-2 ml-4 text-slate-400/60 italic text-[10px]">—</div>
      </div>
    </div>

    <!-- Stashes -->
    <div>
      <div @click="toggleSection('stashes')"
        class="flex items-center justify-between px-3 py-1.5 hover:bg-slate-100/70 dark:hover:bg-slate-800/25 cursor-pointer transition-colors duration-150">
        <div class="flex items-center gap-1.5 font-medium text-slate-600 dark:text-slate-400 text-[11px]">
          <div class="i-mdi-chevron-right text-xs transition-transform duration-200" :class="{ 'rotate-90': expandedSections.stashes }" />
          <div class="i-mdi-package-down text-xs text-indigo-500" />
          {{ t('git.stash') }}
        </div>
        <span class="text-[9px] text-slate-400 bg-slate-100/80 dark:bg-slate-800/60 px-1.5 py-0.5 rounded-full leading-none font-medium">{{ stashList.length }}</span>
      </div>
      <div v-if="expandedSections.stashes" class="pb-1 pt-0.5">
        <div v-for="stash in stashList" :key="stash.index"
          class="flex items-center gap-1.5 px-2 py-[3px] ml-4 mr-1.5 rounded hover:bg-indigo-50/60 dark:hover:bg-indigo-900/10 cursor-pointer transition-colors duration-150"
          @click="emit('open-stash')">
          <div class="w-1.5 h-1.5 rounded-full bg-indigo-400/50 dark:bg-indigo-500/40 flex-shrink-0" />
          <span class="truncate text-[11px] text-slate-600 dark:text-slate-400">{{ stash.message }}</span>
        </div>
        <div v-if="stashList.length === 0" class="px-3 py-2 ml-4 text-slate-400/60 italic text-[10px]">{{ t('git.stashEmpty') }}</div>
        <div @click="emit('open-stash')"
          class="flex items-center gap-1.5 px-2 py-[3px] ml-4 mr-1.5 rounded hover:bg-slate-100/80 dark:hover:bg-slate-800/30 cursor-pointer text-slate-400 hover:text-indigo-500 transition-colors duration-150">
          <div class="i-mdi-plus text-[10px]" />
          <span class="text-[10px]">{{ t('git.stashSave') }}</span>
        </div>
      </div>
    </div>

    <!-- Context Menu -->
    <Teleport to="body">
      <Transition name="ctx-menu">
        <div v-if="contextMenu.show"
          id="git-sidebar-ctx-menu"
          class="fixed flex flex-col min-w-[164px] py-[3px] rounded-[4px] border border-slate-300/85 dark:border-slate-600/60 bg-white/96 dark:bg-[#141d2e]/96 shadow-[0_8px_18px_rgba(15,23,42,0.12)] dark:shadow-[0_10px_24px_rgba(2,6,23,0.48)] backdrop-blur-md text-[12px]"
          :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px', zIndex: 99999, isolation: 'isolate' }">

          <!-- Branch context menu: non-current -->
          <template v-if="(contextMenu.type === 'local' || contextMenu.type === 'remote') && !contextMenu.isCurrent">
            <div @click="handleCheckout" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.checkoutBranch', { name: contextMenu.name }) }}</div>
            <div class="ctx-sep" />
            <div @click="handleMergeBranch" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.mergeBranchTo', { name: contextMenu.name }) }}</div>
            <div @click="handleRebaseBranch" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.rebaseBranchTo', { name: contextMenu.name }) }}</div>
            <div class="ctx-sep" />
            <div @click="handleFetchCurrent" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.fetchBranch', { name: contextMenu.name }) }}</div>
            <div @click="handlePullBranch" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.pullBranch') }}</div>
            <template v-if="contextMenu.type === 'local'">
              <div class="ctx-sep" />
              <div @click="handleRenameBranch" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.renameBranch') }}</div>
              <div @click="handleDeleteBranch" class="ctx-action ctx-danger w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-red-600 dark:text-red-400 hover:bg-red-500/12 dark:hover:bg-red-500/20 hover:text-red-700 dark:hover:text-red-200 cursor-pointer transition-colors truncate">{{ t('git.deleteBranch') }}</div>
            </template>
          </template>

          <!-- Branch context menu: current branch -->
          <template v-if="(contextMenu.type === 'local') && contextMenu.isCurrent">
            <div @click="handleFetchCurrent" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.fetchBranch', { name: contextMenu.name }) }}</div>
            <div @click="handlePullCurrent" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.pullCurrentBranch') }}</div>
            <div @click="handlePushCurrent" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.pushCurrentBranch') }}</div>
            <div class="ctx-sep" />
            <div @click="handleRenameBranch" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.renameBranch') }}</div>
          </template>

          <!-- Tag context menu -->
          <template v-if="contextMenu.type === 'tag'">
            <div @click="handleCheckout" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.checkoutTag') }}</div>
            <div class="ctx-sep" />
            <div @click="handleDeleteTag" class="ctx-action ctx-danger w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-red-600 dark:text-red-400 hover:bg-red-500/12 dark:hover:bg-red-500/20 hover:text-red-700 dark:hover:text-red-200 cursor-pointer transition-colors truncate">{{ t('git.deleteTag') }}</div>
          </template>

          <!-- Remote repo context menu -->
          <template v-if="contextMenu.type === 'remoteRepo'">
            <div @click="handleFetchRemote" class="ctx-action w-full min-h-[26px] px-3 py-[5px] text-left text-[12px] font-normal text-slate-600 dark:text-slate-300 hover:bg-slate-400/15 dark:hover:bg-slate-600/45 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer transition-colors truncate">{{ t('git.fetchRemote') }}</div>
          </template>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
:global(#git-sidebar-ctx-menu) {
  min-width: 220px !important;
  padding: 3px 0 !important;
  border-radius: 2px !important;
  border: none !important;
  background: #f3f4f6 !important;
  box-shadow: 0 4px 10px rgba(15, 23, 42, 0.12) !important;
  backdrop-filter: none !important;
}

:global(#git-sidebar-ctx-menu .ctx-action) {
  display: block;
  width: 100%;
  min-height: 24px;
  padding: 0 14px;
  text-align: left;
  font-size: 12px;
  font-weight: 400;
  line-height: 24px;
  color: #1f2937 !important;
  background: transparent !important;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition: background-color 80ms ease;
}

:global(#git-sidebar-ctx-menu .ctx-action:hover) {
  background: #e5e7eb !important;
}

:global(#git-sidebar-ctx-menu .ctx-danger),
:global(#git-sidebar-ctx-menu .ctx-danger:hover) {
  color: #1f2937 !important;
}

:global(#git-sidebar-ctx-menu .ctx-sep) {
  border-top: 1px solid #d1d5db;
  margin: 3px 0;
}

:global(.dark #git-sidebar-ctx-menu) {
  border: none !important;
  background: #2f343c !important;
  box-shadow: 0 8px 18px rgba(2, 6, 23, 0.52) !important;
}

:global(.dark #git-sidebar-ctx-menu .ctx-action) {
  color: #e5e7eb !important;
}

:global(.dark #git-sidebar-ctx-menu .ctx-action:hover) {
  background: #3d4450 !important;
}

:global(.dark #git-sidebar-ctx-menu .ctx-sep) {
  border-top-color: #4b5563;
}

.ctx-menu-enter-active {
  transition: opacity 120ms ease-out, transform 120ms ease-out;
}
.ctx-menu-leave-active {
  transition: opacity 80ms ease-in, transform 80ms ease-in;
}
.ctx-menu-enter-from,
.ctx-menu-leave-to {
  opacity: 0;
  transform: scale(0.98);
}

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
