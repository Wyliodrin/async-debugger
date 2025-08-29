<script setup lang="ts">
import {useDataStore} from '@/stores/data';
import {listen} from '@tauri-apps/api/event';
import {computed, ref} from 'vue';
import {DataTableHeader} from 'vuetify';

const pollsSearch = ref('');
const selectedApp = ref('All');
const dataStore = useDataStore();

const pollsHeaders = ref<DataTableHeader[]>([
  {title: "Received at", key: "received_at", align: "center"},
  {title: "Poll Type", key: "poll_type", align: "center"},
  {title: "Resource Name", key: "resource_name", align: "center"},
  {title: "Task", key: "task_name", align: "center"},
  {title: "Status", key: "is_ready", align: "center"},
  {title: "Location", key: "location", align: "center"},
]);

const appList = computed(() => {
  const names = dataStore.polls.map(p => p.app_name)
  const unique = Array.from(new Set(names)).sort()
  return ['All', ...unique]
})

const filteredPolls = computed(() => {
  return dataStore.polls
      .filter(p => selectedApp.value === 'All' || p.app_name === selectedApp.value)
      .filter(p => {
        const q = pollsSearch.value.toLowerCase()
        return (
            p.poll_type.toLowerCase().includes(q) ||
            p.resource_id.toString().includes(q) ||
            p.resource_name.toString().includes(q) ||
            p.task_id.toString().includes(q) ||
            p.task_name.toString().includes(q) ||
            p.location.toLowerCase().includes(q)
        )
      })
});

listen<any[]>('update:polls', e => {
  dataStore.handlePollUpdate(e);
})
</script>

<template>
  <v-card elevation="2">
    <v-card-text>
      <h1 class="mb-4 font-weight-bold">Polling Overview</h1>

      <div class="d-flex align-center justify-space-between mb-4">
        <v-text-field
            v-model="pollsSearch"
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
          :headers="pollsHeaders"
          :items="filteredPolls"
      >
        <template #item.is_ready="{ item }">
          <v-chip
              :color="item.is_ready ? 'green' : 'red'"
              size="small"
              class="text-uppercase"
          >
            {{ item.is_ready ? 'Ready' : 'Not Ready' }}
          </v-chip>
        </template>

        <template #item.received_at="{ item }">
          <span v-html="item.received_at"></span>
        </template>

        <template #item.resource_name="{ item }">
          <v-tooltip location="bottom">
            <template #activator="{ props }">
              <span v-bind="props">
                {{ item.resource_name }}
              </span>
            </template>
            <span>ID:{{ item.resource_id }}</span>
          </v-tooltip>
        </template>

        <template #item.location="{ item }">
          <v-chip
              v-if="item.location === 'Unknown'"
              color="grey"
              size="small"
          >
            Unknown
          </v-chip>

          <span
              v-else
              v-html="item.location"
          ></span>
        </template>

        <template #item.task_name="{ item }">
          <v-tooltip location="bottom">
            <template #activator="{ props }">
              <v-chip
                  :color="item.task_color? item.task_color : 'gray'"
                  size="small">
                <span v-bind="props">
                  {{ item.task_name ? item.task_name : item.task_id }}
                </span>
              </v-chip>
            </template>
            <span>ID:{{ item.task_id }}</span>
          </v-tooltip>
        </template>
      </v-data-table>
    </v-card-text>
  </v-card>
</template>