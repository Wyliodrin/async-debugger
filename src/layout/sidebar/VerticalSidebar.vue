<script setup lang="ts">
import { shallowRef } from 'vue';
import sidebarItems from './sidebarItems';
import NavItem from './NavItem.vue';
import { useLayoutStore } from '@/stores/layout';

const sidebarMenu = shallowRef(sidebarItems);
const layoutStore = useLayoutStore();

const drawer = layoutStore.getSidebarState;
</script>

<template>
    <v-navigation-drawer v-model="drawer" location="left" temporary>
        <v-list class="py-5 px-4 bg-muted">
            <template v-for="item in sidebarMenu">
                <NavItem :item="item" />
            </template>
        </v-list>
    </v-navigation-drawer>
</template>
