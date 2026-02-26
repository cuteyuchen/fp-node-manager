<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { api } from '../api';
import type { Project } from '../types';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';

const { t } = useI18n();
const props = defineProps<{ 
    modelValue: boolean,
    editProject?: Project | null
}>();
const emit = defineEmits(['update:modelValue', 'add', 'update']);

const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
});

const isEdit = computed(() => !!props.editProject);

const form = ref<{
  id: string;
  name: string;
  path: string;
  type: 'node' | 'static';
  nodeVersion: string;
  packageManager: 'npm' | 'yarn' | 'pnpm' | 'cnpm';
  scripts: string[];
}>({
  id: '',
  name: '',
  path: '',
  type: 'node',
  nodeVersion: '',
  packageManager: 'npm',
  scripts: []
});

const nodeVersions = ref<string[]>([]);
const loading = ref(false);

function normalizeNvmVersion(rawVersion?: string | null): string | null {
  if (!rawVersion) return null;
  const trimmed = rawVersion.trim();
  if (!trimmed) return null;

  const normalized = trimmed.toLowerCase().startsWith('v') ? trimmed.slice(1) : trimmed;
  if (!/^\d+(\.\d+){0,2}$/.test(normalized)) {
    return null;
  }

  return normalized;
}

function findInstalledNodeVersion(targetVersion: string): string | undefined {
  return nodeVersions.value.find((item) => {
    const normalizedItem = item.toLowerCase().startsWith('v') ? item.slice(1) : item;
    return normalizedItem === targetVersion || normalizedItem.startsWith(`${targetVersion}.`);
  });
}

function resetForm() {
    form.value = {
        id: '',
        name: '',
        path: '',
        type: 'node',
        nodeVersion: nodeVersions.value[0] || '',
        packageManager: 'npm',
        scripts: []
    };
}

watch(() => props.modelValue, (val) => {
  if (val) {
      if (props.editProject) {
          form.value = { ...props.editProject };
      } else {
          resetForm();
      }
  }
});

onMounted(async () => {
  try {
    const list = await api.getNvmList();
    nodeVersions.value = list.map(v => v.version);
    if (nodeVersions.value.length > 0) {
      form.value.nodeVersion = nodeVersions.value[0];
    }
  } catch (e) {
    console.error('Failed to load node versions', e);
  }
});

async function selectFolder() {
  try {
    const selected = await api.openDialog({
      directory: true,
      multiple: false,
    });
    
    if (selected && typeof selected === 'string') {
      form.value.path = selected;
      // Auto scan
      try {
        loading.value = true;
        const info = await api.scanProject(selected);
        
        // Auto fill name if empty
        if (!form.value.name && info.name) {
            form.value.name = info.name;
        }
        
        if (info.packageManager) {
            form.value.packageManager = info.packageManager;
        }
        
        form.value.scripts = info.scripts;

        const normalizedNvmVersion = normalizeNvmVersion(info.nvmVersion);
        if (normalizedNvmVersion) {
          const installed = findInstalledNodeVersion(normalizedNvmVersion);

          if (installed) {
            form.value.nodeVersion = installed;
          } else {
            try {
              ElMessage.info(t('project.autoInstallStart', { version: normalizedNvmVersion }));
              await api.installNode(normalizedNvmVersion);
              ElMessage.success(t('project.autoInstallSuccess', { version: normalizedNvmVersion }));
              const list = await api.getNvmList();
              nodeVersions.value = list.map(v => v.version);

              const newInstalled = findInstalledNodeVersion(normalizedNvmVersion);
              if (newInstalled) {
                form.value.nodeVersion = newInstalled;
              }
            } catch (installErr) {
              ElMessage.error(`${t('project.autoInstallFailed', { version: normalizedNvmVersion })}: ${String(installErr)}`);
              console.error('Failed to auto-install node version', installErr);
            }
          }
        } else if (info.nvmVersion) {
          console.warn('Invalid .nvmrc version, skipping auto install', info.nvmVersion);
          ElMessage.warning(t('project.invalidNvmrc'));
        }
      } catch (e) {
        console.error('Failed to scan project', e);
      } finally {
        loading.value = false;
      }
    }
  } catch (err) {
    console.error('Failed to open dialog:', err);
  }
}

function submit() {
  if (!form.value.name || !form.value.path) return;
  
  const project: Project = {
    id: isEdit.value ? form.value.id : crypto.randomUUID(),
    name: form.value.name,
    path: form.value.path,
    type: form.value.type,
    nodeVersion: form.value.nodeVersion,
    packageManager: form.value.packageManager,
    scripts: form.value.scripts
  };
  
  if (isEdit.value) {
      emit('update', project);
  } else {
      emit('add', project);
  }
  visible.value = false;
  // Reset form will be handled by watch or next open if we clear editProject
}
</script>

<template>
  <el-dialog
    v-model="visible"
    :title="isEdit ? t('project.editProject') : t('dashboard.addProject')"
    width="500px"
    :close-on-click-modal="false"
    destroy-on-close
  >
    <el-form label-position="top" :model="form">
        <el-form-item :label="t('project.name')">
            <el-input v-model="form.name" :placeholder="t('project.namePlaceholder')" />
        </el-form-item>
        
        <el-form-item :label="t('project.path')" required>
            <div class="flex gap-2 w-full">
                <el-input v-model="form.path" :placeholder="t('project.selectFolder')" readonly>
                    <template #append>
                        <el-button @click="selectFolder">
                             <el-icon><div class="i-mdi-folder" /></el-icon>
                        </el-button>
                    </template>
                </el-input>
            </div>
        </el-form-item>

        <el-row :gutter="20">
            <el-col :span="12">
                <el-form-item :label="t('project.type')">
                    <el-select v-model="form.type">
                        <el-option label="Node" value="node" />
                        <el-option label="Static" value="static" />
                    </el-select>
                </el-form-item>
            </el-col>
            <el-col :span="12">
                <el-form-item :label="t('project.nodeVersion')">
                    <el-select v-model="form.nodeVersion">
                        <el-option :label="t('nodes.select')" value="" />
                        <el-option v-for="v in nodeVersions" :key="v" :label="v" :value="v" />
                    </el-select>
                </el-form-item>
            </el-col>
        </el-row>

        <el-form-item :label="t('project.packageManager')">
            <el-select v-model="form.packageManager">
                <el-option label="npm" value="npm" />
                <el-option label="yarn" value="yarn" />
                <el-option label="pnpm" value="pnpm" />
                <el-option label="cnpm" value="cnpm" />
            </el-select>
        </el-form-item>
    </el-form>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="visible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="submit" :disabled="!form.name || !form.path" :loading="loading">
          {{ t('common.confirm') }}
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>
