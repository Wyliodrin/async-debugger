<script setup lang="ts">
import { ref, computed, watch, nextTick, onBeforeUnmount } from 'vue'
import { listen } from '@tauri-apps/api/event'
import throttle from 'lodash.throttle'
import {
  Chart,
  BarController,
  BarElement,
  TimeScale,
  LinearScale,
  CategoryScale,
  Title,
  Tooltip,
  Legend,
  type ChartOptions,
  type LegendItem
} from 'chart.js'
import zoomPlugin from 'chartjs-plugin-zoom'
import datalabelsPlugin from 'chartjs-plugin-datalabels'
import 'chartjs-adapter-date-fns'

Chart.register(
  BarController,
  BarElement,
  TimeScale,
  LinearScale,
  CategoryScale,
  Title,
  Tooltip,
  Legend,
  zoomPlugin,
  datalabelsPlugin
)

import type { TimeStamp, TaskOpMap } from '@/types/async_ops'
import { useDataStore } from '@/stores/data'
const dataStore = useDataStore()

function toMillis(ts: TimeStamp | null): number {
  if (!ts) return 0
  return ts.seconds * 1000 + ts.nanos / 1_000_000
}

interface Operation {
  start: number
  end: number
  type: string
}
interface Task {
  name: string
  operations: Operation[]
}
interface OpDatum {
  x: [number, number]
  y: string
  label: string
  backgroundColor: string
  borderColor: string
  borderWidth: number
}

const loaded = ref(false)
const rawOps = ref<TaskOpMap>({})
const canvasRef = ref<HTMLCanvasElement>()
let chart: Chart<'bar'> | null = null

const tasks = computed<Task[]>(() => {
  return Object.entries(rawOps.value).map(([name, entry]) => ({
    name,
    operations: entry.operations
      .filter(o => o.started_at && o.stopped_at)
      .map(o => ({
        start: toMillis(o.started_at),
        end: toMillis(o.stopped_at),
        type: o.resource_target ?? 'unknown'
      }))
  }))
})

// compute global time bounds
const dataMin = computed(() => {
  const all = tasks.value.flatMap(t => t.operations.map(o => o.start))
  return all.length ? Math.min(...all) : 0
})
const dataMax = computed(() => {
  const all = tasks.value.flatMap(t => t.operations.map(o => o.end))
  return all.length ? Math.max(...all) : 0
})

// slider range, initialized to zero until we get data
const sliderRange = ref<[number, number]>([0, 0])
const stepSize = 1 // ms

// color palettes
const palette = [
  'rgba(255,99,132,0.5)',
  'rgba(54,162,235,0.5)',
  'rgba(255,206,86,0.5)',
  'rgba(75,192,192,0.5)',
  'rgba(153,102,255,0.5)',
  'rgba(255,159,64,0.5)'
]
const borderPalette = [
  'rgb(255,99,132)',
  'rgb(54,162,235)',
  'rgb(255,206,86)',
  'rgb(75,192,192)',
  'rgb(153,102,255)',
  'rgb(255,159,64)'
]

const typeColors = computed(() => {
  const types = Array.from(
    new Set(tasks.value.flatMap(t => t.operations.map(o => o.type)))
  ).sort()
  const map: Record<string, { bg: string, border: string }> = {}
  types.forEach((ty, i) => {
    map[ty] = {
      bg: palette[i % palette.length]!,
      border: borderPalette[i % borderPalette.length]!
    }
  })
  return map
})

function buildDataset(): OpDatum[] {
  return tasks.value.flatMap((task) =>
    task.operations.map((op, i) => {
      const { bg, border } = typeColors.value[op.type]!
      return {
        x: [op.start, op.end],
        y: task.name,
        label: `${op.type.charAt(0) || '#'}${i + 1}`,
        backgroundColor: bg,
        borderColor: border,
        borderWidth: 1
      }
    })
  )
}

const chartOptions = computed<ChartOptions<'bar'>>(() => ({
  indexAxis: 'y',
  responsive: true,
  scales: {
    x: {
      type: 'time',
      min: sliderRange.value[0],
      max: sliderRange.value[1],
      title: { display: true, text: 'Time' }
    },
    y: {
      type: 'category',
      title: { display: true, text: 'Task' }
    }
  },
  plugins: {
    legend: {
      display: true,
      position: 'bottom',
      labels: {
        generateLabels: () => {
          return Object.entries(typeColors.value).map(([text, { bg, border }]) => ({
            text,
            fillStyle: bg,
            strokeStyle: border,
            lineWidth: 1
          })) as LegendItem[]
        }
      }
    },
    tooltip: {
      callbacks: {
        label(ctx) {
          const raw = ctx.raw as OpDatum
          const [s, e] = raw.x
          return `${new Date(s).toLocaleTimeString()} → ${new Date(e).toLocaleTimeString()}`
        }
      }
    },
    zoom: {
      pan: { enabled: true, mode: 'x' },
      zoom: { wheel: { enabled: true }, pinch: { enabled: true }, mode: 'x' }
    },
    datalabels: {
      color: '#000', font: { size: 10, weight: 'bold' },
      anchor: 'center', align: 'center',
      formatter: (v) => (v as OpDatum).label
    }
  }
}))

function initChart() {
  if (!canvasRef.value || chart) return
  chart = new Chart(canvasRef.value, {
    type: 'bar',
    data: {
      datasets: [{
        data: buildDataset(),
        backgroundColor: (ctx) => (ctx.raw as OpDatum).backgroundColor,
        borderColor: (ctx) => (ctx.raw as OpDatum).borderColor,
        borderWidth: (ctx) => (ctx.raw as OpDatum).borderWidth
      }]
    },
    options: chartOptions.value
  })
}

function updateChartData() {
  if (!chart) return
  // update slider if we have new bounds
  sliderRange.value = [
    Math.min(sliderRange.value[0], dataMin.value),
    Math.max(sliderRange.value[1], dataMax.value)
  ]
  // swap in the new data
  chart.data.datasets![0].data = buildDataset()
  chart.update('none')
}

watch(sliderRange, ([min, max]) => {
  if (!chart) return
  chart.options.scales!['x']!.min = min
  chart.options.scales!['x']!.max = max
  chart.update('none')
}, { deep: true })

function resetView() {
  sliderRange.value = [dataMin.value, dataMax.value]
  chart?.resetZoom()
}

const formatTime = (ts: number) => new Date(ts).toLocaleTimeString()

const onPayload = throttle((payload: TaskOpMap) => {
  // If the user has paused the updates, do nothing
  if (dataStore.pause) {
    return
  }

  rawOps.value = payload

  // first time ever?
  if (!loaded.value) {
    loaded.value = true
    // initialize slider to data‐bounds
    sliderRange.value = [dataMin.value, dataMax.value]
    // mount the chart on nextTick
    nextTick(initChart)
  } else {
    // subsequent updates: just refill the dataset
    updateChartData()
  }
}, 500/*ms*/)

// subscribe
const unlisten = listen<TaskOpMap>('update:tasks_ops', (evt) => {
  onPayload(evt.payload)
})

onBeforeUnmount(() => {
  unlisten.then(fn => fn())
  chart?.destroy()
})
</script>


<template>
  <v-card elevation="2">
    <v-card-text>
      <h1 class="mb-4 font-weight-bold">Operations Timeline</h1>

      <v-skeleton-loader v-if="!loaded" type="image, paragraph" />

      <div v-else>
        <canvas ref="canvasRef" class="timeline-canvas" />

        <v-row align="center" class="mt-4">
          <v-col cols="10">
            <v-range-slider v-model="sliderRange" :min="dataMin" :max="dataMax" :step="stepSize" hide-details dense
              thumb-label>
              <template #append>
                <span>
                  {{ formatTime(sliderRange[0]) }}
                  —
                  {{ formatTime(sliderRange[1]) }}
                </span>
              </template>
            </v-range-slider>
          </v-col>
          <v-col cols="2" class="text-center">
            <v-btn color="primary" small @click="resetView">
              Auto-fit
            </v-btn>
          </v-col>
        </v-row>
      </div>
    </v-card-text>
  </v-card>
</template>

<style scoped>
.timeline-canvas {
  width: 100% !important;
  height: 400px;
}
</style>
