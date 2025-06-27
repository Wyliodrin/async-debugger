<script setup>
defineProps({
    item: {
        type: Object,
        required: true
    },
    level: {
        type: Number,
        default: 0
    }
});
</script>

<template>
    <template v-if="level > 0">
        <component :is="item" size="14" stroke-width="1.5" class="iconClass"></component>
    </template>
    <template v-else>
        <component :is="item" size="20" stroke-width="1.5" class="iconClass"></component>
    </template>
</template>
