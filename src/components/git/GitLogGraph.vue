<script setup lang="ts">
import { ref, computed } from 'vue';
import { useGitStore } from '../../stores/git';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import type { Project, GitCommit } from '../../types';

const props = defineProps<{
  project: Project;
}>();

const { t } = useI18n();
const gitStore = useGitStore();

const loadCount = ref(200);
const selectedCommitHash = ref<string | null>(null);
const selectedCommit = ref<GitCommit | null>(null);

// ─── Commit detail state ────────────────────────────────────────────────
const commitDiffRaw = ref<string>('');
const commitFiles = ref<CommitFileInfo[]>([]);
const selectedFileName = ref<string>('');
const selectedFileDiff = ref<string>('');

interface CommitFileInfo {
  path: string;
  status: 'modified' | 'added' | 'deleted' | 'renamed';
  additions: number;
  deletions: number;
  rawDiff: string;
}

// ─── Resizable panel state ──────────────────────────────────────────────
const detailHeight = ref(280);
const fileListWidth = ref(300);

const commits = computed(() => gitStore.getCommits(props.project.id));

// Graph colors palette
const GRAPH_COLORS = [
  '#3b82f6', // blue
  '#22c55e', // green
  '#f59e0b', // amber
  '#ef4444', // red
  '#a855f7', // purple
  '#06b6d4', // cyan
  '#ec4899', // pink
  '#f97316', // orange
  '#14b8a6', // teal
  '#8b5cf6', // violet
];

// Build graph layout from commits
const graphData = computed(() => {
  const commitList = commits.value;
  if (!commitList.length) return { nodes: [] as GraphLayoutNode[], maxColumns: 0 };

  const nodes: GraphLayoutNode[] = [];
  const activeLanes: (string | null)[] = []; // Each lane holds the hash it's tracking
  const commitToColumn: Record<string, number> = {};
  const commitToColor: Record<string, string> = {};

  for (let i = 0; i < commitList.length; i++) {
    const commit = commitList[i];
    
    // Find which lane this commit should go in
    let column = activeLanes.indexOf(commit.hash);
    
    if (column === -1) {
      // New commit not yet in any lane - find a free lane or add new one
      const freeIdx = activeLanes.indexOf(null);
      if (freeIdx !== -1) {
        column = freeIdx;
        activeLanes[freeIdx] = commit.hash;
      } else {
        column = activeLanes.length;
        activeLanes.push(commit.hash);
      }
    }

    // Assign color based on column
    const color = commitToColor[commit.hash] || GRAPH_COLORS[column % GRAPH_COLORS.length];
    commitToColor[commit.hash] = color;
    commitToColumn[commit.hash] = column;

    // Process parents
    const parentLinks: { hash: string; column: number; color: string }[] = [];
    
    // Clear this lane (the commit has been placed)
    activeLanes[column] = null;

    for (let pi = 0; pi < commit.parents.length; pi++) {
      const parentHash = commit.parents[pi];
      let parentColumn = activeLanes.indexOf(parentHash);
      
      if (parentColumn === -1) {
        // Parent not yet in a lane
        if (pi === 0) {
          // First parent goes in the same column
          activeLanes[column] = parentHash;
          parentColumn = column;
        } else {
          // Other parents get new lanes
          const freeIdx = activeLanes.indexOf(null);
          if (freeIdx !== -1) {
            parentColumn = freeIdx;
            activeLanes[freeIdx] = parentHash;
          } else {
            parentColumn = activeLanes.length;
            activeLanes.push(parentHash);
          }
        }
      } else {
        // Parent already in a lane, will converge
      }

      const parentColor = pi === 0 ? color : GRAPH_COLORS[parentColumn % GRAPH_COLORS.length];
      if (!commitToColor[parentHash]) {
        commitToColor[parentHash] = parentColor;
      }
      
      parentLinks.push({ hash: parentHash, column: parentColumn, color: parentColor });
    }

    // Clean up empty trailing lanes
    while (activeLanes.length > 0 && activeLanes[activeLanes.length - 1] === null) {
      activeLanes.pop();
    }

    nodes.push({
      hash: commit.hash,
      column,
      color,
      row: i,
      parentLinks,
      activeLaneCount: Math.max(activeLanes.filter(l => l !== null).length, column + 1),
    });
  }

  const maxColumns = Math.max(...nodes.map(n => Math.max(n.column, ...n.parentLinks.map(p => p.column)))) + 1;

  return { nodes, maxColumns: Math.min(maxColumns, 12) };
});

interface GraphLayoutNode {
  hash: string;
  column: number;
  color: string;
  row: number;
  parentLinks: { hash: string; column: number; color: string }[];
  activeLaneCount: number;
}

const ROW_HEIGHT = 28;
const COL_WIDTH = 16;
const NODE_RADIUS = 4;
const GRAPH_PADDING = 12;

const svgWidth = computed(() => (graphData.value.maxColumns * COL_WIDTH) + GRAPH_PADDING * 2);
const svgHeight = computed(() => commits.value.length * ROW_HEIGHT + ROW_HEIGHT);

function nodeX(col: number): number {
  return GRAPH_PADDING + col * COL_WIDTH + COL_WIDTH / 2;
}

function nodeY(row: number): number {
  return ROW_HEIGHT / 2 + row * ROW_HEIGHT;
}

// Generate SVG path for parent connections
function generatePaths(): { d: string; color: string }[] {
  const paths: { d: string; color: string }[] = [];
  const { nodes } = graphData.value;
  const commitIndexMap: Record<string, number> = {};
  
  for (const node of nodes) {
    commitIndexMap[node.hash] = node.row;
  }

  for (const node of nodes) {
    const x1 = nodeX(node.column);
    const y1 = nodeY(node.row);

    for (const parent of node.parentLinks) {
      const parentRow = commitIndexMap[parent.hash];
      if (parentRow === undefined) {
        // Parent not in visible commits, draw line going down
        const x2 = nodeX(parent.column);
        const y2 = svgHeight.value;
        if (node.column === parent.column) {
          paths.push({ d: `M ${x1} ${y1} L ${x2} ${y2}`, color: parent.color });
        } else {
          const midY = y1 + ROW_HEIGHT / 2;
          paths.push({ d: `M ${x1} ${y1} C ${x1} ${midY}, ${x2} ${midY}, ${x2} ${y1 + ROW_HEIGHT}`, color: parent.color });
          paths.push({ d: `M ${x2} ${y1 + ROW_HEIGHT} L ${x2} ${y2}`, color: parent.color });
        }
        continue;
      }

      const x2 = nodeX(parent.column);
      const y2 = nodeY(parentRow);

      if (node.column === parent.column) {
        // Straight line
        paths.push({ d: `M ${x1} ${y1} L ${x2} ${y2}`, color: parent.color });
      } else {
        // Bezier curve for branch/merge
        const midY = (y1 + y2) / 2;
        paths.push({
          d: `M ${x1} ${y1} C ${x1} ${midY}, ${x2} ${midY}, ${x2} ${y2}`,
          color: parent.color,
        });
      }
    }
  }

  return paths;
}

const svgPaths = computed(() => generatePaths());

function formatDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    
    if (days === 0) {
      const hours = Math.floor(diff / (1000 * 60 * 60));
      if (hours === 0) {
        const mins = Math.floor(diff / (1000 * 60));
        return `${mins}m ago`;
      }
      return `${hours}h ago`;
    }
    if (days < 7) return `${days}d ago`;
    if (days < 30) return `${Math.floor(days / 7)}w ago`;
    
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: date.getFullYear() !== now.getFullYear() ? 'numeric' : undefined });
  } catch {
    return dateStr;
  }
}

function parseRefs(refs: string[]): { type: 'branch' | 'tag' | 'head' | 'remote'; name: string }[] {
  return refs.map(ref => {
    if (ref.startsWith('HEAD -> ')) {
      return { type: 'head', name: ref.replace('HEAD -> ', '') };
    }
    if (ref.startsWith('tag: ')) {
      return { type: 'tag', name: ref.replace('tag: ', '') };
    }
    if (ref.includes('/')) {
      return { type: 'remote', name: ref };
    }
    return { type: 'branch', name: ref };
  });
}

function getRefClass(type: string): string {
  switch (type) {
    case 'head': return 'bg-blue-500/20 text-blue-600 dark:text-blue-400 border-blue-500/30';
    case 'branch': return 'bg-green-500/20 text-green-600 dark:text-green-400 border-green-500/30';
    case 'tag': return 'bg-amber-500/20 text-amber-600 dark:text-amber-400 border-amber-500/30';
    case 'remote': return 'bg-purple-500/20 text-purple-600 dark:text-purple-400 border-purple-500/30';
    default: return 'bg-slate-200 text-slate-600';
  }
}

// ─── Parse commit diff into file list ──────────────────────────────────
function parseCommitFiles(rawDiff: string): CommitFileInfo[] {
  if (!rawDiff) return [];
  const files: CommitFileInfo[] = [];
  const sections = rawDiff.split(/(?=diff --git )/);

  for (const section of sections) {
    if (!section.startsWith('diff --git')) continue;
    const nameMatch = section.match(/^diff --git a\/(.+?) b\/(.+?)$/m);
    if (!nameMatch) continue;
    const filePath = nameMatch[2];

    let status: CommitFileInfo['status'] = 'modified';
    if (section.includes('new file mode')) status = 'added';
    else if (section.includes('deleted file mode')) status = 'deleted';
    else if (section.includes('rename from')) status = 'renamed';

    let additions = 0, deletions = 0;
    for (const line of section.split('\n')) {
      if (line.startsWith('+') && !line.startsWith('+++')) additions++;
      else if (line.startsWith('-') && !line.startsWith('---')) deletions++;
    }

    files.push({ path: filePath, status, additions, deletions, rawDiff: section });
  }
  return files;
}

// ─── Diff hunk types ────────────────────────────────────────────────────
interface DiffHunk {
  header: string;
  lines: string[];
  rawPatch: string;
  additions: number;
  deletions: number;
}

interface ParsedFileDiff {
  fileHeader: string;
  hunks: DiffHunk[];
  fileName: string;
  additions: number;
  deletions: number;
}

const parsedFileDiff = computed<ParsedFileDiff | null>(() => {
  const raw = selectedFileDiff.value;
  if (!raw) return null;

  const lines = raw.split('\n');
  let fileHeader = '';
  const hunks: DiffHunk[] = [];
  let currentHunk: DiffHunk | null = null;
  let headerDone = false;

  for (const line of lines) {
    if (!headerDone) {
      if (line.startsWith('@@')) {
        headerDone = true;
      } else {
        fileHeader += line + '\n';
        continue;
      }
    }
    if (line.startsWith('@@')) {
      if (currentHunk) hunks.push(currentHunk);
      currentHunk = { header: line, lines: [line], rawPatch: '', additions: 0, deletions: 0 };
    } else if (currentHunk) {
      currentHunk.lines.push(line);
      if (line.startsWith('+') && !line.startsWith('+++')) currentHunk.additions++;
      else if (line.startsWith('-') && !line.startsWith('---')) currentHunk.deletions++;
    }
  }
  if (currentHunk) hunks.push(currentHunk);

  for (const hunk of hunks) {
    hunk.rawPatch = fileHeader + hunk.lines.join('\n') + '\n';
  }

  let totalAdd = 0, totalDel = 0;
  for (const h of hunks) { totalAdd += h.additions; totalDel += h.deletions; }

  const nameMatch = fileHeader.match(/^diff --git a\/(.+?) b\//m);
  const fileName = nameMatch ? nameMatch[1] : selectedFileName.value;

  return { fileHeader, hunks, fileName, additions: totalAdd, deletions: totalDel };
});

// ─── Line number helpers ─────────────────────────────────────────────────
interface NumberedLine {
  oldNum: number | null;
  newNum: number | null;
  text: string;
  type: 'add' | 'del' | 'context' | 'header';
}

function computeLineNumbers(lines: string[]): NumberedLine[] {
  const result: NumberedLine[] = [];
  let oldLine = 0, newLine = 0;

  for (const line of lines) {
    if (line.startsWith('@@')) {
      const match = line.match(/@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
      if (match) {
        oldLine = parseInt(match[1]) - 1;
        newLine = parseInt(match[2]) - 1;
      }
      result.push({ oldNum: null, newNum: null, text: line, type: 'header' });
    } else if (line.startsWith('+') && !line.startsWith('+++')) {
      newLine++;
      result.push({ oldNum: null, newNum: newLine, text: line.substring(1), type: 'add' });
    } else if (line.startsWith('-') && !line.startsWith('---')) {
      oldLine++;
      result.push({ oldNum: oldLine, newNum: null, text: line.substring(1), type: 'del' });
    } else {
      oldLine++;
      newLine++;
      result.push({ oldNum: oldLine, newNum: newLine, text: line.startsWith(' ') ? line.substring(1) : line, type: 'context' });
    }
  }
  return result;
}

// ─── Status helpers ──────────────────────────────────────────────────────
function getStatusColor(status: string): string {
  const map: Record<string, string> = {
    modified: 'text-amber-500', added: 'text-green-500',
    deleted: 'text-red-500', renamed: 'text-purple-500',
  };
  return map[status] || 'text-slate-400';
}

function getStatusLetter(status: string): string {
  const map: Record<string, string> = { modified: 'M', added: 'A', deleted: 'D', renamed: 'R' };
  return map[status] || '?';
}

function getFileName(path: string): string {
  return path.split('/').pop() || path;
}

function getFileDir(path: string): string {
  const parts = path.split('/');
  parts.pop();
  return parts.length > 0 ? parts.join('/') + '/' : '';
}

// ─── Commit actions ──────────────────────────────────────────────────────
async function handleSelectCommit(commit: GitCommit) {
  selectedCommitHash.value = commit.hash;
  selectedCommit.value = commit;
  selectedFileName.value = '';
  selectedFileDiff.value = '';

  try {
    const rawDiff = await gitStore.getDiffCommit(props.project.path, commit.hash);
    commitDiffRaw.value = rawDiff;
    commitFiles.value = parseCommitFiles(rawDiff);
  } catch (e) {
    console.error('Failed to get commit diff:', e);
    commitDiffRaw.value = '';
    commitFiles.value = [];
  }
}

function handleSelectFile(file: CommitFileInfo) {
  selectedFileName.value = file.path;
  selectedFileDiff.value = file.rawDiff;
}

async function handleRevertHunk(hunk: DiffHunk) {
  try {
    await ElMessageBox.confirm(
      t('git.revertHunkConfirm'),
      t('git.revertHunk'),
      { confirmButtonText: t('common.confirm'), cancelButtonText: t('common.cancel'), type: 'warning' }
    );
    await gitStore.applyPatch(props.project.id, props.project.path, hunk.rawPatch, false, true);
    ElMessage.success(t('git.revertSuccess'));
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(t('git.operationFailed', { error: String(e) }));
  }
}

async function handleCopyHash() {
  if (!selectedCommit.value) return;
  try {
    await navigator.clipboard.writeText(selectedCommit.value.hash);
    ElMessage.success(t('git.copyHashSuccess'));
  } catch {
    ElMessage.error(t('git.operationFailed', { error: 'Clipboard not available' }));
  }
}

async function handleLoadMore() {
  loadCount.value += 200;
  await gitStore.refreshLog(props.project.id, props.project.path, loadCount.value);
}

// ─── Scroll to branch (called from sidebar click) ───────────────────────────
const scrollContainerRef = ref<HTMLElement | null>(null);

function findRefIndex(targetRef: string): number {
  const target = targetRef.replace(/^HEAD -> /, '');
  const tagTarget = target.startsWith('tag: ') ? target : `tag: ${target}`;
  return commits.value.findIndex(c =>
    c.refs.some(r => {
      const cleanRef = r.replace(/^HEAD -> /, '');
      return (
        cleanRef === target ||
        cleanRef === tagTarget ||
        cleanRef.endsWith('/' + target) ||
        cleanRef.endsWith('/' + tagTarget)
      );
    })
  );
}

async function scrollToBranch(branchName: string) {
  let idx = findRefIndex(branchName);

  if (idx === -1 && props.project) {
    const attempts = [800, 2000, 5000];
    for (const count of attempts) {
      await gitStore.refreshLog(props.project.id, props.project.path, count);
      idx = findRefIndex(branchName);
      if (idx !== -1) break;
    }
  }

  if (idx === -1) return;
  const y = idx * ROW_HEIGHT;
  scrollContainerRef.value?.scrollTo({ top: Math.max(0, y - 80), behavior: 'smooth' });
  selectedCommitHash.value = commits.value[idx].hash;
}

defineExpose({ scrollToBranch });

// ─── Drag resize ─────────────────────────────────────────────────────────
let dragging: 'detail' | 'fileList' | null = null;
let startPos = 0;
let startSize = 0;

function onDragStart(type: typeof dragging, e: MouseEvent) {
  dragging = type;
  startPos = type === 'detail' ? e.clientY : e.clientX;
  startSize = type === 'detail' ? detailHeight.value : fileListWidth.value;
  document.addEventListener('mousemove', onDragMove);
  document.addEventListener('mouseup', onDragEnd);
  document.body.style.cursor = type === 'detail' ? 'row-resize' : 'col-resize';
  document.body.style.userSelect = 'none';
}

function onDragMove(e: MouseEvent) {
  if (!dragging) return;
  if (dragging === 'detail') {
    const delta = startPos - e.clientY;
    detailHeight.value = Math.max(120, Math.min(600, startSize + delta));
  } else {
    const delta = e.clientX - startPos;
    fileListWidth.value = Math.max(200, Math.min(500, startSize + delta));
  }
}

function onDragEnd() {
  dragging = null;
  document.removeEventListener('mousemove', onDragMove);
  document.removeEventListener('mouseup', onDragEnd);
  document.body.style.cursor = '';
  document.body.style.userSelect = '';
}
</script>

<template>
  <div class="h-full flex flex-col overflow-hidden">
    <!-- Top: Commit graph list -->
    <div ref="scrollContainerRef" class="overflow-auto custom-scrollbar"
      :style="selectedCommit ? { flex: 'none', height: `calc(100% - ${detailHeight}px - 3px)` } : { flex: '1' }">

      <div v-if="commits.length === 0" class="flex items-center justify-center h-full text-slate-400">
        <div class="text-center">
          <div class="i-mdi-source-commit text-5xl opacity-20 mx-auto mb-3" />
          <p class="text-sm">{{ t('git.noCommits') }}</p>
        </div>
      </div>

      <div v-else class="relative" :style="{ minHeight: svgHeight + 'px' }">
        <!-- SVG graph layer -->
        <svg class="absolute top-0 left-0 pointer-events-none" 
          :width="svgWidth" :height="svgHeight"
          :viewBox="`0 0 ${svgWidth} ${svgHeight}`">
          <path v-for="(path, i) in svgPaths" :key="'path-' + i"
            :d="path.d" :stroke="path.color" stroke-width="2" fill="none" stroke-linecap="round" />
          <circle v-for="node in graphData.nodes" :key="'node-' + node.hash"
            :cx="nodeX(node.column)" :cy="nodeY(node.row)" :r="NODE_RADIUS"
            :fill="node.color" stroke="white" stroke-width="1.5" class="dark:stroke-slate-900" />
        </svg>

        <!-- Commit rows -->
        <div v-for="commit in commits" :key="commit.hash"
          @click="handleSelectCommit(commit)"
          class="flex items-center h-7 hover:bg-slate-100/60 dark:hover:bg-slate-800/20 cursor-pointer transition-all duration-150 text-xs border-b border-slate-100/80 dark:border-slate-800/20"
          :class="{ 'bg-blue-50/70 dark:bg-blue-900/8 ring-1 ring-inset ring-blue-500/10': selectedCommitHash === commit.hash }"
          :style="{ paddingLeft: svgWidth + 'px' }">
          <span class="w-16 font-mono text-[9px] text-blue-500/80 flex-shrink-0 px-2">{{ commit.short_hash }}</span>
          <div class="flex items-center gap-1 flex-shrink-0 mr-2">
            <span v-for="ref in parseRefs(commit.refs)" :key="ref.name"
              class="px-1.5 py-0 rounded-md text-[8px] font-medium border" :class="getRefClass(ref.type)">
              {{ ref.name }}
            </span>
          </div>
          <span class="flex-1 truncate text-slate-700 dark:text-slate-300 min-w-0 text-[11px]">{{ commit.message }}</span>
          <span class="w-24 truncate text-slate-400/70 text-[9px] px-2 flex-shrink-0 text-right">{{ commit.author }}</span>
          <span class="w-20 text-slate-400/50 text-[9px] px-2 flex-shrink-0 text-right font-mono">{{ formatDate(commit.date) }}</span>
        </div>

        <!-- Load more -->
        <div v-if="commits.length >= loadCount" @click="handleLoadMore"
          class="flex items-center justify-center py-2.5 text-[11px] text-blue-500/80 hover:text-blue-600 cursor-pointer hover:bg-slate-50/60 dark:hover:bg-slate-800/20 transition-all duration-150">
          <div class="i-mdi-dots-horizontal mr-1 text-xs" />
          {{ t('git.loadMore') }}
        </div>
      </div>
    </div>

    <!-- Drag handle between top and bottom -->
    <div v-if="selectedCommit"
      class="h-[3px] hover:bg-blue-400/40 active:bg-blue-500/50 cursor-row-resize transition-colors duration-150 shrink-0 bg-slate-200/40 dark:bg-slate-700/20"
      @mousedown="onDragStart('detail', $event)" />

    <!-- Bottom: Commit detail panel -->
    <div v-if="selectedCommit" class="flex overflow-hidden shrink-0"
      :style="{ height: detailHeight + 'px' }">

      <!-- Left: Commit info + file list -->
      <div class="flex flex-col overflow-hidden border-r border-slate-200/40 dark:border-slate-700/20"
        :style="{ width: fileListWidth + 'px', minWidth: fileListWidth + 'px' }">

        <!-- Commit info -->
        <div class="p-3 border-b border-slate-200/40 dark:border-slate-700/20 shrink-0 text-xs">
          <div class="flex items-center justify-between mb-1.5">
            <span class="font-medium text-slate-500 dark:text-slate-400 text-[10px] flex items-center gap-1">
              <div class="i-mdi-information-outline text-xs text-blue-500/60" />
              {{ t('git.commitDetail') }}
            </span>
            <button @click="handleCopyHash"
              class="text-[9px] text-blue-500/70 hover:text-blue-600 cursor-pointer px-1.5 py-0.5 rounded-md hover:bg-blue-50/60 dark:hover:bg-blue-900/15 transition-all duration-150"
              :title="t('git.copyHash')">
              <div class="i-mdi-content-copy text-[10px]" />
            </button>
          </div>
          <div class="space-y-1 text-[10px]">
            <div class="flex gap-2">
              <span class="text-slate-400/70 w-10 shrink-0">{{ t('git.hash') }}</span>
              <span class="font-mono text-blue-500/80 truncate">{{ selectedCommit.short_hash }}</span>
            </div>
            <div class="flex gap-2">
              <span class="text-slate-400/70 w-10 shrink-0">{{ t('git.author') }}</span>
              <span class="text-slate-700 dark:text-slate-300 truncate">{{ selectedCommit.author }}</span>
            </div>
            <div class="flex gap-2">
              <span class="text-slate-400/70 w-10 shrink-0">{{ t('git.date') }}</span>
              <span class="text-slate-500/80 truncate">{{ formatDate(selectedCommit.date) }}</span>
            </div>
            <div class="flex gap-2">
              <span class="text-slate-400/70 w-10 shrink-0">{{ t('git.message') }}</span>
              <span class="text-slate-700 dark:text-slate-300 truncate">{{ selectedCommit.message }}</span>
            </div>
          </div>
        </div>

        <!-- Changed files header -->
        <div class="flex items-center justify-between px-3 py-1 bg-white/40 dark:bg-[#141d2e]/30 shrink-0 border-b border-slate-200/40 dark:border-slate-700/20">
          <span class="font-medium text-[11px] text-slate-600 dark:text-slate-400 flex items-center gap-1">
            <div class="i-mdi-file-document-multiple-outline text-xs text-blue-500/60" />
            {{ t('git.changedFiles') }}
            <span class="text-[9px] bg-blue-500/8 text-blue-600 dark:text-blue-400 px-1.5 py-0.5 rounded-full font-bold leading-none">
              {{ commitFiles.length }}
            </span>
          </span>
        </div>

        <!-- File list -->
        <div class="flex-1 overflow-y-auto custom-scrollbar">
          <div v-for="file in commitFiles" :key="file.path"
            @click="handleSelectFile(file)"
            class="flex items-center gap-1.5 px-3 py-[3px] hover:bg-slate-100/60 dark:hover:bg-slate-800/15 cursor-pointer transition-all duration-150 text-xs"
            :class="{ 'bg-blue-50/60 dark:bg-blue-900/8 ring-1 ring-inset ring-blue-500/10': selectedFileName === file.path }">
            <span class="w-4 text-center text-[9px] font-bold flex-shrink-0" :class="getStatusColor(file.status)">
              {{ getStatusLetter(file.status) }}
            </span>
            <span class="text-slate-400/50 text-[9px] truncate">{{ getFileDir(file.path) }}</span>
            <span class="truncate flex-1 min-w-0 text-slate-700 dark:text-slate-300 text-[11px]">{{ getFileName(file.path) }}</span>
            <span class="text-[8px] text-green-500/80 flex-shrink-0" v-if="file.additions > 0">+{{ file.additions }}</span>
            <span class="text-[8px] text-red-500/80 flex-shrink-0" v-if="file.deletions > 0">-{{ file.deletions }}</span>
          </div>
          <div v-if="commitFiles.length === 0" class="flex items-center justify-center py-6 text-slate-400/40 text-[11px]">
            {{ t('git.loading') }}
          </div>
        </div>
      </div>

      <!-- File list resize handle -->
      <div class="w-[3px] hover:bg-blue-400/40 active:bg-blue-500/50 cursor-col-resize transition-colors duration-150 shrink-0 bg-slate-200/40 dark:bg-slate-700/20"
        @mousedown="onDragStart('fileList', $event)" />

      <!-- Right: File diff view -->
      <div class="flex-1 flex flex-col overflow-hidden min-w-0">
        <!-- Diff header -->
        <div v-if="parsedFileDiff" class="flex items-center justify-between px-3 py-1 border-b border-slate-200/40 dark:border-slate-700/20 bg-white/40 dark:bg-[#1e293b]/40 flex-shrink-0">
          <div class="flex items-center gap-2 text-[11px]">
            <div class="i-mdi-file-compare text-xs text-blue-500" />
            <span class="font-medium text-slate-700 dark:text-slate-300 truncate max-w-[260px]">{{ parsedFileDiff.fileName }}</span>
            <span class="text-green-500 font-mono text-[10px]">+{{ parsedFileDiff.additions }}</span>
            <span class="text-red-500 font-mono text-[10px]">-{{ parsedFileDiff.deletions }}</span>
          </div>
        </div>

        <!-- Hunk list -->
        <div v-if="parsedFileDiff && parsedFileDiff.hunks.length > 0" class="flex-1 overflow-auto custom-scrollbar">
          <div v-for="(hunk, hunkIdx) in parsedFileDiff.hunks" :key="hunkIdx"
            class="border-b border-slate-200/40 dark:border-slate-700/20">
            <!-- Hunk header with revert button -->
            <div class="flex items-center justify-between px-3 py-0.5 bg-blue-50/40 dark:bg-blue-900/8 sticky top-0 z-10 border-b border-slate-200/30 dark:border-slate-700/15">
              <span class="font-mono text-[10px] text-blue-600/80 dark:text-blue-400/80 truncate flex-1">{{ hunk.header }}</span>
              <div class="flex items-center gap-1 flex-shrink-0 ml-2">
                <button @click="handleRevertHunk(hunk)"
                  class="flex items-center gap-0.5 px-1.5 py-0.5 rounded-md text-[9px] font-medium cursor-pointer transition-all duration-150 text-orange-600 dark:text-orange-400 hover:bg-orange-100/80 dark:hover:bg-orange-900/20"
                  :title="t('git.revertHunk')">
                  {{ t('git.revertHunk') }}
                </button>
              </div>
            </div>

            <!-- Hunk content -->
            <table class="w-full text-xs font-mono diff-table">
              <tbody>
                <tr v-for="(nline, lineIdx) in computeLineNumbers(hunk.lines.slice(1))" :key="lineIdx"
                  :class="{
                    'bg-green-50/80 dark:bg-green-900/10': nline.type === 'add',
                    'bg-red-50/80 dark:bg-red-900/10': nline.type === 'del',
                  }">
                  <td class="diff-line-num text-right select-none w-[50px] px-2 border-r border-slate-200/40 dark:border-slate-700/20"
                    :class="{ 'text-red-400': nline.type === 'del', 'text-green-400': nline.type === 'add' }">
                    {{ nline.oldNum ?? '' }}
                  </td>
                  <td class="diff-line-num text-right select-none w-[50px] px-2 border-r border-slate-200/40 dark:border-slate-700/20"
                    :class="{ 'text-red-400': nline.type === 'del', 'text-green-400': nline.type === 'add' }">
                    {{ nline.newNum ?? '' }}
                  </td>
                  <td class="diff-line-sign w-[18px] text-center select-none"
                    :class="{ 'text-green-600 dark:text-green-400': nline.type === 'add', 'text-red-600 dark:text-red-400': nline.type === 'del' }">
                    {{ nline.type === 'add' ? '+' : nline.type === 'del' ? '-' : '' }}
                  </td>
                  <td class="diff-line-content whitespace-pre pl-1 pr-3">{{ nline.text }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Empty state -->
        <div v-else class="flex-1 flex items-center justify-center text-slate-400 dark:text-slate-500">
          <div class="text-center">
            <div class="i-mdi-file-compare text-4xl opacity-15 mx-auto mb-3" />
            <p class="text-[11px] text-slate-400/60">{{ t('git.noFileSelected') }}</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dark circle {
  stroke: #0f172a;
}

.diff-table {
  border-collapse: collapse;
}

.diff-table td {
  padding-top: 0;
  padding-bottom: 0;
  line-height: 20px;
  font-size: 12px;
}

.diff-line-num {
  color: #94a3b8;
  font-size: 10px;
  min-width: 35px;
  user-select: none;
}

.diff-line-content {
  font-family: 'Cascadia Code', 'Fira Code', 'Source Code Pro', 'Consolas', monospace;
  tab-size: 4;
}

.custom-scrollbar::-webkit-scrollbar {
  width: 5px;
  height: 5px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 3px;
}
.dark .custom-scrollbar::-webkit-scrollbar-thumb {
  background: #334155;
}
</style>
