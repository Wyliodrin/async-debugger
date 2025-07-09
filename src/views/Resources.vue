<script setup lang="ts">
import { ref, computed } from 'vue'
import type { Resource } from '@/types/resources';
import { DataTableHeader } from 'vuetify';
import { listen } from '@tauri-apps/api/event'

const resources = ref<Resource[]>([]);
const resourcesSearch = ref('');

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
        case 'Pending': return 'brown'
        default:        return 'gray'
    }
}

const filteredResources = computed(() => {
  return resources.value
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
    resources.value = e.payload.map(r => {

      return {
        ...r,
        duration: r.duration.formatted
      }
    })
    console.log("Resources primite:" + JSON.stringify(e.payload[0]));
})

</script>

<template>
  <v-card elevation="2">
    <v-card-text>
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

      <v-data-table
        :headers="resourcesHeaders"
        :items="filteredResources"
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
      </v-data-table>
    </v-card-text>
  </v-card>
</template>