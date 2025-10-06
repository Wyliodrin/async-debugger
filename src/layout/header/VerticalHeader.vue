<template>
  <v-app-bar elevation="1">
    <v-app-bar-nav-icon variant="text" @click.stop="layoutStore.triggerSidebar()" class="mr-1"></v-app-bar-nav-icon>
    <div class="d-flex align-center flex-grow-10">
      <img class="mr-2" src="../../assets/logo.png" width="30" height="30">
      <v-toolbar-title class="text-no-wrap">Async Debug Tool</v-toolbar-title>
      <div class="ml-5">
      <v-chip :color="dataStore.pause ? 'red' : 'green'" dark>
        {{ dataStore.pause ? 'Paused' : 'Connected' }}
      </v-chip>
      </div>
    </div>
  </v-app-bar>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { useLayoutStore } from "@/stores/layout";
import { useDataStore } from "@/stores/data";

export default defineComponent({
  name: "VerticalHeader",
  data() {
    return {
      layoutStore: useLayoutStore(),
      dataStore: useDataStore(),
    };
  },
});
</script>
