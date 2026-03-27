<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { api } from '../api';
import type { Project, CustomCommand } from '../types';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { normalizeNvmVersion, findInstalledNodeVersion } from '../utils/nvm';

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
  type: 'node' | 'other';
  nodeVersion: string;
  packageManager: 'npm' | 'yarn' | 'pnpm' | 'cnpm';
  scripts: string[];
  customCommands: CustomCommand[];
}>({
  id: '',
  name: '',
  path: '',
  type: 'node',
  nodeVersion: '',
  packageManager: 'npm',
  scripts: [],
  customCommands: []
});

const nodeVersions = ref<string[]>([]);
const loading = ref(false);

function resetForm() {
    form.value = {
        id: '',
        name: '',
        path: '',
        type: 'node',
        nodeVersion: nodeVersions.value[0] || '',
        packageManager: 'npm',
        scripts: [],
        customCommands: []
    };
}

watch(() => props.modelValue, (val) => {
  if (val) {
      if (props.editProject) {
          form.value = {
              ...props.editProject,
              nodeVersion: props.editProject.nodeVersion || '',
              packageManager: props.editProject.packageManager || 'npm',
              scripts: props.editProject.scripts || [],
              customCommands: props.editProject.customCommands
                  ? props.editProject.customCommands.map(c => ({ ...c }))
                  : []
          };
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
        
        // Set type based on scan result
        if (info.projectType === 'node') {
            form.value.type = 'node';
            if (info.packageManager) {
                form.value.packageManager = info.packageManager;
            }
            form.value.scripts = info.scripts;

            const normalizedNvmVersion = normalizeNvmVersion(info.nvmVersion);
            if (normalizedNvmVersion) {
              const installed = findInstalledNodeVersion(nodeVersions.value, normalizedNvmVersion);

              if (installed) {
                form.value.nodeVersion = installed;
              } else {
                try {
                  ElMessage.info(t('project.autoInstallStart', { version: normalizedNvmVersion }));
                  await api.installNode(normalizedNvmVersion);
                  ElMessage.success(t('project.autoInstallSuccess', { version: normalizedNvmVersion }));
                  const list = await api.getNvmList();
                  nodeVersions.value = list.map(v => v.version);

                  const newInstalled = findInstalledNodeVersion(nodeVersions.value, normalizedNvmVersion);
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
        } else {
            form.value.type = 'other';
            form.value.scripts = [];
        }
      } catch (e) {
        console.error('Failed to scan project', e);
        // Even if scan fails, still set the name from path
        if (!form.value.name) {
            form.value.name = selected.split(/[/\\]/).pop() || '';
        }
        form.value.type = 'other';
      } finally {
        loading.value = false;
      }
    }
  } catch (err) {
    console.error('Failed to open dialog:', err);
  }
}

function addCustomCommand() {
    form.value.customCommands.push({
        id: crypto.randomUUID(),
        name: '',
        command: ''
    });
}

function removeCustomCommand(index: number) {
    form.value.customCommands.splice(index, 1);
}

function submit() {
  if (!form.value.name || !form.value.path) return;
  
  const project: Project = {
    id: isEdit.value ? form.value.id : crypto.randomUUID(),
    name: form.value.name,
    path: form.value.path,
    type: form.value.type,
  };

  if (form.value.type === 'node') {
    project.nodeVersion = form.value.nodeVersion;
    project.packageManager = form.value.packageManager;
    project.scripts = form.value.scripts;
  }

  if (form.value.customCommands.length > 0) {
    project.customCommands = form.value.customCommands.filter(c => c.name && c.command);
  }
  
  if (isEdit.value) {
      emit('update', project);
  } else {
      emit('add', project);
  }
  visible.value = false;
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

        <el-form-item :label="t('project.type')">
            <el-select v-model="form.type" class="w-full">
                <el-option label="Node" value="node" />
                <el-option :label="t('project.typeOther')" value="other" />
            </el-select>
        </el-form-item>

        <template v-if="form.type === 'node'">
            <el-row :gutter="20">
                <el-col :span="12">
                    <el-form-item :label="t('project.nodeVersion')">
                        <el-select v-model="form.nodeVersion">
                            <el-option :label="t('nodes.select')" value="" />
                            <el-option v-for="v in nodeVersions" :key="v" :label="v" :value="v" />
                        </el-select>
                    </el-form-item>
                </el-col>
                <el-col :span="12">
                    <el-form-item :label="t('project.packageManager')">
                        <el-select v-model="form.packageManager">
                            <el-option label="npm" value="npm" />
                            <el-option label="yarn" value="yarn" />
                            <el-option label="pnpm" value="pnpm" />
                            <el-option label="cnpm" value="cnpm" />
                        </el-select>
                    </el-form-item>
                </el-col>
            </el-row>
        </template>

        <!-- Custom Commands for non-Node projects (also available for Node if desired) -->
        <template v-if="form.type !== 'node'">
            <el-form-item :label="t('project.customCommands')">
                <div class="w-full space-y-2">
                    <div v-for="(cmd, index) in form.customCommands" :key="cmd.id" class="flex gap-2 items-center">
                        <el-input v-model="cmd.name" :placeholder="t('project.commandName')" class="w-1/3" />
                        <el-input v-model="cmd.command" :placeholder="t('project.commandContent')" class="flex-1" />
                        <el-button type="danger" text @click="removeCustomCommand(index)">
                            <el-icon><div class="i-mdi-close" /></el-icon>
                        </el-button>
                    </div>
                    <el-button type="primary" text @click="addCustomCommand">
                        <el-icon class="mr-1"><div class="i-mdi-plus" /></el-icon>
                        {{ t('project.addCommand') }}
                    </el-button>
                </div>
            </el-form-item>
        </template>
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
