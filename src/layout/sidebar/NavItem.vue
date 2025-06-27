<script setup>
import Icon from './Icon.vue';
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
    <v-list-item :to="item.to" rounded class="mb-1">
        <template #prepend >
            <Icon :item="item.icon" :level="level" class="mr-2"/>
        </template>
        <v-list-item-title>{{ item.title }}</v-list-item-title>
    </v-list-item>
</template>
