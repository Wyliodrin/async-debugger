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
                    state: 'Enabled'
                });
            }
        ).catch(
            (error) => console.log("Failed to send add application command: " + error)
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

        if (indexOfApp !== -1) {
            applications.value[indexOfApp] = app;
        }
    }

    // TODO cheama din functiile Amaliei
    async function toggleAppState(appID: string) {
        const application = applications.value.find(item => item.id === appID);
        if (application) {
            application.state = application.state === 'Enabled' ? 'Disabled' : 'Enabled';
        }
    };

    return {
        applications, getApplications, addApplication, deleteApplication, editApplication, toggleAppState
    }
});