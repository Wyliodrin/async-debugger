<template>
    <v-navigation-drawer v-model="drawer" location="left" temporary>
        <v-list class="py-5 px-4 bg-muted">
          <NavItem
            v-for="item in sidebarMenu"
            :key="item.to"
            :item="item"
            :level="0"
          />
        </v-list>
    </v-navigation-drawer>
</template>

<script lang="ts">
import { defineComponent, shallowRef } from "vue";
import sidebarItems from "./sidebarItems";
import NavItem from "./NavItem.vue";
import { useLayoutStore } from "@/stores/layout";

export default defineComponent({
  name: "SidebarMenu",
  components: {
    NavItem,
  },
  data() {
    const layoutStore = useLayoutStore();
    const drawer = layoutStore.getSidebarState;
    return {
      sidebarMenu: shallowRef(sidebarItems),
      layoutStore,
      drawer,
    };
  },
});
</script>
