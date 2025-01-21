<script setup lang="ts">
import { ref } from "vue";
// import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import DataTable from "primevue/datatable";
import Column from "primevue/column";

type Task = {
    id: number;
    tid?: number;
    name?: string;
    kind?: string;
};

const tasks = ref([] as Task[]);

listen<[Task]>("update:tasks", (event) => {
    tasks.value = event.payload;
    console.log(tasks);
});
</script>

<template>
    <DataTable :value="tasks" size="small" tableStyle="min-width: 50rem">
        <Column field="name" header="Name" style="width: 34%"></Column>
        <Column field="tid" header="ID" style="width: 33%"></Column>
        <Column field="kind" header="Type" style="width: 33%"></Column>
    </DataTable>
</template>
