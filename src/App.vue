<script setup lang="ts">
import { ref } from "vue";
// import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Task = {
    id: number;
    tid?: number;
    name?: string;
    kind?: string;
};

listen<[Task]>("task-update", (event) => {
    tasks.value = event.payload;
});

const headers = [
    {
        align: "start",
        key: "tid",
        sortable: false,
        title: "Id",
    },
    { key: "name", title: "Name" },
    { key: "kind", title: "Kind" },
];

const tasks = ref([] as Task[]);
</script>

<template>
    <v-app>
        <v-data-table :headers="headers" :items="tasks"></v-data-table>
    </v-app>
</template>
