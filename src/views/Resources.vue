<script setup lang="ts">
import { ref, computed } from 'vue'
import { DataTableHeader } from 'vuetify';
import { listen } from '@tauri-apps/api/event'
import { useDataStore } from '@/stores/data';

const resourcesSearch = ref('');
const selectedApp  = ref('All');
const dataStore = useDataStore();

const resourcesHeaders = ref<DataTableHeader[]>([
    { title: "Resource Type", key: "resource_type", align:"center" },
    { title: "ID", key: "id", align:"center" },
    { title: "Status", key: "status", align:"center" },
    { title: "Target", key: "target", align:"center" },
    { title: "Duration", key: "duration", align:"center" },
    { title: "Location", key: "location", align:"center" },
    { title: "Attributes", key: "attributes", align:"center" },
]);

function getResourceChipColor(status: string){
    switch(status){
        case 'Ready': return 'green'
        default:        return 'gray'
    }
}

const appList = computed(() => {
  const names  = dataStore.resources.map(r => r.app_name)
  const unique = Array.from(new Set(names)).sort()
  return ['All', ...unique]
})

const filteredResources = computed(() => {
  return dataStore.resources
    .filter(r => selectedApp.value === 'All' || r.app_name === selectedApp.value)
    .filter(r => {
      const q = resourcesSearch.value.toLowerCase()
      return (
        r.resource_type.toLowerCase().includes(q) ||
        r.id.toString().includes(q)     ||
        r.status.toString().includes(q)          ||
        r.location.toLowerCase().includes(q)
      )
    })
});

listen<any[]>('update:resources', e => {
  dataStore.handleResourceUpdate(e);
})

</script>

<template>
  <v-card elevation="2">
    <v-card-text>
      <h1 class="mb-4 font-weight-bold">Resources</h1>
      
      <div class="d-flex align-center justify-space-between mb-4">
        <v-text-field
          v-model="resourcesSearch"
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
        :headers="resourcesHeaders"
        :items="filteredResources"
        :item-value="item => `${item.app_name}-${item.id}`"
      >
        <template #item.status="{ item }">
          <v-chip
            :color="getResourceChipColor(item.status)"
            size="small"
            class="text-uppercase"
          >
            {{ item.status }}
          </v-chip>
        </template>

        <template #item.location="{ item }">
            <span v-html="item.location"></span>
        </template>
      </v-data-table>
    </v-card-text>
  </v-card>
</template>