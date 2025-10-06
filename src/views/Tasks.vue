<template>
  <v-card elevation="2">
    <v-dialog v-model="dialog" max-width="500" persistent>
      <v-card v-click-outside="close">
        <v-card-title class="pa-4 bg-primary">
          <span class="title text-white">Edit task</span>
        </v-card-title>

        <v-card-text>
          <v-form ref="form" lazy-validation @submit.prevent>
            <v-row class="mb-4 mt-4">
              <v-text-field
                  variant="outlined"
                  hide-details
                  v-model="editedItem.name"
                  label="Task Name"
              ></v-text-field>
            </v-row>

            <v-row class="mb-4">
              <v-color-picker
                  v-model="editedItem.color"
                  flat
                  hide-canvas
                  hide-inputs
                  show-swatches
                  swatches-max-height="150"
              ></v-color-picker>
            </v-row>

            <v-row class="mb-4">
              <v-expansion-panels>
                <v-expansion-panel>
                  <v-expansion-panel-title>
                    > Warnings
                  </v-expansion-panel-title>

                  <v-expansion-panel-text>
                    <v-list>
                      <v-list-item
                          v-for="(warning, key) in editedItem.warnings"
                          :key="key"
                      >
                        <v-row class="align-center justify-space-between" no-gutters>
                          <v-col cols="4">
                            <strong>{{ key }}</strong>
                          </v-col>

                          <v-col cols="8" class="d-flex justify-end align-center">
                            <v-text-field
                                v-if="warning.parameter !== undefined"
                                v-model.number="warning.parameter"
                                label="Parameter"
                                type="number"
                                hide-details
                                density="compact"
                                class="mr-4"
                                style="max-width: 120px"
                            />

                            <v-switch
                                v-model="warning.enabled"
                                color="green"
                                inset
                                hide-details
                            />
                          </v-col>
                        </v-row>
                      </v-list-item>
                    </v-list>
                  </v-expansion-panel-text>
                </v-expansion-panel>
              </v-expansion-panels>
            </v-row>
          </v-form>
        </v-card-text>

        <v-card-actions class="pa-4">
          <v-spacer></v-spacer>
          <v-btn color="error" variant="flat" @click="close">Cancel</v-btn>
          <v-btn color="primary" :disabled="editedItem.name === ''" variant="flat"
                 @click="save">Save
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
    <v-card-text>
      <h1 class="mb-4 font-weight-bold">Tasks Overview</h1>

      <div class="d-flex align-center justify-space-between mb-4">
        <v-text-field
            v-model="tasksSearch"
            label="Search"
            prepend-inner-icon="mdi-magnify"
            variant="outlined"
            hide-details
            single-line
            class="search-container"
        />
      </div>

      <div class="d-flex flex-wrap mb-4">
        <v-btn
            v-for="app in appList"
            :key="app"
            :color="selectedApp === app ? 'primary' : 'grey lighten-2'"
            variant="tonal"
            size="small"
            class="ma-1"
            @click="selectedApp = app"
        >
          {{ app }}
        </v-btn>
      </div>

      <v-data-table
          :headers="taskHeaders"
          :items="filteredTasks"
          :item-value="item => `${item.app_name}-${item.id}-${item.tid}`"
          :item-class="rowClass"
      >

        <template #[`item.kind`]="{ item }">
          <v-chip
              :color="getTaskChipColor(item.kind)"
              size="small"
              class="text-uppercase"
          >
            {{ item.kind }}
          </v-chip>
        </template>

        <template #[`item.state`]="{ item }">
          <v-chip
              :color="getTaskChipColor(item.state)"
              size="small"
          >
            {{ item.state }}
          </v-chip>
        </template>

        <template #[`item.created_at`]="{ item }">
          <span>{{formatCreatedAt(item.created_at)}}</span>
        </template>

        <template #[`item.runtime`]="{ item }">
          <span>{{ formatDuration(item.runtime) }}</span>
        </template>

        <template #[`item.scheduled`]="{ item }">
          <span>{{ formatDuration(item.scheduled) }}</span>
        </template>

        <template #[`item.idle`]="{ item }">
          <span>{{ formatDuration(item.idle) }}</span>
        </template>

        <template #[`item.busy`]="{ item }">
          <span>{{ formatDuration(item.busy) }}</span>
        </template>

        <template #[`item.location`]="{ item }">
          <span v-html="item.location"></span>
        </template>

        <template #[`item.name`]="{ item }">
          <v-chip
              :color="item.color ? item.color : 'gray'"
              size="small">
            {{ item.name ? item.name : "No name" }}
          </v-chip>
          <v-tooltip text="Edit">
            <template v-slot:activator="{ props }">
              <v-btn icon flat @click="editTask(item)" v-bind="props">
                <PencilIcon stroke-width="1.5" size="20" class="text-primary"/>
              </v-btn>
            </template>
          </v-tooltip>
        </template>
      </v-data-table>
    </v-card-text>
  </v-card>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import type { DataTableHeader } from "vuetify";
import moment from "moment";
import { useDataStore } from "@/stores/data";
import type { Task, TaskWarnings } from "@/types/tasks";

export default defineComponent({
  name: "TasksOverview",
  data() {
    const dataStore = useDataStore();

    return {
      // stores
      dataStore,

      // ui state
      tasksSearch: "" as string,
      selectedApp: "All" as string,
      dialog: false as boolean,

      // table headers
      taskHeaders: [
        { title: "App Name", key: "app_name", align: "center" },
        { title: "ID", key: "id", align: "center" },
        { title: "TID", key: "tid", align: "center" },
        { title: "Name", key: "name", align: "center" },
        { title: "Type", key: "kind", align: "center" },
        { title: "State", key: "state", align: "center" },
        { title: "Spawned Time", key: "created_at", align: "center" },
        { title: "Runtime", key: "runtime", align: "center" },
        { title: "Scheduled", key: "scheduled", align: "center" },
        { title: "Idle", key: "idle", align: "center" },
        { title: "Busy", key: "busy", align: "center" },
        { title: "Location", key: "location", align: "center" },
      ] as DataTableHeader[],

      // edit dialog models
      editedItem: {
        app_name: "",
        id: 0,
        name: "",
        color: "",
        warnings: {
          self_wake_percent: { enabled: true, parameter: 50, description: "" },
          lost_waker: { enabled: true },
          never_yielded: { enabled: true, parameter: 1, description: "" },
          auto_boxed_feature: { enabled: true },
          large_feature: { enabled: true, parameter: 1024, description: "" },
        } as TaskWarnings,
      },

      defaultItem: {
        app_name: "",
        id: 0,
        name: "",
        color: "",
        warnings: {
          self_wake_percent: { enabled: true, parameter: 50, description: "" },
          lost_waker: { enabled: true },
          never_yielded: { enabled: true, parameter: 1, description: "" },
          auto_boxed_feature: { enabled: true },
          large_feature: { enabled: true, parameter: 1024, description: "" },
        } as TaskWarnings,
      },
    };
  },

  computed: {
    appList(): string[] {
      const names = this.dataStore.tasks.map((t) => t.app_name);
      const unique = Array.from(new Set(names)).sort();
      return ["All", ...unique];
    },

    filteredTasks(): Task[] {
      const q = this.tasksSearch.toLowerCase();

      return this.dataStore.tasks
        .filter(
          (t) => this.selectedApp === "All" || t.app_name === this.selectedApp
        )
        .filter((t) => {
          return (
            t.app_name.toLowerCase().includes(q) ||
            t.name.toLowerCase().includes(q) ||
            t.id.toString().includes(q) ||
            t.tid.toString().includes(q)
          );
        });
    },
  },

  methods: {
    getTaskChipColor(stateOrKind: string) {
      switch (stateOrKind) {
        case "Running":
          return "green";
        case "Stopped":
          return "red";
        case "Starved":
          return "purple";
        case "SPAWN":
          return "blue";
        case "BLOCKING":
          return "orange";
        default:
          return "grey";
      }
    },

    close() {
      this.dialog = false;
      this.editedItem = Object.assign({}, this.defaultItem);
    },

    async save() {
      this.dialog = false;
      try {
        await this.dataStore.editTask(
          this.editedItem.id,
          this.editedItem.name,
          this.editedItem.color,
          this.editedItem.app_name,
          this.editedItem.warnings
        );
        this.close();
      } catch (error: unknown) {
        this.dialog = true;
        console.error("Edit task failed:", error);
        alert("Edit task failed:" + (String(error)));
      }
    },

    editTask(task: Task) {
      const { id, name, app_name, warnings } = task;
      this.editedItem.id = id;
      this.editedItem.app_name = app_name;
      this.editedItem.name = name;
      this.editedItem.warnings = warnings;
      this.dialog = true;
    },

    rowClass(item: Task) {
      return item?.state === "Starved" ? "row-starved" : "";
    },

    formatCreatedAt(value: string) {
      if (!value) return "";
      const m = moment(value);
      if (!m.isValid()) return value;
      return m.format("DD.MM.YYYY HH:mm:ss");
    },

    toNumber(x: unknown): number | undefined {
      if (x === null || x === undefined) return undefined;
      if (typeof x === "number") return x;
      if (typeof x === "string" && x.trim() !== "") {
        const n = Number(x);
        return Number.isNaN(n) ? undefined : n;
      }
      return undefined;
    },

    formatDuration(v: unknown): string {
      if (v == null) return "";
      if (typeof v === "string") return v;

      const d = v as {
        secs?: number; seconds?: number; Secs?: number;
        nanos?: number; Nanos?: number; Nano?: number;
      };

      const secs = this.toNumber(d.secs ?? d.seconds ?? d.Secs) ?? 0;
      const nanos = this.toNumber(d.nanos ?? d.Nanos ?? d.Nano) ?? 0;
      const ms = secs * 1000 + Math.floor(nanos / 1e6);
      return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
    }
  },
});
</script>



<style scoped>
.search-container {
  width: 400px;
}

.row-starved {
  background-color: rgba(255, 0, 0, 0.08) !important;
}

.row-starved td {
  background-color: rgba(255, 0, 0, 0.08) !important;
  color: #7a0a0a !important;
}

</style>
