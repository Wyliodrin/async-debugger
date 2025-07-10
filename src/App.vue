<script setup lang="ts">
import { RouterView } from 'vue-router';
import { useDataStore } from './stores/data';

const pauseStore = useDataStore();
window.addEventListener('keydown', (e: KeyboardEvent) => {
  if (e.code === 'Space') {
    e.preventDefault()
    pauseStore.togglePause();
  }
})
</script>

<template>
    <RouterView></RouterView>
</template>