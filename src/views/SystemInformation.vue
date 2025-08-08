<script setup lang="ts">
import { useApplicationStore } from '@/stores/application';
import { Application } from '@/types/applications';
import { computed, Ref, ref } from 'vue';
import { PlayerPlayFilledIcon, PlayerPauseFilledIcon, PencilIcon, TrashIcon, PlusIcon } from 'vue-tabler-icons';
import { listen } from '@tauri-apps/api/event';
import { useDataStore } from '@/stores/data';

const applicationsStore = useApplicationStore();
const dataStore = useDataStore();
const errorMessage = ref('');
const errorSnackbar = ref(false);

const applicationHeaders: any = ref([
    { title: "Status", align: 'center', key: 'connection_status'},
    { title: "Name", align: 'center', key: 'title' },
    { title: "PID", align: 'center', key: 'pid' }, //
    { title: "URL", align: 'center', key: 'url' },
    { title: "State", align: 'center', key: 'state' },
    { title: "Start Time", align: 'center', key: 'start_time' }, //
    { title: "CPU", align: 'center', key: 'cpu_usage' }, // 
    { title: "Memory", align: 'center', key: 'memory_usage' }, //
    { title: "Actions", align: 'center', key: 'actions', sortable: false }
]);

const getStateChipColor = (state: string): string => {
    const colorMap: Record<string, string> = {
        'Enabled': 'green',
        'Disabled': 'red',
    };
    return colorMap[state] || 'default';
};

const getRowProps = (item: any) => {
    return {
        style: item.item.state === 'Disabled'
            ? { backgroundColor: '#F5F5F5' }
            : {},
    };
};

const editedItem: Ref<{
    connection_status: string,
    pid: any;
    state: any;
    startTime: any;
    cpu_usage: any;
    memory_usage: any;
    processStatus: any;
    id: string,
    title: string,
    url: string,
}> = ref({
    connection_status: '',
    id: '',
    title: '',
    url: '',
    pid: '',
    state: '',
    startTime: '',
    cpu_usage: '',
    memory_usage: '',
    processStatus: '',
});

const defaultItem: Ref<{
    connection_status: string,
    pid: any;
    state: any;
    startTime: any;
    cpu_usage: any;
    memory_usage: any;
    processStatus: any;
    id: string,
    title: string,
    url: string,
}> = ref({
    connection_status: '',
    id: '',
    title: '',
    url: '',
    pid: '',
    state: '',
    startTime: '',
    cpu_usage: '',
    memory_usage: '',
    processStatus: '',
});

const valid = ref(true);
const dialog = ref(false);
const applications = ref('');
const editedIndex = ref(-1);
const editedItemName = ref('');

const formTitle = computed(() => {
    return editedIndex.value === -1 ? 'New Application' : 'Edit Application';
});

function close() {
    dialog.value = false;

    editedItem.value = Object.assign({}, defaultItem.value);
    editedIndex.value = -1;
    editedItemName.value = '';
}

function showError(msg: string) {
  errorMessage.value = msg;
  errorSnackbar.value = true;
}

async function save() {
    dialog.value = false;
    const currentApplication = {
        connection_status: editedItem.value.connection_status,
        pid: editedItem.value.pid,
        id: editedItem.value.id,
        title: editedItem.value.title,
        url: editedItem.value.url,
        state: editedItem.value.state,
        startTime: editedItem.value.startTime,
        cpu_usage: editedItem.value.cpu_usage,
        memory_usage: editedItem.value.memory_usage,
        processStatus: editedItem.value.processStatus
    }

    if (editedIndex.value > -1) {
        await applicationsStore.editApplication(currentApplication);
    } else {
        try{
            await applicationsStore.addApplication(currentApplication.title, currentApplication.url);
        }
        catch (error) {
            if (typeof error == 'string'){
                if (error.includes("ApplicationAlreadyConnected")){
                    showError("This URL Application is already in the list");
                }
                else if (error.includes("PIDNotFound")){
                    showError("The application at this URL is not running");
                }
                else{
                    showError("Unexpected Error" + error);
                }
            }
            else{
                showError("Unexpected Error" + error);
            }
        }
    }

    close();
}

async function deleteApp(appID: string) {
    if (await confirm('Are you sure you want to delete this project?')) {
        await applicationsStore.deleteApplication(appID);
    }
}

function editApp(app: Application) {
    editedIndex.value = applicationsStore.getApplications.value.indexOf(app);

    const { title, url, id } = app;

    editedItemName.value = title;

    editedItem.value.id = id;
    editedItem.value.title = title;
    editedItem.value.url = url;

    dialog.value = true;
}

listen<Application[]>("update:applications", (event) => {
    if (dataStore.pause == false) {
        console.log("Received applications: " + JSON.stringify(event.payload[0]));
        event.payload.forEach(newApp => {
            console.log(newApp);
            const existingApp = applicationsStore.applications.find(app => app.id === newApp.id);
            if (existingApp) {
                Object.assign(existingApp, newApp);
            } else {
                applicationsStore.applications.push(newApp);
            }
        });
    }
});

listen<{ id: string; pid: number }>('update:pid', (event) => {
  const { id, pid } = event.payload;
  const app = applicationsStore.applications.find(a => a.id === id);
  if (app) {
    app.pid = pid;
  }
});


</script>

<template>
    <v-card elevation="2">
        <template v-slot:text>
            <h1 class="mb-4 font-weight-bold">Applications traced</h1>

            <div class="d-flex align-center justify-space-between">
                <div class="search-container">
                    <v-text-field v-model="applications" label="Search" prepend-inner-icon="mdi-magnify"
                        variant="outlined" hide-details single-line></v-text-field>
                </div>
                <v-btn color="primary" @click="dialog = true">
                    <PlusIcon stroke-width="1.5" size="25" class="mr-1" />
                    Add new application
                </v-btn>
            </div>
        </template>
        <v-snackbar
            v-model="errorSnackbar"
            :timeout="6000"
            color="error"
            top
            right
        >
            {{ errorMessage }}
            
            <v-btn
            color="white"
            variant="text"
            @click="errorSnackbar = false"
            >
            Close
            </v-btn>
        </v-snackbar>

        <v-dialog v-model="dialog" max-width="500" persistent>
            <v-card v-click-outside="close">
                <v-card-title class="pa-4 bg-primary">
                    <span class="title text-white">{{ formTitle }}</span>
                </v-card-title>

                <v-card-text>
                    <v-form ref="form" v-model="valid" lazy-validation @submit.prevent>
                        <v-row align="center">
                            <v-col cols="12">
                                <v-text-field variant="outlined" hide-details v-model="editedItem.title"
                                    label="Application Name"></v-text-field>
                            </v-col>
                        </v-row>

                        <v-row align="center">
                            <v-col cols="12">
                                <v-text-field variant="outlined" hide-details v-model="editedItem.url"
                                    label="Application URL"></v-text-field>
                            </v-col>
                        </v-row>
                    </v-form>
                </v-card-text>

                <v-card-actions class="pa-4">
                    <v-spacer></v-spacer>
                    <v-btn color="error" variant="flat" @click="close">Cancel</v-btn>
                    <v-btn color="primary" :disabled="editedItem.title === '' || editedItem.url === ''" variant="flat"
                        @click="save">Save</v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>

        <v-data-table :search="applications" :headers="applicationHeaders"
            :items="applicationsStore.getApplications.value" :row-props="getRowProps">
            <template v-slot:item.connection_status="{ item }">
                <v-progress-circular
                    v-if="item.connection_status == 'Connecting'"
                    indeterminate
                    color="primary"
                    size="24"
                />
                <v-icon v-else-if="item.connection_status == 'Connected'" color="green">mdi-check-circle</v-icon>
                <v-icon v-else color="red">mdi-alert-circle</v-icon>
            </template>
            <template v-slot:item.state="{ item }">
                <div class="justify-center">
                    <v-chip :color="getStateChipColor(item.state)" class="text-uppercase" label size="small">
                        <div v-if="item.state === 'Enabled'">Enabled</div>
                        <div v-else>Disabled</div>
                    </v-chip>
                </div>
            </template>
            <template v-slot:item.memory_usage="{ item }">
                <div class="justify-center">
                    {{ item.memory_usage }} MB
                </div>
            </template>
            <template v-slot:item.actions="{ item }">
                <v-btn icon flat @click="applicationsStore.toggleAppState(item)"
                    :class="item.state === 'Disabled' ? 'disabled-action-btn' : ''">
                    <PlayerPlayFilledIcon v-if="item.state === 'Disabled'" stroke-width="1.5" size="20"
                        class="text-primary" />
                    <PlayerPauseFilledIcon v-else stroke-width="1.5" size="20" class="text-primary" />
                </v-btn>
                <v-tooltip text="Edit">
                    <template v-slot:activator="{ props }">
                        <v-btn icon flat @click="editApp(item)" v-bind="props"
                            :class="item.state === 'Disabled' ? 'disabled-action-btn' : ''">
                            <PencilIcon stroke-width="1.5" size="20" class="text-primary" />
                        </v-btn>
                    </template>
                </v-tooltip>
                <v-tooltip text="Delete">
                    <template v-slot:activator="{ props }">
                        <v-btn icon flat @click="deleteApp(item.id)" v-bind="props"
                            :class="item.state === 'Disabled' ? 'disabled-action-btn' : ''">
                            <TrashIcon stroke-width="1.5" size="20" class="text-error" />
                        </v-btn>
                    </template>
                </v-tooltip>
            </template>
        </v-data-table>
    </v-card>
</template>

<style scoped>
.disabled-action-btn {
    opacity: 0.7;
    background-color: transparent !important;
    box-shadow: none !important;
    border: none !important;
}

.disabled-action-btn::before {
    opacity: 0 !important;
}

.disabled-action-btn:hover {
    background-color: transparent !important;
}

.text-white {
    color: rgb(255, 255, 255) !important;
}

.search-container {
    width: 400px;
}
</style>