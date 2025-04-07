<script setup lang="ts">
import { useLayoutStore } from '@/stores/layout';
import VerticalSidebar from './sidebar/VerticalSidebar.vue';
import VerticalHeader from './header/VerticalHeader.vue';

const layoutStore = useLayoutStore();

</script>

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
