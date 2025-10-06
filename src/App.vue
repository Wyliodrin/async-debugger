<template>
  <v-app>
    <v-main>
      <router-view />
    </v-main>
  </v-app>
</template>

<script lang="ts">
import { defineComponent, onMounted, onBeforeUnmount } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useDataStore } from "@/stores/data";
import { useApplicationStore } from "@/stores/application";
import type { Task } from "@/types/tasks";
import type { Resource } from "@/types/resources";
import type { Poll } from "@/types/polls";

export default defineComponent({
  name: "AppShell",
  setup() {
    const dataStore = useDataStore();
    const applicationsStore = useApplicationStore();
    const unlisteners: UnlistenFn[] = [];

    const onKeydown = (e: KeyboardEvent) => {
      if (e.code !== "Space") return;

      const active = document.activeElement;
      const isTypingInInput =
        active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        (active instanceof HTMLElement && active.isContentEditable);

      if (isTypingInInput) return;

      e.preventDefault();
      dataStore.togglePause();
    };

    onMounted(async () => {
      // global spacebar toggle (unless typing)
      window.addEventListener("keydown", onKeydown, { passive: false });

      // pid updates
      const unlistenPid = await listen<{ id: string; pid: number }>("update:pid", (evt) => {
        const { id, pid } = evt.payload;
        const app = applicationsStore.applications.find(a => a.id === id);
        if (app) app.pid = pid;
      });

      // task updates
      const unlistenTasks = await listen("update:tasks", (evt: { payload: Task[] }) => dataStore.handleTaskUpdate(evt));

      // resources updates
      const unlistenResources = await listen("update:resources", (evt: { payload: Resource[] }) => dataStore.handleResourceUpdate(evt));

      // polls updates
      const unlistenPolls = await listen("update:polls", (evt: { payload: Poll[] }) => dataStore.handlePollUpdate(evt));

      unlisteners.push(unlistenPid, unlistenTasks, unlistenResources, unlistenPolls);
    });

    onBeforeUnmount(() => {
      window.removeEventListener("keydown", onKeydown);
      unlisteners.forEach((fn) => {
        try { fn(); } catch (err) {
          console.log("unlisten failed: ", err);
        }
      });
    });

    return {};
  },
});
</script>
