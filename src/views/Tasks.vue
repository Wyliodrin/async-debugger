<script setup lang="ts">
import { ref, computed, Ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { DataTableHeader } from 'vuetify'
import { useDataStore } from '@/stores/data'
import { Task } from '@/types/tasks'

const tasksSearch  = ref('')
const selectedApp  = ref('All')
const dataStore = useDataStore();
const dialog = ref(false);

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

const editedItem: Ref<{
    app_name: string,
    id: any;
    name: any;
    color: string,
}> = ref({
    app_name: '',
    id: '',
    name: '',
    color: '',
});

const defaultItem: Ref<{
    app_name: string,
    id: any;
    name: any;
    color: string,
}> = ref({
    app_name: '',
    id: '',
    name: '',
    color: '',
});

function close() {
    dialog.value = false;

    editedItem.value = Object.assign({}, defaultItem.value);
}

async function save() {
    dialog.value = false;
    dataStore.editTask(editedItem.value.id, editedItem.value.name, editedItem.value.color, editedItem.value.app_name);
    close();
}

function editTask(task: Task){
    const { id, name, app_name } = task;

    editedItem.value.id = id;
    editedItem.value.app_name = app_name;
    editedItem.value.name = name;

    dialog.value = true;

}



listen<any[]>('update:tasks', e => {
  dataStore.handleTaskUpdate(e);
})
</script>

<template>
  <v-card elevation="2">
      <v-dialog v-model="dialog" max-width="500" persistent>
          <v-card v-click-outside="close">
              <v-card-title class="pa-4 bg-primary">
                  <span class="title text-white">Change name of the task</span>
              </v-card-title>

              <v-card-text>
                <v-form ref="form" lazy-validation @submit.prevent>
                  <v-row align="center">
                    <v-col cols="12">
                      <v-text-field
                        variant="outlined"
                        hide-details
                        v-model="editedItem.name"
                        label="Task Name"
                      ></v-text-field>
                    </v-col>

                    <v-col cols="12">
                      <v-color-picker
                        v-model="editedItem.color"
                        flat
                        hide-canvas
                        hide-inputs
                        show-swatches
                        swatches-max-height="150"
                      ></v-color-picker>
                    </v-col>
                  </v-row>
                </v-form>
              </v-card-text>

              <v-card-actions class="pa-4">
                  <v-spacer></v-spacer>
                  <v-btn color="error" variant="flat" @click="close">Cancel</v-btn>
                  <v-btn color="primary" :disabled="editedItem.name === ''" variant="flat"
                      @click="save">Save</v-btn>
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

        <template #item.created_at="{ item }">
          <span v-html="item.created_at"></span>
        </template>

        <template #item.location="{ item }">
          <span v-html="item.location"></span>
        </template>

        <template #item.name="{ item }">
          <v-chip
              :color="item.color ? item.color : 'gray'"
              size="small">
                {{ item.name? item.name : "No name" }}
         </v-chip>
          <v-tooltip text="Edit">
              <template v-slot:activator="{ props }">
                  <v-btn icon flat @click="editTask(item)" v-bind="props">
                      <PencilIcon stroke-width="1.5" size="20" class="text-primary" />
                  </v-btn>
              </template>
          </v-tooltip>
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
