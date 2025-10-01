<template>
    <v-app>
        <VerticalHeader />

        <VerticalSidebar />

        <v-main>
            <v-container fluid>
                <RouterView />
            </v-container>
        </v-main>
    </v-app>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import { useLayoutStore } from "@/stores/layout";
import VerticalSidebar from "./sidebar/VerticalSidebar.vue";
import VerticalHeader from "./header/VerticalHeader.vue";

export default defineComponent({
  name: "AppLayout",
  components: {
    VerticalSidebar,
    VerticalHeader,
  },
  data() {
    return {
      layoutStore: useLayoutStore(),
    };
  },
});
</script>
