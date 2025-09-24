<template>
  <v-card elevation="2">
    <v-card-text>
      <h1 class="mb-4 font-weight-bold">Resources</h1>

      <div class="d-flex align-center justify-space-between mb-4">
        <v-text-field
            v-model="resourcesSearch"
            label="Search"
            prepend-inner-icon="mdi-magnify"
            variant="outlined"
            hide-details
            single-line
            class="search-container"
        />
      </div>

      <div class="d-flex flex-wrap mb-4">
        <v-btn
            v-for="app in appList"
            :key="app"
            :color="selectedApp === app ? 'primary' : 'grey lighten-2'"
            variant="tonal"
            size="small"
            class="ma-1"
            @click="selectedApp = app"
        >
          {{ app }}
        </v-btn>
      </div>

      <v-data-table
          :headers="resourcesHeaders"
          :items="filteredResources"
          :item-value="item => `${item.app_name}-${item.id}`"
      >
        <template #item.status="{ item }">
          <v-chip
              :color="getResourceChipColor(item.status)"
              size="small"
              class="text-uppercase"
          >
            {{ item.status }}
          </v-chip>
        </template>

        <template #item.location="{ item }">
          <span v-html="item.location"></span>
        </template>

        <template #item.duration="{ item }">
          <span>{{ formatDuration(item.duration) }}</span>
        </template>
      </v-data-table>
    </v-card-text>
  </v-card>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import type { DataTableHeader } from "vuetify";
import { useDataStore } from "@/stores/data";

export default defineComponent({
  name: "Resources",
  data() {
    const dataStore = useDataStore();

    return {
      dataStore,
      resourcesSearch: "" as string,
      selectedApp: "All" as string,

      resourcesHeaders: [
        { title: "Resource Type", key: "resource_type", align: "center" },
        { title: "ID", key: "id", align: "center" },
        { title: "Status", key: "status", align: "center" },
        { title: "Target", key: "target", align: "center" },
        { title: "Duration", key: "duration", align: "center" },
        { title: "Location", key: "location", align: "center" },
        { title: "Attributes", key: "attributes", align: "center" },
      ] as DataTableHeader[],
    };
  },

  computed: {
    appList(): string[] {
      const names = this.dataStore.resources.map((r) => r.app_name);
      const unique = Array.from(new Set(names)).sort();
      return ["All", ...unique];
    },

    filteredResources(): any[] {
      const q = this.resourcesSearch.toLowerCase();

      return this.dataStore.resources
        .filter(
          (r) => this.selectedApp === "All" || r.app_name === this.selectedApp
        )
        .filter((r) => {
          return (
            r.resource_type.toLowerCase().includes(q) ||
            r.id.toString().includes(q) ||
            r.status.toString().includes(q) ||
            r.location.toLowerCase().includes(q)
          );
        });
    },
  },

  methods: {
    getResourceChipColor(status: string) {
      switch (status) {
        case "Ready":
          return "green";
        default:
          return "gray";
      }
    },

    toNumber(x: unknown): number | undefined {
      if (x === null || x === undefined) return undefined;
      if (typeof x === "number") return x;
      if (typeof x === "string" && x.trim() !== "") {
        const n = Number(x);
        return Number.isNaN(n) ? undefined : n;
      }
      return undefined;
    },

    formatDuration(v: any) {
      if (v == null) return "";
      if (typeof v === "string") return v;
      const secs = this.toNumber(v.secs ?? v.seconds ?? v.Secs) ?? 0;
      const nanos = this.toNumber(v.nanos ?? v.Nanos ?? v.Nano) ?? 0;
      if (secs === undefined) return String(v);
      const ms = secs * 1000 + Math.floor(nanos / 1e6);
      return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
    },
  },
});
</script>
