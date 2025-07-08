<script setup lang="ts">
import { ref, computed } from "vue";
import { listen } from "@tauri-apps/api/event";
import { Task } from "@/types/tasks";
import type { DataTableHeader } from "vuetify";

const tasks = ref<Task[]>([]);
const tasksSearch = ref("");
const pause = ref(false);
const selectedApp = ref<string>("All");

const appList = computed<string[]>(() => {
  const names = tasks.value.map((t) => t.app_name);
  const unique = Array.from(new Set(names));
  return ["All", ...unique.sort()];
});

const filteredTasks = computed<Task[]>(() => {
  return tasks.value
    .filter((t) => {
      // filter by selected app
      if (selectedApp.value !== "All" && t.app_name !== selectedApp.value)
        return false;
      return true;
    })
    .filter((t) => {
      const q = tasksSearch.value.toLowerCase();
      return (
        t.app_name.toLowerCase().includes(q) ||
        t.name.toLowerCase().includes(q) ||
        t.id.toString().includes(q) ||
        t.tid.toString().includes(q)
      );
    });
});

const taskHeaders: DataTableHeader<Task>[] = [
  { title: "App Name", value: "app_name", align: "center" },
  { title: "ID",       value: "id",       align: "center" },
  { title: "TID",      value: "tid",      align: "center" },
  { title: "Name",     value: "name",     align: "center" },
  { title: "Type",     value: "kind",     align: "center" },
  { title: "State",    value: "state",    align: "center" },
  { title: "Spawned time", value: "created_at", align: "center" },
  { title: "Runtime",  value: "runtime",  align: "center" },
  { title: "Scheduled",value: "scheduled",align: "center" },
  { title: "Idle",     value: "idle",     align: "center" },
  { title: "Busy",     value: "busy",     align: "center" },
  { title: "Location", value: "location", align: "center" },
];


function getTaskChipColor(stateOrKind: string): string {
  switch (stateOrKind) {
    case "Running":
      return "green";
    case "Stopped":
      return "red";
    case "SPAWN":
      return "blue";
    case "BLOCKING":
      return "orange";
    default:
      return "grey";
  }
}

window.addEventListener("keydown", (event: KeyboardEvent) => {
  if (event.code === "Space") {
    event.preventDefault();
    pause.value = !pause.value;
  }
});

listen<any[]>("update:tasks", (e) => {
  if (!pause.value) {
    tasks.value = e.payload.map((t: any) => {
      const formattedFields = {
        runtime: t.runtime.formatted,
        scheduled: t.scheduled.formatted,
        idle: t.idle.formatted,
        busy: t.busy.formatted,
      };
      let stateKey: string;
      if (typeof t.state === "string") {
        stateKey = t.state;
      } else {
        const keys = Object.keys(t.state ?? {});
        stateKey = keys.length ? keys[0]! : "Unknown";
      }
      return {
        ...t,
        state: stateKey,
        ...formattedFields,
      };
    });
  }
});
</script>

<template>
  <v-card elevation="2">
    <v-card-text>
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
        <v-chip :color="pause ? 'red' : 'green'" dark class="ma-2">
          {{ pause ? "Paused" : "Connected" }}
        </v-chip>
      </div>

      <div class="d-flex flex-wrap mb-4">
        <v-btn
          v-for="app in appList"
          :key="app"
          :color="selectedApp === app ? 'primary' : 'grey lighten-2'"
          small
          class="ma-1"
          @click="selectedApp = app"
        >
          {{ app }}
        </v-btn>
      </div>

      <v-data-table
        :headers="taskHeaders"
        :items="filteredTasks"
      >
        <template #item.kind="{ item }">
          <v-chip :color="getTaskChipColor(item.kind)" small class="text-uppercase">
            {{ item.kind }}
          </v-chip>
        </template>
        <template #item.state="{ item }">
          <v-chip :color="getTaskChipColor(item.state)" small>
            {{ item.state }}
          </v-chip>
        </template>
      </v-data-table>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.search-container {
  width: 400px;
}
</style>
