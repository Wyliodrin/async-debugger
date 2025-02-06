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

listen<[Task]>("task-update", (event) => {
	tasks.value = event.payload;
	console.log("Afisez tasks:");
	console.log(tasks.value[0]);
});
</script>

<template>
	<DataTable :value="tasks">
		<Column field="name" header="Name"></Column>
		<Column field="tid" header="ID"></Column>
		<Column field="kind" header="Type"></Column>
	</DataTable>
</template>
