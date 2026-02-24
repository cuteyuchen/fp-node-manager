<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useSettingsStore } from '../stores/settings';
import { useProjectStore } from '../stores/project';
import { useNodeStore } from '../stores/node';
import { api } from '../api';
import { ElMessage } from 'element-plus';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const settingsStore = useSettingsStore();
const projectStore = useProjectStore();
const nodeStore = useNodeStore();
const appVersion = ref('');
const target = import.meta.env.VITE_TARGET;
const contextMenuEnabled = ref(false);
const contextMenuSupported = ref(false);
const autoLaunchEnabled = ref(false);

onMounted(async () => {
    appVersion.value = await api.getAppVersion();
    // Use cached terminals or empty first
    if (settingsStore.availableTerminals.length > 0) {
        // If already cached, no need to fetch immediately
    } else {
        // We can fetch on demand, or fetch here if we want to pre-populate but non-blocking?
        // User requested: "Fetch when software runs AND when clicking dropdown".
        // If we fetch here, it's "when opening settings", not "software runs".
        // But let's respect "don't run every time settings opens".
        // We will move the initial fetch to App.vue or store init, but for now, check if empty.
        // Actually, let's just use the store's fetch which handles caching.
        settingsStore.fetchAvailableTerminals();
    }
    
    if (target !== 'utools') {
        contextMenuSupported.value = await api.isContextMenuSupported();
        if (contextMenuSupported.value) {
            contextMenuEnabled.value = await api.checkContextMenu();
        }
        const autostart = await import('@tauri-apps/plugin-autostart');
        autoLaunchEnabled.value = await autostart.isEnabled();
    }
});

async function toggleContextMenu(val: boolean) {
    try {
        await api.setContextMenu(val, settingsStore.settings.locale);
        ElMessage.success(val ? t('common.success') : t('common.success'));
    } catch (e) {
        ElMessage.error(t('common.error') + ': ' + e);
        contextMenuEnabled.value = !val; // revert
    }
}

async function toggleAutoLaunch(val: boolean) {
    try {
        const autostart = await import('@tauri-apps/plugin-autostart');
        if (val) {
            await autostart.enable();
        } else {
            await autostart.disable();
        }
        ElMessage.success(t('common.success'));
    } catch (e) {
        ElMessage.error(t('common.error') + ': ' + e);
        autoLaunchEnabled.value = !val; // revert
    }
}

async function selectEditor() {
    try {
        const selected = await api.openDialog({
            multiple: false,
            filters: [{
                name: 'Executable',
                extensions: ['exe', 'cmd', 'bat']
            }]
        });
        if (selected && typeof selected === 'string') {
            settingsStore.settings.editorPath = selected;
        }
    } catch (e) {
        console.error(e);
    }
}

async function exportData() {
    try {
        const path = await api.saveDialog({
            filters: [{
                name: 'JSON',
                extensions: ['json']
            }],
            defaultPath: 'frontend-manager-backup.json'
        });

        if (path) {
            const data = {
                projects: projectStore.projects,
                settings: settingsStore.settings,
                customNodes: nodeStore.versions.filter(v => v.source === 'custom')
            };
            await api.writeTextFile(path, JSON.stringify(data, null, 2));
            ElMessage.success(t('settings.exportSuccess'));
        }
    } catch (e) {
        console.error(e);
        ElMessage.error(`${t('settings.exportError')}: ${e}`);
    }
}

async function importData() {
    try {
        const path = await api.openDialog({
            multiple: false,
            filters: [{
                name: 'JSON',
                extensions: ['json']
            }]
        });

        if (path && typeof path === 'string') {
            const content = await api.readTextFile(path);
            const data = JSON.parse(content);
            
            if (data.projects) projectStore.projects = data.projects;
            if (data.settings) settingsStore.settings = data.settings;
            if (data.customNodes) {
                // Merge custom nodes
                const existing = new Set(nodeStore.versions.map(v => v.path));
                data.customNodes.forEach((n: any) => {
                    if (!existing.has(n.path)) {
                        nodeStore.versions.push(n);
                    }
                });
            }
            ElMessage.success(t('settings.importSuccess'));
        }
    } catch (e) {
        console.error(e);
        ElMessage.error(`${t('settings.importError')}: ${e}`);
    }
}

function openReleases() {
    api.openUrl('https://github.com/cuteyuchen/fp-node-manager/releases');
}
</script>

<template>
  <div class="p-6 h-full flex flex-col overflow-y-auto">
    <h1 class="text-2xl font-bold text-slate-900 dark:text-white mb-6">{{ t('settings.title') }}</h1>
    
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 pb-20">
        <el-card class="!bg-white dark:!bg-gray-800 !border-gray-200 dark:!border-gray-700 shadow-sm h-full flex flex-col">
            <template #header>
                <div class="font-bold">{{ t('settings.general') }}</div>
            </template>
            <el-form label-position="top">
                <el-form-item :label="t('settings.editorPath')">
                    <div class="flex gap-2 w-full">
                        <el-input v-model="settingsStore.settings.editorPath" :placeholder="t('settings.editorPathPlaceholder')">
                            <template #prepend>
                                <el-icon><div class="i-mdi-console" /></el-icon>
                            </template>
                            <template #append>
                                <el-button @click="selectEditor">{{ t('settings.selectFile') }}</el-button>
                            </template>
                        </el-input>
                    </div>
                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        {{ t('settings.editorPathHint') }}
                    </div>
                </el-form-item>

                <el-form-item :label="t('settings.defaultTerminal')">
                    <el-select 
                        v-model="settingsStore.settings.defaultTerminal" 
                        class="w-full"
                    >
                        <el-option 
                            v-for="term in settingsStore.availableTerminals" 
                            :key="term.id" 
                            :label="term.name" 
                            :value="term.id"
                        />
                    </el-select>
                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        {{ t('settings.terminalHint') }}
                    </div>
                </el-form-item>

                <el-form-item :label="t('settings.contextMenu')" v-if="target !== 'utools' && contextMenuSupported">
                    <el-switch v-model="contextMenuEnabled" @change="toggleContextMenu" />
                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        {{ t('settings.contextMenuHint') }}
                    </div>
                </el-form-item>

                <el-form-item :label="t('settings.autoLaunch')" v-if="target !== 'utools'">
                    <el-switch v-model="autoLaunchEnabled" @change="toggleAutoLaunch" />
                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        {{ t('settings.autoLaunchHint') }}
                    </div>
                </el-form-item>
            </el-form>
        </el-card>

        <el-card class="!bg-white dark:!bg-gray-800 !border-gray-200 dark:!border-gray-700 shadow-sm h-full flex flex-col">
            <template #header>
                <div class="font-bold">{{ t('settings.appearance') }}</div>
            </template>
            <el-form label-position="top">
                <el-form-item :label="t('settings.language')">
                    <el-select v-model="settingsStore.settings.locale" class="w-full">
                        <el-option label="中文" value="zh" />
                        <el-option label="English" value="en" />
                    </el-select>
                </el-form-item>

                <el-form-item :label="t('settings.theme')">
                    <el-select v-model="settingsStore.settings.themeMode" class="w-full">
                        <el-option :label="t('settings.themeMode.dark')" value="dark" />
                        <el-option :label="t('settings.themeMode.light')" value="light" />
                        <el-option :label="t('settings.themeMode.system')" value="auto" />
                    </el-select>
                </el-form-item>
            </el-form>
        </el-card>

        <el-card class="!bg-white dark:!bg-gray-800 !border-gray-200 dark:!border-gray-700 shadow-sm h-full flex flex-col">
            <template #header>
                <div class="font-bold">{{ t('settings.update') }}</div>
            </template>
            <el-form label-position="top">
                <el-form-item :label="t('settings.version')">
                    <el-tag type="info" effect="plain" round>v{{ appVersion }}</el-tag>
                </el-form-item>

                <el-form-item :label="t('settings.autoUpdate')" v-if="target !== 'utools'">
                    <el-switch v-model="settingsStore.settings.autoUpdate" />
                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        {{ t('settings.autoUpdateHint') }}
                    </div>
                </el-form-item>
                
                <div class="mt-4">
                    <div class="text-sm font-medium mb-2">{{ t('settings.releases') }}</div>
                    <el-button link type="primary" @click="openReleases">
                        https://github.com/cuteyuchen/fp-node-manager/releases
                        <el-icon class="ml-1"><div class="i-mdi-open-in-new" /></el-icon>
                    </el-button>
                </div>
            </el-form>
        </el-card>

        <el-card class="!bg-white dark:!bg-gray-800 !border-gray-200 dark:!border-gray-700 shadow-sm h-full flex flex-col">
            <template #header>
                <div class="font-bold">{{ t('settings.data') }}</div>
            </template>
            <div class="flex gap-4">
                <el-button type="primary" @click="exportData">
                    <el-icon class="mr-1"><div class="i-mdi-export" /></el-icon>
                    {{ t('settings.export') }}
                </el-button>
                <el-button @click="importData">
                    <el-icon class="mr-1"><div class="i-mdi-import" /></el-icon>
                    {{ t('settings.import') }}
                </el-button>
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-400 mt-2">
                {{ t('settings.dataHint') }}
            </div>
        </el-card>
    </div>
  </div>
</template>
