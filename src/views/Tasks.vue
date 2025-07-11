<script setup lang="ts">
import { ref, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { DataTableHeader } from 'vuetify'
import { useDataStore } from '@/stores/data'

const tasksSearch  = ref('')
const selectedApp  = ref('All')
const dataStore = useDataStore();

const appList = computed(() => {
  const names  = dataStore.tasks.map(t => t.app_name)
  const unique = Array.from(new Set(names)).sort()
  return ['All', ...unique]
})

const filteredTasks = computed(() => {
  return dataStore.tasks
    .filter(t => selectedApp.value === 'All' || t.app_name === selectedApp.value)
    .filter(t => {
      const q = tasksSearch.value.toLowerCase()
      return (
        t.app_name.toLowerCase().includes(q) ||
        t.name.toLowerCase().includes(q)     ||
        t.id.toString().includes(q)          ||
        t.tid.toString().includes(q)
      )
    })
})

const taskHeaders = ref<DataTableHeader[]>([
  { title: 'App Name',      key: 'app_name',       align: 'center'},
  { title: 'ID',            key: 'id',             align: 'center'},
  { title: 'TID',           key: 'tid',            align: 'center'},
  { title: 'Name',          key: 'name',           align: 'center'},
  { title: 'Type',          key: 'kind',           align: 'center'},
  { title: 'State',         key: 'state',          align: 'center'},
  { title: 'Spawned Time',  key: 'created_at',     align: 'center'},
  { title: 'Runtime',       key: 'runtime',        align: 'center'},
  { title: 'Scheduled',     key: 'scheduled',      align: 'center'},
  { title: 'Idle',          key: 'idle',           align: 'center'},
  { title: 'Busy',          key: 'busy',           align: 'center'},
  { title: 'Location',      key: 'location',       align: 'center'},
])

function getTaskChipColor(stateOrKind: string) {
  switch (stateOrKind) {
    case 'Running':   return 'green'
    case 'Stopped':   return 'red'
    case 'SPAWN':     return 'blue'
    case 'BLOCKING':  return 'orange'
    default:          return 'grey'
  }
}



listen<any[]>('update:tasks', e => {
  dataStore.handleTaskUpdate(e);
})
</script>

<template>
  <v-card elevation="2">
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
      >
        <template #item.kind="{ item }">
          <v-chip
            :color="getTaskChipColor(item.kind)"
            size="small"
            class="text-uppercase"
          >
            {{ item.kind }}
          </v-chip>
        </template>

        <template #item.state="{ item }">
          <v-chip
            :color="getTaskChipColor(item.state)"
            size="small"
          >
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
