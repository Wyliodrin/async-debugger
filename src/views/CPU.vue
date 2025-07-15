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
import type { TimeStamp, TaskOpMap } from '@/types/async_ops'
import { useDataStore } from '@/stores/data'

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

const dataStore = useDataStore()
function toMillis(ts: TimeStamp | null) {
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

// derive tasks/time bounds exactly as you had them
const tasks = computed(() => {
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

function updateChart() {
  if (!chart) return

  // swap in the new data
  chart.data.datasets[0].data = buildDataset()
  chart.options.scales!['x']!.min = sliderRange.value[0]
  chart.options.scales!['x']!.max = sliderRange.value[1]
  chart.update('none')
}

watch(sliderRange, ([min, max]) => {
  fromInput.value = formatTime(min);
  toInput.value = formatTime(max);
}, { deep: true });


function resetView() {
  sliderRange.value = [dataMin.value, dataMax.value]
  fromInput.value = formatTime(dataMin.value);
  toInput.value = formatTime(dataMax.value);
  chart?.resetZoom()
}

const onPayload = throttle((payload: TaskOpMap) => {
  // if the user has paused the updates, do nothing
  if (dataStore.pause) return
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
    updateChart()
  }
}, 500)
// subscribe
const unlisten = listen<TaskOpMap>('update:tasks_ops', e => onPayload(e.payload))
onBeforeUnmount(async () => await unlisten.then(f => f()) && chart?.destroy())

const fromInput = ref(formatISO(dataMin.value))
const toInput = ref(formatISO(dataMax.value))

function formatISO(ms: number) {
  return new Date(ms).toISOString().slice(11, 19)
}

function parseTimeOnBase(baseMs: number, hhmmss: string): number | null {
  const [h, m, s] = hhmmss.split(':').map(x => Number(x));
  if (
    hhmmss.length !== 8 ||
    isNaN(h!) || h! < 0 || h! > 23 ||
    isNaN(m!) || m! < 0 || m! > 59 ||
    isNaN(s!) || s! < 0 || s! > 59
  ) {
    return null;
  }

  const base = new Date(baseMs);
  const out = new Date(base.getTime());
  out.setHours(h!, m, s, 0);
  return out.getTime();
}

function formatTime(ms: number): string {
  const d = new Date(ms);
  // pad to always have 2 digits
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  const ss = String(d.getSeconds()).padStart(2, '0');
  return `${hh}:${mm}:${ss}`;
}



function onFromBlur() {
  const ms = parseTimeOnBase(dataMin.value, fromInput.value);
  if (ms !== null) {
    sliderRange.value[0] = Math.min(Math.max(dataMin.value, ms), sliderRange.value[1]);
  } else {
    fromInput.value = formatTime(sliderRange.value[0]);
  }
}


function onToBlur() {
  const ms = parseTimeOnBase(dataMin.value, toInput.value);
  if (ms !== null) {
    sliderRange.value[1] = Math.max(Math.min(dataMax.value, ms), sliderRange.value[0]);
  } else {
    toInput.value = formatTime(sliderRange.value[1]);
  }
}

// keep inputs in sync when slider moves
watch(sliderRange, ([min, max]) => {
  fromInput.value = formatTime(min)
  toInput.value = formatTime(max)
}, { deep: true })

watch(sliderRange, () => {
  updateChart()
}, { deep: true })

</script>


<template>
  <v-card elevation="2">
    <v-card-text>
      <h1 class="mb-4">Operations Timeline</h1>

      <v-skeleton-loader v-if="!loaded" type="image, paragraph" />

      <div v-else>
        <canvas ref="canvasRef" class="timeline-canvas"></canvas>

        <v-row align="center" class="mt-4">
          <v-col cols="10">
            <v-range-slider v-model="sliderRange" :min="dataMin" :max="dataMax" :step="stepSize" hide-details dense
              thumb-label>
              <template #append>
                <div style="display: flex; align-items: center; gap: 4px;">
                  <input type="text" v-model="fromInput" @blur="onFromBlur" @keyup.enter.prevent="onFromBlur"
                    :style="{ width: '5em', fontSize: '0.9em' }" />
                  —
                  <input type="text" v-model="toInput" @blur="onToBlur" @keyup.enter.prevent="onToBlur"
                    :style="{ width: '5em', fontSize: '0.9em' }" />
                </div>
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
  height: 300px;
}
</style>
