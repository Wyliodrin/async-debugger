<script setup lang="ts">
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { Task } from "@/types/tasks";

const tasks = ref([] as Task[]);
const tasksSearch = ref('');

const taskHeaders: any = ref([
    { title: "App UUID", align: 'center', key: 'app_id'},
    { title: "ID", align: 'center', key: 'id' },
    { title: "TID", align: 'center', key: 'tid' },
    { title: "Name", align: 'center', key: 'name' },
    { title: "Type", align: 'center', key: 'kind' },

]);

const getTaskChipColor = (state: string): string => {
    const colorMap: Record<string, string> = {
        'SPAWN': 'green',
        'BLOCKING': 'red'
    };
    return colorMap[state] || 'default';
};
listen<Task[]>("update:tasks", (event) => {
    tasks.value = event.payload;
    console.log("Afisez task " + JSON.stringify(tasks.value[0]));
});
</script>

<template>
    <v-card elevation="2">
        <template v-slot:text>
            <div class="d-flex align-center justify-space-between">
                <div class="search-container">
                    <v-text-field v-model="tasksSearch" label="Search" prepend-inner-icon="mdi-magnify"
                        variant="outlined" hide-details single-line></v-text-field>
                </div>
            </div>
        </template>

        <v-data-table :search="tasksSearch" :headers="taskHeaders" :items="tasks">
            <template v-slot:item.kind="{ item }">
                <div class="justify-center">
                    <v-chip :color="getTaskChipColor(item.kind)" class="text-uppercase" label size="small">
                        <div>{{ item.kind }}</div>
                    </v-chip>
                </div>
            </template>
        </v-data-table>
    </v-card>
</template>

<style scoped>
.search-container {
    width: 400px;
}
</style>