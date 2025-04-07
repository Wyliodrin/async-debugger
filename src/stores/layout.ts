import { defineStore } from "pinia";
import { computed, ref } from "vue";

export const useLayoutStore = defineStore('layout', () => {
    const isSidebarTriggered = ref(false);

    const getSidebarState = computed(() => isSidebarTriggered);

    function triggerSidebar() {
        isSidebarTriggered.value = !isSidebarTriggered.value;
    }

    return {
        isSidebarTriggered, getSidebarState, triggerSidebar
    }
});