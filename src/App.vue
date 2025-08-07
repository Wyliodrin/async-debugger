<script setup lang="ts">
import { RouterView } from 'vue-router';
import { useDataStore } from './stores/data';

const pauseStore = useDataStore();
window.addEventListener('keydown', (e: KeyboardEvent) => {
  if (e.code === 'Space') {
    const activeElement = document.activeElement;

    const isTypingInInput =
      activeElement instanceof HTMLInputElement ||
      activeElement instanceof HTMLTextAreaElement ||
      (activeElement instanceof HTMLElement && activeElement.isContentEditable);

    if (isTypingInInput) {
      return;
    }
    e.preventDefault()
    pauseStore.togglePause();
  }
})
</script>

<template>
    <RouterView></RouterView>
</template>