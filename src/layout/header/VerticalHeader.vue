<script setup lang="ts">
import { useLayoutStore } from '@/stores/layout';
const layoutStore = useLayoutStore();
</script>

<template>
    <v-app-bar elevation="1">
      <v-app-bar-nav-icon variant="text" @click.stop="layoutStore.triggerSidebar()" class="mr-1"></v-app-bar-nav-icon>
      <div class="d-flex align-center flex-grow-1">
        <img class="mr-2" src="../../assets/logo.png" width="30" height="30">
        <v-toolbar-title class="text-no-wrap">Async Debug Tool</v-toolbar-title>
      </div>
    </v-app-bar>
  </template>