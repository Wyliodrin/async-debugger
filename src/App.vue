<script setup lang="ts">
import Tabs from "primevue/tabs";
import TabList from "primevue/tablist";
import Tab from "primevue/tab";
import TabPanels from "primevue/tabpanels";
import TabPanel from "primevue/tabpanel";

import Tasks from "./views/Tasks.vue";
import Applications from "./views/Applications.vue";
</script>

<template>
    <Tabs value="0">
        <TabList>
            <Tab value="0">Tasks</Tab>
            <Tab value="1">Resources</Tab>
            <Tab value="2">Applications</Tab>
        </TabList>
        <TabPanels>
            <TabPanel value="0">
                <Tasks />
            </TabPanel>
            <TabPanel value="1"> </TabPanel>
            <TabPanel value="2">
                <Applications />
            </TabPanel>
        </TabPanels>
    </Tabs>
</template>

<style>
html {
    font-size: 14px;
}

body {
    margin: 0px;
    min-height: 100%;
    overflow-x: hidden;
    overflow-y: auto;
    background-color: var(--body-bg);
    font-weight: normal;
    font-family:
        system-ui,
        -apple-system,
        BlinkMacSystemFont,
        "Segoe UI",
        Roboto,
        Oxygen,
        Ubuntu,
        Cantarell,
        "Open Sans",
        "Helvetica Neue",
        sans-serif;
    color: var(--body-text-color);
    padding: 1rem;
}

.card {
    background: var(--card-bg);
    border: var(--card-border);
    padding: 2rem;
    border-radius: 10px;
    margin-bottom: 1rem;
}
</style>
