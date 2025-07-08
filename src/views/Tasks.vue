<script setup lang="ts">
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { Task } from "@/types/tasks";

const tasks = ref([] as Task[]);
const tasksSearch = ref('');
let pause = ref(false);

const taskHeaders: any = ref([
    { title: "App Name", key: "app_name", align: "center" },
    { title: "ID", key: "id", align: "center" },
    { title: "TID", key: "tid", align: "center" },
    { title: "Name", key: "name", align: "center" },
    { title: "Type", key: "kind", align: "center" },
    { title: "State", key: "state", align: "center" },
    { title: "Spawned time", key: "nice_created_at", align: "center"},
    { title: "Runtime", key: "runtime", align: "center" },
    { title: "Scheduled", key: "scheduled", align: "center" },
    { title: "Idle", key: "idle", align: "center" },
    { title: "Busy", key: "busy", align: "center" },
    { title: "Location", key: "location", align: "center" },

]);

function getTaskChipColor(stateOrKind: string): string {
    switch (stateOrKind) {
        case "Running": return "green";
        case "Stopped": return "red";
        case "SPAWN": return "blue";
        case "BLOCKING": return "orange";
        default: return "grey";
    }
};

window.addEventListener("keydown", (event: KeyboardEvent) => {
    if (event.code === "Space") {
        console.log("daa");
        event.preventDefault();
        pause.value = !pause.value;
    }
});

listen<any[]>("update:tasks", (e) => {
    if (pause.value == false) {
        tasks.value = e.payload.map((t: any) => {
        const formattedFields = {
        runtime: t.runtime.formatted,
        scheduled: t.scheduled.formatted,
        idle: t.idle.formatted,
        busy: t.busy.formatted,
        }
        if (typeof t.state === "string") {
            return { ...t, 
                state: t.state,
                ...formattedFields,
                }
        }
        const keys = Object.keys(t.state ?? {})
        const stateKey = keys.length ? keys[0] : "Unknown"

        return {
            ...t,
            state: stateKey,
            ...formattedFields
        }
    })
        console.log("Afisez task " + JSON.stringify(tasks.value[0]));
        }
})

</script>

<template>
    <v-card elevation="2">
        <v-card-text>
            <div class="d-flex align-center justify-space-between mb-4">
                <div>
                <v-text-field v-model="tasksSearch" label="Search" prepend-inner-icon="mdi-magnify" variant="outlined"
                    hide-details single-line class="search-container" />
                </div>
                <v-chip
                    :color="pause === false ? 'green' : 'red'"
                    dark
                    class="ma-2"
                >
                    {{ pause === false ? 'Connected' : 'Paused' }}
                </v-chip>
            </div>

            <v-data-table :headers="taskHeaders" :items="tasks" :search="tasksSearch">
                <template #item.kind="{ item }">
                    <v-chip :color="getTaskChipColor(item.kind)" class="text-uppercase" small>
                        {{ item.kind }}
                    </v-chip>
                </template>

                <template #item.state="{ item }">
                    <v-chip :color="getTaskChipColor(item.state)" small>
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
