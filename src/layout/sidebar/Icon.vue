<template>
    <template v-if="level > 0">
        <component :is="item" size="14" stroke-width="1.5" class="iconClass"></component>
    </template>
    <template v-else>
        <component :is="item" size="20" stroke-width="1.5" class="iconClass"></component>
    </template>
</template>

<script lang="ts">
import { defineComponent } from "vue";

export default defineComponent({
  name: "Icon",
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
