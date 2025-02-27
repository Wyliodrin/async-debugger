<template>
    <Dialog
        :visible="visible"
        @update:visible="changeVisibility"
        modal
        :header="application ? 'Edit' : 'New' + ' Application'"
        :style="{ width: '25rem' }"
    >
        <div class="flex items-center gap-4 mb-4">
            <label for="title" class="font-semibold w-24">Title</label>
            <InputText
                id="title"
                class="flex-auto"
                autocomplete="off"
                v-model="title"
            />
        </div>
        <div class="flex items-center gap-4 mb-8">
            <label for="url" class="font-semibold w-24">URL</label>
            <InputText
                id="url"
                class="flex-auto"
                autocomplete="off"
                v-model="url"
            />
        </div>
        <div class="flex justify-end gap-2">
            <Button
                type="button"
                label="Cancel"
                severity="secondary"
                @click="close"
            ></Button>
            <Button type="button" label="Save" @click="save"></Button>
        </div>
    </Dialog>
</template>

<script setup lang="ts">
import { Dialog, InputText, Button } from "primevue";
import * as uuid from "uuid";
import { ref, Ref, watch } from "vue";

import { Application } from "../types/applications";

const title: Ref<string> = ref("");
const url: Ref<string> = ref("");

let props = defineProps<{
    application?: Application;
    visible: boolean;
}>();

const emit = defineEmits<{
    (e: "save", a: Application): void;
    (e: "close"): void;
}>();

watch(
    () => props.visible,
    (isVisible) => {
        if (isVisible) {
            if (props.application) {
                title.value = props.application.title;
                url.value = props.application.url;
            } else {
                title.value = "";
                url.value = "";
            }
        }
    },
);

function changeVisibility(visibility: boolean) {
    if (!visibility) {
        close();
    }
}

function save() {
    let app = props.application;
    if (!app) {
        app = {
            id: uuid.v4(),
            title: "",
            url: "",
        };
    }
    app.title = title.value;
    app.url = url.value;
    emit("save", app);
}

function close() {
    emit("close");
}
</script>
