
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
    <VDialog v-model="listFromDiskDialog" max-width="900">
      <VCard>
        <VCardTitle>Available applications tracings on your computer</VCardTitle>
        <VCardText>
          <div v-if="exportsList.length === 0">No exports found.</div>
          <VDataTable
              v-else
              :headers="diskHeaders"
              :items="exportsList"
              item-key="app_dir"
              :items-per-page="8"
          >
            <template v-slot:[`item.select`]="{ item }">
              <div class="cell-flex">
                <div class="col-2">
                  <VRadio
                      v-model="selectedRowId"
                      :value="item.app_dir"
                      density="compact"
                      hide-details
                      @change="() => selectRowById(item.app_dir)"
                      aria-label="Select export"
                  />
                </div>
              </div>
            </template>

            <template v-slot:[`item.title`]="{ item }">
              <div class="cell-flex">{{ item.title }}</div>
            </template>

            <template v-slot:[`item.app_dir`]="{ item }">
              <div class="cell-flex">
                {{ item.app_dir }}
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
            <template v-slot:[`item.select`]="{ item }">
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
                :items="appItems" :row-props="getRowProps">
      <template v-slot:[`item.connection_status`]="{ item }">
        <VProgressCircular
            v-if="item.connection_status == 'Connecting'"
            indeterminate
            color="primary"
            size="24"
        />
        <VIcon v-else-if="item.connection_status == 'Connected'" color="green">mdi-check-circle</VIcon>
        <VIcon v-else color="red">mdi-alert-circle</VIcon>
      </template>
      <template v-slot:[`item.state`]="{ item }">
        <div class="justify-center">
          <VChip :color="getStateChipColor(item.state)" class="text-uppercase" label size="small">
            <div v-if="item.state === 'Enabled'">Enabled</div>
            <div v-else>Disabled</div>
          </VChip>
        </div>
      </template>
      <template v-slot:[`item.memory_usage`]="{ item }">
        <div class="justify-center">
          {{ item.memory_usage }} MB
        </div>
      </template>
      <template v-slot:[`item.actions`]="{ item }">
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

<script lang="ts">
import { ComputedRef, defineComponent, Ref, type CSSProperties } from "vue";
import type { DataTableHeader } from "vuetify";
import { useApplicationStore } from "@/stores/application";
import { useDataStore } from "@/stores/data";
import type { Application } from "@/types/applications";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { homeDir } from "@tauri-apps/api/path";
import {
  PlusIcon,
  PlayerPlayFilledIcon,
  PlayerPauseFilledIcon,
  PencilIcon,
  TrashIcon,
} from "vue-tabler-icons";

type ExportEntry = { app_dir: string; title: string };
type TimestampEntry = { ts: string; path?: string; comment_preview?: string };
type PendingExport = { title: string | undefined; pid: number } | null;

type EditedApplication = {
  connection_status: Application["connection_status"] | "";
  pid: Application["pid"];
  state: Application["state"] | "";
  startTime: string; 
  cpu_usage: Application["cpu_usage"];
  memory_usage: Application["memory_usage"];
  processStatus: Application["processStatus"] | "";
  id: Application["id"];
  title: Application["title"];
  url: Application["url"];
};

export default defineComponent({
  name: "ApplicationsOverview",

  components: {
    PlusIcon,
    PlayerPlayFilledIcon,
    PlayerPauseFilledIcon,
    PencilIcon,
    TrashIcon,
  },

  data() {
    const applicationsStore = useApplicationStore();
    const dataStore = useDataStore();

    const emptyEdited: EditedApplication = {
      connection_status: "",
      pid: 0 as Application["pid"],
      state: "",
      startTime: "",
      cpu_usage: 0 as Application["cpu_usage"],
      memory_usage: 0 as Application["memory_usage"],
      processStatus: "",
      id: "",
      title: "",
      url: "",
    };

    return {
      // stores
      applicationsStore,
      dataStore,

      // ui state
      errorMessage: "" as string,
      errorSnackbar: false as boolean,
      exporting: false as boolean,

      // dialogs
      dialog: false as boolean,
      toastDialog: false as boolean,
      listFromDiskDialog: false as boolean,
      timestampsDialog: false as boolean,

      // search
      applications: "" as string,

      // edit
      valid: true as boolean,
      editedIndex: -1,
      editedItemName: "" as string,
      editedItem: { ...emptyEdited } as EditedApplication,
      defaultItem: { ...emptyEdited } as EditedApplication,

      // export + timestamps
      pendingExport: null as PendingExport,
      exportsList: [] as ExportEntry[],
      timestampsList: [] as TimestampEntry[],
      selectedRowId: null as string | null,
      selectedAppDir: "" as string,
      selectedAppTitle: "" as string,
      selectedTimestamp: "" as string,
      selectedCommentPreview: "" as string,
      toastComment: "" as string,

      // headers
      applicationHeaders: [
        { title: "Status", align: "center", key: "connection_status" },
        { title: "Name", align: "center", key: "title" },
        { title: "PID", align: "center", key: "pid" },
        { title: "URL", align: "center", key: "url" },
        { title: "State", align: "center", key: "state" },
        { title: "Start Time", align: "center", key: "start_time" },
        { title: "CPU", align: "center", key: "cpu_usage" },
        { title: "Memory", align: "center", key: "memory_usage" },
        { title: "Actions", align: "center", key: "actions", sortable: false },
      ] as DataTableHeader[],

      diskHeaders: [
        { title: "", align: "center", key: "select", sortable: false },
        { title: "Title", align: "center", key: "title" },
      ] as DataTableHeader[],

      // helpers
      lastAppStatuses: new Map<string, string>(),
    };
  },

  computed: {
    formTitle(): string {
      return this.editedIndex === -1 ? "New Application" : "Edit Application";
    },

    timestampsTableItems(): Array<TimestampEntry & { date: string; time: string }> {
      return this.timestampsList.map((item) => {
        const { date, time } = this.splitTs(item.ts);
        return { ...item, date, time };
      });
    },

    appItems(): Application[] {
      const ga = this.applicationsStore.getApplications as Application[] | Ref<Application[]> | ComputedRef<Application[]>;
      return Array.isArray(ga) ? ga : ga.value ?? [];
    },
  },

  methods: {
    getStateChipColor(state: string): string {
      const colorMap: Record<string, string> = { Enabled: "green", Disabled: "red" };
      return colorMap[state] || "default";
    },

    getRowProps(ctx: { item: Application }) {
      const style: CSSProperties =
        ctx.item.state === "Disabled" ? { backgroundColor: "#F5F5F5" } : {};
      return { style };
    },

    close() {
      this.dialog = false;
      this.editedItem = { ...this.defaultItem };
      this.editedIndex = -1;
      this.editedItemName = "";
    },

    showError(msg: string) {
      this.errorMessage = msg;
      this.errorSnackbar = true;
    },

    splitTs(ts: string) {
      const parts = ts.split(" ");
      const date = parts[0] || "";
      const time = parts[1] || "";
      const hm = time.split(":").slice(0, 2).join(":"); 
      return { date, time: hm };
    },

    async save() {
      this.dialog = false;

      const currentApplication = {
        connection_status: this.editedItem.connection_status,
        pid: this.editedItem.pid,
        id: this.editedItem.id,
        title: this.editedItem.title,
        url: this.editedItem.url,
        state: this.editedItem.state,
        startTime: this.editedItem.startTime,
        cpu_usage: this.editedItem.cpu_usage,
        memory_usage: this.editedItem.memory_usage,
        processStatus: this.editedItem.processStatus,
      };

      if (this.editedIndex > -1) {
        await this.applicationsStore.editApplication(currentApplication);
      } else {
        try {
          await this.applicationsStore.addApplication(
            currentApplication.title,
            currentApplication.url
          );
        }  catch (error) {
        if (typeof error == "string") {
          if (error.includes("ApplicationAlreadyConnected")) {
            this.showError("This URL Application is already in the list");
          } else if (error.includes("PIDNotFound")) {
            this.showError("The application at this URL is not running");
          } else {
            this.showError("Unexpected Error" + error);     
          }
        } else {
          this.showError("Unexpected Error" + error);       
        }
      }
    }
      this.close();
    },

    async deleteApp(appID: string) {
      if (confirm("Are you sure you want to delete this project?")) {
        await this.applicationsStore.deleteApplication(appID);
      }
    },

    editApp(app: Application) {
      this.editedIndex = this.appItems.indexOf(app);
      const { title, url, id } = app;
      this.editedItemName = title;
      this.editedItem.id = id;
      this.editedItem.title = title;
      this.editedItem.url = url;
      this.dialog = true;
    },

    selectRowById(id: string) {
      this.selectedRowId = id;
      const entry = this.exportsList.find((e) => e.app_dir === id || e.app_dir === id);
      if (entry) this.selectAppDir(entry.app_dir, entry.title);
    },

    async doExport() {
      if (!this.pendingExport) return this.showError("No pending export selected");

      this.exporting = true;
      try {
        const res = await invoke<string>("export_app_instance", {
          title: this.pendingExport.title,
          name: this.toastComment || "",
        });
        console.log("Exported to", res);
        window.location.reload();
      } catch (e) {
        this.showError("Export failed: " + String(e));
      } finally {
        this.exporting = false;
        this.toastDialog = false;
        this.pendingExport = null;
        this.toastComment = "";
      }
    },

    showExportDialog(newApp: { id: string; pid: number; title?: string }) {
      if (!newApp || !newApp.id) return;
      this.pendingExport = { title: newApp.title, pid: newApp.pid }; 
      this.toastComment = "";
      this.toastDialog = true;
    },

    async loadFromDisk() {
      try {
        const home = await homeDir();
        const storage = home ? `${home}/.async-tracing` : undefined;
        const res = await invoke<ExportEntry[]>("list_exports", { storageFolder: storage });
        this.exportsList = res || [];
        this.listFromDiskDialog = true;
      } catch (e) {
        this.showError("List exports failed: " + String(e));
      }
    },

    selectTimestamp(item: { ts: string; path?: string; comment_preview?: string }) {
      this.selectedTimestamp = item.ts;
      this.selectedCommentPreview = item.comment_preview || "";
    },

    async selectAppDir(appDir: string, appTitle?: string) {
      this.selectedAppDir = appDir;
      this.selectedAppTitle = appTitle || appDir;
      try {
        const home = await homeDir();
        const storage = home ? `${home}/.async-tracing` : undefined;
        const res = await invoke<TimestampEntry[]>("list_app_timestamps", {
          storageFolder: storage,
          appDir,
        });
        this.timestampsList = res || [];
        this.listFromDiskDialog = false;
        this.timestampsDialog = true;
      } catch (e) {
        this.showError("List timestamps failed: " + String(e));
      }
    },

    importSelectedTimestamp() {
      if (!this.selectedTimestamp) return;
      return this.importSelectedTimestampImpl();
    },

    async importSelectedTimestampImpl() {
      if (!this.selectedAppDir || !this.selectedTimestamp) {
        return this.showError("Select an export timestamp first");
      }
      try {
        const home = await homeDir();
        const storage = home ? `${home}/.async-tracing` : undefined;
        await invoke("import_from_export_folder", {
          storageFolder: storage,
          appDir: this.selectedAppDir,
          ts: this.selectedTimestamp,
        });
        this.timestampsDialog = false;
        window.location.reload();
      } catch (e) {
        this.showError("Import failed: " + String(e));
      }
    },

    onLoadTimestamps() {
      if (!this.selectedRowId) return this.showError("Select an export first");
      const entry = this.exportsList.find((e) => e.app_dir === this.selectedRowId);
      if (!entry) return this.showError("Selected export not found");
      this.listFromDiskDialog = false;
      this.selectAppDir(entry.app_dir, entry.title);
    },
  },

  created() {
    listen<Application[]>("update:applications", (event) => {
      if (this.dataStore.pause == false) {                
        console.log("Received applications: " + JSON.stringify(event.payload[0])); 
        event.payload.forEach((newApp) => {
          console.log(newApp);                             

          const existingApp = this.applicationsStore.applications.find(
            (app) => app.id === newApp.id
          );
          const prevStatus = existingApp?.connection_status;

          if (prevStatus === "Connected" && newApp.connection_status !== "Connected") {
            console.log("Popup");                          
            this.showExportDialog({ id: newApp.id, pid: newApp.pid, title: newApp.title });
          }

          this.lastAppStatuses.set(newApp.id, newApp.connection_status);

          if (existingApp) Object.assign(existingApp, newApp);
          else this.applicationsStore.applications.push(newApp);
        });
      }
    });

    listen<{ id: string; pid: number }>("update:pid", (event) => {
      const { id, pid } = event.payload;
      const app = this.applicationsStore.applications.find((a) => a.id === id);
      if (app) app.pid = pid;
    });
  },
});
</script>


<style scoped>
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
</style>