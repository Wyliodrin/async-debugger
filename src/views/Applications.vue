<template>
    <div>
        <EditApplications
            :application="editApplication"
            :visible="showEditApplication"
            @save="saveEditApplication"
            @close="closeEditApplication"
        />
        <Toolbar>
            <template #start>
                <Button
                    icon="pi pi-plus"
                    class="mr-2"
                    severity="secondary"
                    text
                    @click="addApplication"
                />
            </template>
        </Toolbar>

        <DataTable :value="applications" size="small" tableStyle="min-width: 50rem">
        <Column field="title" header="Name" style="width: 34%"></Column>
        <Column field="id" header="Uuid" style="width: 33%"></Column>
        <Column field="url" header="Url" style="width: 33%"></Column>
        <!-- <Column header="Remove">
            <template #body="slotProps">
                <Button
            icon="pi pi-minus"
                    class="mr-2"
                    severity="secondary"
                    text
                    @click="removeApplication(slotProps.data.title)"
                />
            </template>
            </Column> -->
    </DataTable>
    </div>
</template>

<script setup lang="ts">
import { ref, Ref } from "vue";

import Button from "primevue/button";
import Toolbar from "primevue/toolbar";
import Column from "primevue/column";
import DataTable from "primevue/datatable";
import { Application } from "../types/applications";
import EditApplications from "./EditApplication.vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const showEditApplication = ref(false);
const editApplication: Ref<Application | undefined> = ref(undefined);

const applications = ref([] as Application[]);

function addApplication() {
    console.log("Showing add application popup");
    editApplication.value = undefined;
    showEditApplication.value = true;
}

async function removeApplication(app: Application) {
    console.log("Removing application");
    try {
        await invoke("delete_application", { uuid: app.id})
    } catch (error) {
        console.log("Failed to send delete application command: " + error);
    }
}

async function saveEditApplication(app: Application) {
    showEditApplication.value = false;

    try {
        await invoke("applications_add", { title: app.title, url: app.url });
    } catch (error) {
        console.log("Failed to send add application command: " + error);
    }
}

function closeEditApplication() {
    showEditApplication.value = false;
}

listen<[Application]>("update:applications", (event) => {
    applications.value = event.payload;
    console.log("Received applications: " + JSON.stringify(event.payload[0]));
});
</script>
