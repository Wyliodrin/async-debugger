<script lang="ts">
import { defineComponent, onMounted, onBeforeUnmount, ref } from 'vue';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface Payload {
  name: string;
  kind: string;
  ts: number;
  payload: string;
}

export default defineComponent({
  setup() {
    const rows = ref<Payload[]>([]);
    let unlisten: UnlistenFn | null = null;

    onMounted(async () => {

      unlisten = await listen<Payload>(
        'debug-event',
        event => {
          rows.value.push(event.payload);
        }
      );
    });

    onBeforeUnmount(() => {
      if (unlisten) unlisten();
    });

    return { rows };
  }
});
</script>

<template>
  <v-data-table :headers="[
    { title: 'Name', key: 'name' },
    { title: 'Kind', key: 'kind' },
    { title: 'Timestamp', key: 'ts' },
    { title: 'Payload', key: 'payload' }
  ]" :items="rows" :items-per-page="5">
    <template #item.ts="{ item }">
      {{ new Date(item.ts).toLocaleTimeString() }}
    </template>
  </v-data-table>
</template>
