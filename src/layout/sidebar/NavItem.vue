<script setup>
import Icon from './Icon.vue';
const props = defineProps({ item: Object, level: Number });
</script>

<template>
    <v-list-item :to="item.to" rounded class="mb-1">
        <template v-slot:prepend >
            <Icon :item="item.icon" :level="level" class="mr-2"/>
        </template>
        <v-list-item-title>{{ item.title }}</v-list-item-title>
    </v-list-item>
</template>
