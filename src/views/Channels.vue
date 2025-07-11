<script setup lang="ts">
import { ref, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { SpyPayload } from '@/types/channels'

const spyEvents = ref<SpyPayload[]>([])

const filterText = ref('')

const expanded = ref<string[]>([])

listen<SpyPayload>('spy:event', (e) => {
  spyEvents.value.push(e.payload)
})

const filteredEvents = computed(() => {
  const q = filterText.value.toLowerCase()
  return spyEvents.value.filter(e =>
    e.id.toLowerCase().includes(q) ||
    e.event.variant.toLowerCase().includes(q) ||
    JSON.stringify(e.event.data ?? '').toLowerCase().includes(q)
  )
})

const headers = [
  { title: 'App ID', key: 'id' },
  { title: 'Variant', key: 'event.variant' },
  { title: 'Timestamp', key: 'timestamp', sortable: false },
]

</script>

<template>
  <v-card elevation="2" class="mx-auto my-4">
    <v-card-title>
      <span class="text-h6">Channel Overview</span>
    </v-card-title>

    <v-card-text>
      <div class="d-flex justify-end mb-4">
        <v-text-field v-model="filterText" placeholder="Search events..." prepend-inner-icon="mdi-magnify"
          variant="outlined" hide-details dense clearable style="max-width: 300px;" />
      </div>

      <v-data-table :headers="headers" :items="filteredEvents" item-value="id" show-expand :expanded.sync="expanded"
        class="elevation-1">
        <template #item.event.variant="{ item }">
          {{ item.event.variant }}
        </template>

        <template #item.timestamp="{ item }">
          {{ new Date().toLocaleTimeString() }}
        </template>

        <template #expanded-row="{ item }">
          <v-card flat class="ma-2 pa-4 grey--text text--darken-1">
            <pre style="white-space: pre-wrap;">
{{ JSON.stringify(item.event, null, 2) }}
            </pre>
          </v-card>
        </template>
      </v-data-table>
    </v-card-text>
  </v-card>
</template>

<style scoped>
pre {
  font-family: Roboto, monospace;
  font-size: 0.875rem;
  margin: 0;
}
</style>
