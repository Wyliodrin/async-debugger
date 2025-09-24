<template>
    <v-list-item :to="item.to" rounded class="mb-1">
        <template v-slot:prepend >
            <Icon :item="item.icon" :level="level" class="mr-2"/>
        </template>
        <v-list-item-title>{{ item.title }}</v-list-item-title>
    </v-list-item>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import Icon from "./Icon.vue";

export default defineComponent({
  name: "NavItem",
  components: {
    Icon,
  },
  props: {
    item: {
      type: Object,
      required: true,
    },
    level: {
      type: Number,
      required: true,
    },
  },
});
</script>
