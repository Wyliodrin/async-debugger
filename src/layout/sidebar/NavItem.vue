<template>
  <v-list-item :to="item.to" rounded class="mb-1">
    <template v-slot:prepend>
      <SidebarIcon :item="item.icon" :level="level" class="mr-2" />
    </template>
    <v-list-item-title>{{ item.title }}</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts">
import { defineComponent, type PropType } from "vue";
import SidebarIcon from "./Icon.vue";
import type { sidebarItem } from "./sidebarItems";

export default defineComponent({
  name: "NavItem",
  components: { SidebarIcon },
  props: {
    item: {
      type: Object as PropType<sidebarItem>,
      required: true,
    },
    level: {
      type: Number,
      required: true,
    },
  },
});
</script>
