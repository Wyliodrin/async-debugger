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
    </div>
</template>

<script setup lang="ts">
import { ref, Ref } from "vue";

import Button from "primevue/button";
import Toolbar from "primevue/toolbar";

import { Application } from "../types/applications";
import EditApplications from "./EditApplication.vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const showEditApplication = ref(false);
const editApplication: Ref<Application | undefined> = ref(undefined);

function addApplication() {
    console.log("click");
    editApplication.value = undefined;
    showEditApplication.value = true;
}

async function saveEditApplication(app: Application) {
    showEditApplication.value = false;
    console.log("sending");
    console.log(
        await invoke("applications_add", { title: app.title, url: app.url }),
    );
    console.log("done");
}

function closeEditApplication() {
    showEditApplication.value = false;
}

listen<[Application]>("update:applications", (event) => {
    console.log(event.payload[0]);
});
</script>
