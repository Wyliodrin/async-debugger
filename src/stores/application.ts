import { Application } from "@/types/applications";
import { invoke } from "@tauri-apps/api/core";
import { defineStore } from "pinia";
import { computed, Ref, ref } from "vue";

export const useApplicationStore = defineStore('applications', () => {
    const applications: Ref<Application[]> = ref([]);

    const getApplications = computed(() => applications);

    async function addApplication(title: string, url: string) {
        await invoke("applications_add", { title: title, url: url }).then(
            (uuid) => {
                applications.value.push({
                    id: uuid as string,
                    title: title,
                    url: url,
                    state: 'Enabled',
                    processStatus: "",
                    cpu_usage: 0.0,
                    memory_usage: 0,
                    pid: 0,
                    startTime: '0',
                    connection_status: ""
                });
            }
        );
    }

    async function deleteApplication(appID: string) {
        await invoke("delete_application", { uuid: appID }).then(
            () => {
                applications.value = applications.value.filter(item => item.id !== appID);
            }
        ).catch(
            (error) => console.log("Failed to send delete application command: " + error)
        );
    }

    async function editApplication(app: Application) {
        const indexOfApp = applications.value.findIndex(item => item.id === app.id);
        if (indexOfApp == -1) return;

        const appToEdit = applications.value[indexOfApp];
        if (appToEdit) {
            appToEdit.connection_status = "Connecting";
            await invoke('edit_application', {uuid: app.id, appTitle: app.title, appUrl: app.url, oldTitle: appToEdit.title}).then(
                (uuid) => {
                    appToEdit.title = app.title;
                    appToEdit.url = app.url;
                    appToEdit.id = uuid as string;
                }
            ).catch(
                (e) => console.log("Failed to edit application due to " + e)
            );
            //appToEdit.connection_status = "Connected";
        }
    }

    async function toggleAppState(app: Application) {
        try {
            if (app.state === "Enabled") {
                await invoke("disable_app", { uuid: app.id });
                app.state = "Disabled";
            } else {
                await invoke("enable_app", { uuid: app.id });
                app.state = "Enabled";
            }
        } catch (e) {
            console.error("toggleAppState failed", e);
        }
    }

    return {
        applications, getApplications, addApplication, deleteApplication, editApplication,
        toggleAppState
    }
});