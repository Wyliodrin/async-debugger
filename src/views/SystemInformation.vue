<script setup lang="ts">
import {useApplicationStore} from '@/stores/application';
import {Application} from '@/types/applications';
import {computed, Ref, ref} from 'vue';
import {PencilIcon, PlayerPauseFilledIcon, PlayerPlayFilledIcon, PlusIcon, TrashIcon} from 'vue-tabler-icons';
import {listen} from '@tauri-apps/api/event';
import {useDataStore} from '@/stores/data';
import {invoke} from '@tauri-apps/api/core';
import {homeDir} from '@tauri-apps/api/path';

const applicationsStore = useApplicationStore();
const dataStore = useDataStore();
const errorMessage = ref('');
const errorSnackbar = ref(false);
const exporting = ref(false);
const lastAppStatuses = new Map<string, string>();
const toastDialog = ref(false);
const pendingExport = ref<{ title: string | undefined; pid: number } | null>(null);
const timestampsDialog = ref(false);
const exportsList = ref<Array<{ app_dir: string, title: string }>>([]);
const timestampsList = ref<Array<{ ts: string, path: string, comment_preview?: string }>>([]);
const selectedAppDir = ref('');
const selectedTimestamp = ref('');
const selectedCommentPreview = ref('');
const selectedAppTitle = ref('');
const listFromDiskDialog = ref(false);
const selectedRowId = ref<string | null>(null);

const applicationHeaders: any = ref([
  {title: "Status", align: 'center', key: 'connection_status'},
  {title: "Name", align: 'center', key: 'title'},
  {title: "PID", align: 'center', key: 'pid'},
  {title: "URL", align: 'center', key: 'url'},
  {title: "State", align: 'center', key: 'state'},
  {title: "Start Time", align: 'center', key: 'start_time'},
  {title: "CPU", align: 'center', key: 'cpu_usage'},
  {title: "Memory", align: 'center', key: 'memory_usage'},
  {title: "Actions", align: 'center', key: 'actions', sortable: false}
]);

const diskHeaders = [
  {title: '', align: 'center', key: 'select', sortable: false},
  {title: ' Available Traces ', align: 'left', key: 'title'},
  {title: '', align: 'center', key: 'space', sortable: false},
  {title: '', align: 'center', key: 'space2', sortable: false},
  {title: '', align: 'center', key: 'space3', sortable: false},
  {title: '', align: 'center', key: 'space4', sortable: false},
  {title: '', align: 'center', key: 'space5', sortable: false},
];

const getStateChipColor = (state: string): string => {
  const colorMap: Record<string, string> = {
    'Enabled': 'green',
    'Disabled': 'red',
  };
  return colorMap[state] || 'default';
};

const getRowProps = (item: any) => {
  return {
    style: item.item.state === 'Disabled'
        ? {backgroundColor: '#F5F5F5'}
        : {},
  };
};

const editedItem: Ref<{
  connection_status: string,
  pid: any;
  state: any;
  startTime: any;
  cpu_usage: any;
  memory_usage: any;
  processStatus: any;
  id: string,
  title: string,
  url: string,
}> = ref({
  connection_status: '',
  id: '',
  title: '',
  url: '',
  pid: '',
  state: '',
  startTime: '',
  cpu_usage: '',
  memory_usage: '',
  processStatus: '',
});

const defaultItem: Ref<{
  connection_status: string,
  pid: any;
  state: any;
  startTime: any;
  cpu_usage: any;
  memory_usage: any;
  processStatus: any;
  id: string,
  title: string,
  url: string,
}> = ref({
  connection_status: '',
  id: '',
  title: '',
  url: '',
  pid: '',
  state: '',
  startTime: '',
  cpu_usage: '',
  memory_usage: '',
  processStatus: '',
});

const valid = ref(true);
const dialog = ref(false);
const applications = ref('');
const editedIndex = ref(-1);
const editedItemName = ref('');

const formTitle = computed(() => {
  return editedIndex.value === -1 ? 'New Application' : 'Edit Application';
});

function close() {
  dialog.value = false;

  editedItem.value = Object.assign({}, defaultItem.value);
  editedIndex.value = -1;
  editedItemName.value = '';
}

function showError(msg: string) {
  errorMessage.value = msg;
  errorSnackbar.value = true;
}

function splitTs(ts: string) {
  const parts = ts.split(' ');
  const date = parts[0] || '';
  const time = parts[1] || '';
  // keep only HH:MM
  const hm = time.split(':').slice(0, 2).join(':');
  return {date, time: hm};
}

const timestampsTableItems = computed(() => {
  return timestampsList.value.map(item => {
    const {date, time} = splitTs(item.ts);
    return {...item, date, time};
  });
});

async function save() {
  dialog.value = false;
  const currentApplication = {
    connection_status: editedItem.value.connection_status,
    pid: editedItem.value.pid,
    id: editedItem.value.id,
    title: editedItem.value.title,
    url: editedItem.value.url,
    state: editedItem.value.state,
    startTime: editedItem.value.startTime,
    cpu_usage: editedItem.value.cpu_usage,
    memory_usage: editedItem.value.memory_usage,
    processStatus: editedItem.value.processStatus
  }

  if (editedIndex.value > -1) {
    await applicationsStore.editApplication(currentApplication);
  } else {
    try {
      await applicationsStore.addApplication(currentApplication.title, currentApplication.url);
    } catch (error) {
      if (typeof error == 'string') {
        if (error.includes("ApplicationAlreadyConnected")) {
          showError("This URL Application is already in the list");
        } else if (error.includes("PIDNotFound")) {
          showError("The application at this URL is not running");
        } else {
          showError("Unexpected Error" + error);
        }
      } else {
        showError("Unexpected Error" + error);
      }
    }
  }

  close();
}

async function deleteApp(appID: string) {
  if (confirm('Are you sure you want to delete this project?')) {
    await applicationsStore.deleteApplication(appID);
  }
}

function showExportDialog(newApp: { id: string; pid: number; title?: string }) {
  if (!newApp || !newApp.id) return;
  pendingExport.value = {title: newApp.title, pid: newApp.pid};
  toastComment.value = '';
  toastDialog.value = true;
}


function editApp(app: Application) {
  editedIndex.value = applicationsStore.getApplications.value.indexOf(app);

  const {title, url, id} = app;

  editedItemName.value = title;

  editedItem.value.id = id;
  editedItem.value.title = title;
  editedItem.value.url = url;

  dialog.value = true;
}

listen<Application[]>("update:applications", (event) => {
  if (dataStore.pause == false) {
    console.log("Received applications: " + JSON.stringify(event.payload[0]));
    event.payload.forEach(newApp => {
      console.log(newApp);
      const existingApp = applicationsStore.applications.find(app => app.id === newApp.id);
      const prevStatus = existingApp?.connection_status;
      if (prevStatus === 'Connected' && newApp.connection_status !== 'Connected') {
        console.log("Popup");
        showExportDialog({id: newApp.id, pid: newApp.pid, title: newApp.title});
      }

      lastAppStatuses.set(newApp.id, newApp.connection_status);
      if (existingApp) {
        Object.assign(existingApp, newApp);
      } else {
        applicationsStore.applications.push(newApp);
      }
    });
  }
});

function selectRowById(id: string) {
  selectedRowId.value = id;
  const entry = exportsList.value.find((e: any) => e.app_dir === id || e.app_dir === id);
  if (entry) selectAppDir(entry.app_dir, entry.title);
}

const toastComment = ref('');

// export action invoked when user clicks Save on toast
async function doExport() {
  if (!pendingExport.value) return showError('No pending export selected');

  exporting.value = true;
  try {
    const res = await invoke<string>('export_app_instance', {
      title: pendingExport.value.title,
      name: toastComment.value || ''
    });
    console.log('Exported to', res);
    window.location.reload();
  } catch (e) {
    showError('Export failed: ' + String(e));
  } finally {
    exporting.value = false;
    toastDialog.value = false;
    pendingExport.value = null;
    toastComment.value = '';
  }
}


async function loadFromDisk() {
  try {
    const home = await homeDir();
    const storage = home ? `${home}/.async-tracing` : undefined;
    const res = await invoke<any>('list_exports', {storageFolder: storage});
    exportsList.value = res || [];
    listFromDiskDialog.value = true;
  } catch (e) {
    showError('List exports failed: ' + String(e));
  }
}

function selectTimestamp(item: { ts: string; path?: string; comment_preview?: string }) {
  selectedTimestamp.value = item.ts;
  selectedCommentPreview.value = item.comment_preview || '';
}


async function selectAppDir(appDir: string, appTitle?: string) {
  selectedAppDir.value = appDir;
  selectedAppTitle.value = appTitle || appDir;
  try {
    const home = await homeDir();
    const storage = home ? `${home}/.async-tracing` : undefined;
    const res = await invoke<any>('list_app_timestamps', {storageFolder: storage, appDir});
    timestampsList.value = res || [];
    listFromDiskDialog.value = false;
    timestampsDialog.value = true;
  } catch (e) {
    showError('List timestamps failed: ' + String(e));
  }
}

function importSelectedTimestamp() {
  if (!selectedTimestamp.value) return;
  return importSelectedTimestampImpl();
}

async function importSelectedTimestampImpl() {
  if (!selectedAppDir.value || !selectedTimestamp.value) return showError('Select an export timestamp first');
  try {
    const home = await homeDir();
    const storage = home ? `${home}/.async-tracing` : undefined;
    await invoke('import_from_export_folder', {
      storageFolder: storage,
      appDir: selectedAppDir.value,
      ts: selectedTimestamp.value
    });
    timestampsDialog.value = false;
    window.location.reload();
  } catch (e) {
    showError('Import failed: ' + String(e));
  }
}

function onLoadTimestamps() {
  if (!selectedRowId.value) return showError('Select an export first');
  const entry = exportsList.value.find(e => e.app_dir === selectedRowId.value);
  if (!entry) return showError('Selected export not found');
  listFromDiskDialog.value = false;
  selectAppDir(entry.app_dir, entry.title);
}


listen<{ id: string; pid: number }>('update:pid', (event) => {
  const {id, pid} = event.payload;
  const app = applicationsStore.applications.find(a => a.id === id);
  if (app) {
    app.pid = pid;
  }
});

</script>

<template>
  <VCard elevation="2">
    <template v-slot:text>
      <h1 class="mb-4 font-weight-bold">Applications traced</h1>

      <div class="d-flex align-center justify-space-between">
        <div class="search-container">
          <VTextField v-model="applications" label="Search" prepend-inner-icon="mdi-magnify"
                      variant="outlined" hide-details single-line></VTextField>
        </div>

        <div class="d-flex">
          <VBtn color="secondary" class="mr-2" @click="loadFromDisk">
            Load from disk
          </VBtn>

          <VBtn color="primary" @click="dialog = true">
            <PlusIcon stroke-width="1.5" size="25" class="mr-1"/>
            Add new application
          </VBtn>
        </div>
      </div>
    </template>
    <VDialog v-model="toastDialog" max-width="620">
      <VCard>
        <VCardTitle>Save session for stopped application</VCardTitle>
        <VCardText>
          <div class="d-flex flex-column" style="gap:12px;">
            <div>
              The export will be saved under the application title folder. Optionally add a short comment (visible later
              when loading).
            </div>
            <VTextField
                v-model="toastComment"
                label="Comment (optional)"
                placeholder="What was happening when the trace was taken..."
                hide-details
                density="compact"
                style="width:100%;"
                maxlength="1000"
            />
          </div>
        </VCardText>
        <VCardActions>
          <VSpacer/>
          <VBtn variant="text" @click="toastDialog = false">Dismiss</VBtn>
          <VBtn color="primary" :loading="exporting" @click="doExport">Save</VBtn>
        </VCardActions>
      </VCard>
    </VDialog>


    <VSnackbar
        v-model:model-value="errorSnackbar"
        :timeout="6000"
        color="error"
        top
        right
    >
      {{ errorMessage }}

      <VBtn
          color="white"
          variant="text"
          @click="errorSnackbar = false"
      >
        Close
      </VBtn>
    </VSnackbar>

    <VDialog v-model="dialog" max-width="500" persistent>
      <VCard v-click-outside="close">
        <VCardTitle class="pa-4 bg-primary">
          <span class="title text-white">{{ formTitle }}</span>
        </VCardTitle>

        <VCard-text>
          <VForm ref="form" v-model="valid" lazy-validation @submit.prevent>
            <VRow align="center">
              <VCol cols="12">
                <VTextField variant="outlined" hide-details v-model="editedItem.title"
                            label="Application Name"></VTextField>
              </VCol>
            </VRow>

            <VRow align="center">
              <VCol cols="12">
                <VTextField variant="outlined" hide-details v-model="editedItem.url"
                            label="Application URL"></VTextField>
              </VCol>
            </VRow>
          </VForm>
        </VCard-text>

        <VCard-actions class="pa-4">
          <VSpacer></VSpacer>
          <VBtn color="error" variant="flat" @click="close">Cancel</VBtn>
          <VBtn color="primary" :disabled="editedItem.title === '' || editedItem.url === ''" variant="flat"
                @click="save">Save
          </VBtn>
        </VCard-actions>
      </VCard>
    </VDialog>
    <VDialog v-model="listFromDiskDialog" max-width="600">
      <VCard>
        <VCardTitle>Available applications traces on your computer</VCardTitle>
        <VCardText>
          <div v-if="exportsList.length === 0">No traces found.</div>
          <VDataTable
              v-else
              :headers="diskHeaders"
              :items="exportsList"
              item-key="app_dir"
              :items-per-page="8"
          >
            <template style="width: 20px!important" v-slot:item.select="{ item }">
              <div class="cell-flex">
                  <VRadio
                      v-model="selectedRowId"
                      :value="item.app_dir"
                      density="compact"
                      hide-details
                      @change="() => selectRowById(item.app_dir)"
                      aria-label="Select export"
                      style="width: 20px!important"
                  />
              </div>
            </template>
            <template v-slot:item.app_dir="{ item }">
              <div class="cell-flex">
                {{ item.app_dir }}
                ""
                ""
                ""
              </div>
            </template>
          </VDataTable>
        </VCardText>
        <VCardActions>
          <VSpacer/>
          <VBtn text @click="listFromDiskDialog = false">Cancel</VBtn>
          <VBtn color="primary" :disabled="!selectedRowId" @click="onLoadTimestamps">Load timestamps</VBtn>
        </VCardActions>
      </VCard>
    </VDialog>
    <VDialog v-model="timestampsDialog" max-width="800">
      <VCard>
        <VCardTitle>Choose tracing instance for {{ selectedAppTitle || 'app' }}</VCardTitle>
        <VCardText>
          <div v-if="timestampsList.length === 0">No timestamps found.</div>

          <VDataTable
              v-else
              :headers="[
                          { title: '', key: 'select', align: 'center', sortable: false },
                          { title: 'Date', key: 'date', align: 'center' },
                          { title: 'Time', key: 'time', align: 'center' },
                          { title: 'Comment', key: 'comment_preview', align: 'center' }
                         ]"
              :items="timestampsTableItems"
              item-key="ts"
              :items-per-page="8"
          >
            <template v-slot:item.select="{ item }">
              <div class="cell-flex cell-center">
                <div class="col-2">
                  <VRadio v-model="selectedTimestamp" :value="item.ts" density="compact" hide-details @change="() => selectTimestamp(item)"
                          aria-label="Select timestamp"/>
                </div>
              </div>
            </template>
          </VDataTable>

          <div v-if="selectedTimestamp" class="mt-3">
            <strong>Selected:</strong> {{ selectedTimestamp }}<br/>
            <strong>Comment:</strong>
            <div style="white-space:pre-wrap">{{ selectedCommentPreview }}</div>
          </div>
        </VCardText>

        <VCardActions>
          <VSpacer/>
          <VBtn text @click="timestampsDialog = false">Cancel</VBtn>
          <VBtn color="primary" :disabled="!selectedTimestamp" @click="importSelectedTimestamp">Load</VBtn>
        </VCardActions>
      </VCard>
    </VDialog>

    <VDataTable :search="applications" :headers="applicationHeaders"
                :items="applicationsStore.getApplications.value" :row-props="getRowProps">
      <template v-slot:item.connection_status="{ item }">
        <VProgressCircular
            v-if="item.connection_status == 'Connecting'"
            indeterminate
            color="primary"
            size="24"
        />
        <VIcon v-else-if="item.connection_status == 'Connected'" color="green">mdi-check-circle</VIcon>
        <VIcon v-else color="red">mdi-alert-circle</VIcon>
      </template>
      <template v-slot:item.state="{ item }">
        <div class="justify-center">
          <VChip :color="getStateChipColor(item.state)" class="text-uppercase" label size="small">
            <div v-if="item.state === 'Enabled'">Enabled</div>
            <div v-else>Disabled</div>
          </VChip>
        </div>
      </template>
      <template v-slot:item.memory_usage="{ item }">
        <div class="justify-center">
          {{ item.memory_usage }} MB
        </div>
      </template>
      <template v-slot:item.actions="{ item }">
        <VBtn icon flat @click="applicationsStore.toggleAppState(item)"
              :class="item.state === 'Disabled' ? 'disabled-action-btn' : ''">
          <PlayerPlayFilledIcon v-if="item.state === 'Disabled'" stroke-width="1.5" size="20"
                                class="text-primary"/>
          <PlayerPauseFilledIcon v-else stroke-width="1.5" size="20" class="text-primary"/>
        </VBtn>
        <VTooltip text="Edit">
          <template v-slot:activator="{ props }">
            <VBtn icon flat @click="editApp(item)" v-bind="props"
                  :class="item.state === 'Disabled' ? 'disabled-action-btn' : ''">
              <PencilIcon stroke-width="1.5" size="20" class="text-primary"/>
            </VBtn>
          </template>
        </VTooltip>
        <VTooltip text="Delete">
          <template v-slot:activator="{ props }">
            <VBtn icon flat @click="deleteApp(item.id)" v-bind="props"
                  :class="item.state === 'Disabled' ? 'disabled-action-btn' : ''">
              <TrashIcon stroke-width="1.5" size="20" class="text-error"/>
            </VBtn>
          </template>
        </VTooltip>
      </template>
    </VDataTable>
  </VCard>
</template>

<style scoped>
.cell-flex {
  text-align: left;
}

.disabled-action-btn {
  opacity: 0.7;
  background-color: transparent !important;
  box-shadow: none !important;
  border: none !important;
}

.disabled-action-btn::before {
  opacity: 0 !important;
}

.disabled-action-btn:hover {
  background-color: transparent !important;
}

.text-white {
  color: rgb(255, 255, 255) !important;
}

.search-container {
  width: 400px;
}

.v-data-table-header th:nth-child(1),
.v-data-table__wrapper td:nth-child(1) {
  max-width: 20px;
  width: 20px;
  white-space: nowrap;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
}

.v-data-table__wrapper table {
  table-layout: fixed;
}
</style>