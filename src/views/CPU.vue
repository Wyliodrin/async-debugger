<script lang="ts" setup>
import {computed, nextTick, onBeforeUnmount, ref, watch} from 'vue'
import {listen} from '@tauri-apps/api/event'
import throttle from 'lodash.throttle'
import {
  BarController,
  BarElement,
  CategoryScale,
  Chart,
  type ChartOptions,
  Legend,
  type LegendItem,
  LinearScale,
  TimeScale,
  Title,
  Tooltip
} from 'chart.js'
import zoomPlugin from 'chartjs-plugin-zoom'
import datalabelsPlugin from 'chartjs-plugin-datalabels'
import 'chartjs-adapter-date-fns'
import type {TaskOpMap, TimeStamp} from '@/types/async_ops'
import {useDataStore} from '@/stores/data'

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

function toMillis(ts: TimeStamp | null): number {
  if (!ts) return 0
  const t = Date.parse(ts)
  return Number.isNaN(t) ? 0 : t
}

function stripHtml(s: string | null | undefined) {
  return (s ?? '').replace(/<[^>]*>/g, '')
}

interface OpDatum {
  x: [number, number]
  y: string
  label: string
  backgroundColor: string
  borderColor: string
  borderWidth: number
  realStart?: number
  realEnd?: number
  isGap?: boolean
  realGapMs?: number
  visualWidth?: number
  opName?: string
  location?: string | null
  pid?: number | null
}

const loaded = ref(false)
const rawOps = ref<TaskOpMap>({})
const canvasRef = ref<HTMLCanvasElement>()
let chart: Chart<'bar'> | null = null

const tasks = computed(() => {
  return Object.entries(rawOps.value).map(([key, taskOp], idx) => ({
    displayName:
        taskOp.task_name && taskOp.task_name.trim().length > 0
            ? taskOp.task_name
            : key || String(idx),
    operations: taskOp.operations
        .filter(o => o.started_at && o.stopped_at)
        .map(o => ({
          start: toMillis(o.started_at!),
          end: toMillis(o.stopped_at!),
          type: o.resource_target ?? 'unknown',
          name: o.resource_target ?? undefined,
          location: o.location ?? undefined,
          pid: o.pid ?? undefined
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

const GAP_THRESHOLD = 705
const COMPRESSED_GAP_MS = 1000

const userZoomPercent = ref(100)
const minUserZoomPercent = 100
const maxUserZoomPercent = 30000

const userGapThreshold = ref(GAP_THRESHOLD)
const userCompressedGapMs = ref(COMPRESSED_GAP_MS)
let mappingSegments: Array<{ displayStart: number, displayEnd: number, realStart: number, realEnd: number }> = []

// user-configurable controls
const zoomEditValue = ref(String(userZoomPercent.value))
const zoomStep = 10

function clampZoomPct(p: number) {
  return Math.min(Math.max(Math.round(p), minUserZoomPercent), maxUserZoomPercent)
}

watch(userZoomPercent, v => {
  zoomEditValue.value = String(v)
})

function applyZoomFromEdit() {
  const parsed = Number(zoomEditValue.value)
  if (!isFinite(parsed)) {
    zoomEditValue.value = String(userZoomPercent.value)
    return
  }
  const pct = clampZoomPct(parsed)
  userZoomPercent.value = pct
  zoomEditValue.value = String(pct)
  resetView()
}

function changeZoomBy(deltaPct: number) {
  const next = clampZoomPct((userZoomPercent.value || 100) + deltaPct)
  userZoomPercent.value = next
  zoomEditValue.value = String(next)
  resetView()
}

function buildLabelsAndData() {
  const data = buildDataset()
  const validTasks = new Set<string>()
  data.forEach((d: any) => {
    if (!d.isGap) validTasks.add(d.y)
  })
  const labels = Array.from(validTasks)
  const filtered = data.filter((d: any) => labels.includes(d.y))
  return {labels, data: filtered}
}

function adjustCanvasHeight(visibleRows: number) {
  const base = 28
  const padding = 40
  const min = 60
  const max = 800
  const h = Math.min(max, Math.max(min, visibleRows * base + padding))
  if (canvasRef.value) {
    (canvasRef.value as HTMLCanvasElement).style.height = `${h}px`
    chart?.resize()
  }
}

function computeZoomPercentFromRange(range: [number, number]): number {
  if (!mappingSegments.length) {
    const fullSpan = Math.max(1, dataMax.value - dataMin.value)
    const curSpan = Math.max(1, range[1] - range[0])
    return Math.round((fullSpan / curSpan) * 100)
  }
  const fullStart = mappingSegments[0].displayStart
  const fullEnd = mappingSegments[mappingSegments.length - 1].displayEnd
  const fullSpan = Math.max(1, fullEnd - fullStart)
  const curSpan = Math.max(1, range[1] - range[0])
  return Math.round((fullSpan / curSpan) * 100)
}

const zoomControl = ref(userZoomPercent.value)
let ignoreZoomControlWatcher = false

function buildDataset(): OpDatum[] {
  const items: any[] = []
  mappingSegments = []

  function realToDisplay(realMs: number): number {
    for (const s of mappingSegments) {
      if (realMs >= s.realStart && realMs <= s.realEnd) {
        const frac = (realMs - s.realStart) / Math.max(1, (s.realEnd - s.realStart))
        return s.displayStart + frac * (s.displayEnd - s.displayStart)
      }
    }
    // interpolation for finding the closest point
    let best: { display: number, delta: number } | null = null
    for (const seg of mappingSegments) {
      const cands = [
        {display: seg.displayStart, delta: Math.abs(realMs - seg.realStart)},
        {display: seg.displayEnd, delta: Math.abs(realMs - seg.realEnd)}
      ]
      for (const c of cands) {
        if (!best || c.delta < best.delta) best = c
      }
    }
    // fallback: if no mappingSegments yet, map linearly across dataMin/dataMax
    if (!best) {
      const overallDisplayStart = dataMin.value
      const overallDisplayEnd = dataMax.value === dataMin.value ? dataMin.value + 1 : dataMax.value
      const overallRealStart = dataMin.value
      const overallRealEnd = dataMax.value === dataMin.value ? dataMin.value + 1 : dataMax.value
      if (overallRealEnd === overallRealStart) return overallDisplayStart
      const frac = (realMs - overallRealStart) / (overallRealEnd - overallRealStart)
      return overallDisplayStart + frac * (overallDisplayEnd - overallDisplayStart)
    }
    return best.display
  }

  function realDurationToDisplay(realStart: number, realEnd: number): number {
    const d1 = realToDisplay(realStart)
    const d2 = realToDisplay(realEnd)
    return Math.max(1, Math.round(Math.abs(d2 - d1)))
  }

  for (const task of tasks.value) {
    const ops = task.operations.slice().sort((a, b) => a.start - b.start)
    if (ops.length === 0) continue

    let displayCursor = 0
    for (let i = 0; i < ops.length; i++) {
      const op = ops[i]
      if (i === 0) {
        displayCursor = Math.max(0, op.start - dataMin.value)
      } else {
        const prev = ops[i - 1]
        const gapMs = op.start - prev.end
        if (gapMs >= userGapThreshold.value) {
          const desiredDisplayWidth = realDurationToDisplay(prev.end, op.start)
          const visualWidth = Math.min(desiredDisplayWidth, userCompressedGapMs.value)

          const gapDS = displayCursor
          const gapDE = gapDS + visualWidth

          items.push({
            x: [gapDS + dataMin.value, gapDE + dataMin.value],
            y: task.displayName,
            label: `${(gapMs / 1000).toFixed(gapMs < 1000 ? 2 : 0)}s idle`,
            backgroundColor: 'rgba(200,200,200,0.7)',
            borderColor: 'rgba(160,160,160,1)',
            borderWidth: 0,
            isGap: true,
            realGapMs: gapMs,
            realStart: prev.end,
            realEnd: op.start,
            visualWidth: gapDE - gapDS,
            location: undefined,
            pid: undefined
          })

          mappingSegments.push({
            displayStart: gapDS + dataMin.value,
            displayEnd: gapDE + dataMin.value,
            realStart: prev.end,
            realEnd: op.start
          })

          displayCursor = gapDE
        } else {
          displayCursor += gapMs
          mappingSegments.push({
            displayStart: (displayCursor - gapMs) + dataMin.value,
            displayEnd: displayCursor + dataMin.value,
            realStart: prev.end,
            realEnd: op.start
          })
        }
      }

      const dur = Math.max(1, op.end - op.start)
      const ds = displayCursor
      const de = ds + dur
      const {bg, border} = typeColors.value[op.type] ?? {bg: 'rgba(100,100,100,0.5)', border: 'rgb(80,80,80)'}

      items.push({
        x: [ds + dataMin.value, de + dataMin.value],
        y: task.displayName,
        label: `${op.type.charAt(0) || '#'}${i + 1}`,
        backgroundColor: bg,
        borderColor: border,
        borderWidth: 1,
        isGap: false,
        realStart: op.start,
        realEnd: op.end,
        visualWidth: de - ds,
        opName: op.name,
        location: op.location ?? undefined,
        pid: op.pid ?? undefined
      })

      mappingSegments.push({
        displayStart: ds + dataMin.value,
        displayEnd: de + dataMin.value,
        realStart: op.start,
        realEnd: op.end
      })

      displayCursor = de
    }
  }
  return items as OpDatum[]
}

function displayToReal(displayX: number): number {
  for (const s of mappingSegments) {
    if (displayX >= s.displayStart && displayX <= s.displayEnd) {
      const frac = (displayX - s.displayStart) / Math.max(1, (s.displayEnd - s.displayStart))
      return Math.round(s.realStart + frac * (s.realEnd - s.realStart))
    }
  }
  // fallback: map linearly across overall range
  const overallDisplayStart = mappingSegments.length ? mappingSegments[0].displayStart : dataMin.value
  const overallDisplayEnd = mappingSegments.length ? mappingSegments[mappingSegments.length - 1].displayEnd : dataMax.value
  const overallRealStart = dataMin.value
  const overallRealEnd = dataMax.value
  if (overallDisplayEnd === overallDisplayStart) return overallRealStart
  const frac = (displayX - overallDisplayStart) / (overallDisplayEnd - overallDisplayStart)
  return Math.round(overallRealStart + frac * (overallRealEnd - overallRealStart))
}

const chartOptions = computed<ChartOptions<'bar'>>(() => ({
  indexAxis: 'y',
  responsive: true,
  scales: {
    x: {
      type: 'linear',
      min: sliderRange.value[0],
      max: sliderRange.value[1],
      title: {display: true, text: 'Time'},
      ticks: {
        callback: (val: any) => {
          const num = Number(val)
          if (isNaN(num)) return ''
          const real = displayToReal(num)
          return new Date(real).toLocaleTimeString()
        }
      }
    },
    y: {
      type: 'category',
      title: {display: true, text: 'Task'},
      labels: tasks.value.map(t => t.displayName)
    }
  },
  plugins: {
    legend: {
      display: true,
      position: 'bottom',
      labels: {
        generateLabels: () => {
          return Object.entries(typeColors.value).map(([text, {bg, border}]) => ({
            text,
            fillStyle: bg,
            strokeStyle: border,
            lineWidth: 1
          })) as LegendItem[]
        }
      }
    },
    tooltip: {
      enabled: true,
      displayColors: false,
      callbacks: {
        title(tooltipItems) {
          const item = tooltipItems[0]
          const raw = item && item.dataset && (item.dataset.data as any[])[item.dataIndex] as any
          return raw?.opName ?? raw?.label ?? 'operation'
        },
        label(tooltipItem) {
          const raw = (tooltipItem.dataset.data as any[])[tooltipItem.dataIndex] as any
          if (raw.isGap) {
            const ms = raw.realGapMs ?? Math.max(1, Math.round((raw.realEnd ?? 0) - (raw.realStart ?? 0)))
            return `no activity — ${ms} ms`
          }

          const rs = raw.realStart
          const re = raw.realEnd
          let startMs: number, endMs: number
          if (typeof rs === 'number' && typeof re === 'number') {
            startMs = rs
            endMs = re
          } else {
            const [ds, de] = raw.x as [number, number]
            startMs = displayToReal(ds)
            endMs = displayToReal(de)
          }

          const startStr = new Date(startMs).toLocaleTimeString()
          const durMs = Math.max(1, endMs - startMs)
          const durStr = durMs >= 1000 ? `${(durMs / 1000).toFixed(durMs >= 1000 ? 0 : 2)}s` : `${durMs} ms`

          const namePart = raw.opName ?? raw.label ?? ''
          const locPart = raw.location ? stripHtml(raw.location).trim() : 'unknown';
          const pidPart = raw.pid != null ? `PID: ${raw.pid}` : 'PID: N/A'

          return [
            `Name: ${namePart}`,
            `Location: ${locPart}`,
            `Start: ${startStr}`,
            `Duration: ${durStr}`,
            pidPart
          ]
        }
      }
    },
    zoom: {
      pan: {enabled: true, mode: 'x'},
      zoom: {wheel: {enabled: true}, pinch: {enabled: true}, mode: 'x'}
    },
    datalabels: {
      color: '#000', font: {size: 10, weight: 'bold'},
      anchor: 'center', align: 'center',
      formatter: (v: any) => {
        const raw = v as any
        return raw.isGap && (raw.visualWidth ?? 0) < 40 ? '' : raw.label
      }
    }
  }
}))

function initChart() {
  if (!canvasRef.value || chart) return
  const {labels, data} = buildLabelsAndData()
  console.debug('chart labels', labels);
  console.debug('chart data sample', data.slice(0,10));
  chart = new Chart(canvasRef.value, {
    type: 'bar',
    data: {
      labels,
      datasets: [
        {
          label: 'Operations',
          data,
          backgroundColor: (ctx: any) => {
            return (ctx.dataset.data[ctx.dataIndex] as any).backgroundColor
          },
          borderColor: (ctx: any) => (ctx.dataset.data[ctx.dataIndex] as any).borderColor,
          borderWidth: (ctx: any) => (ctx.dataset.data[ctx.dataIndex] as any).borderWidth ?? 1,
          barPercentage: 1,
          categoryPercentage: 1
        }
      ]
    },
    options: chartOptions.value
  })
  const allData = (chart.data.datasets[0]!.data as any[]).map(d => d.x).flat()
  if (allData.length) {
    const minDisplay = Math.min(...allData)
    const maxDisplay = Math.max(...allData)
    // base auto-fit range
    const fullRange: [number, number] = [minDisplay, maxDisplay]

    const datasetItems = (chart.data.datasets[0]!.data as any[]) as OpDatum[]
    let totalDur = 0
    let totalWeight = 0
    for (const d of datasetItems) {
      if (d.isGap) continue
      const rs = d.realStart ?? null
      const re = d.realEnd ?? null
      const dur = (typeof rs === 'number' && typeof re === 'number') ? Math.max(1, re - rs) : Math.max(1, (d.x as [number, number])[1] - (d.x as [number, number])[0])
      totalDur += dur * dur // weight by duration
      totalWeight += dur
    }
    const avgDur = totalWeight === 0 ? 1 : Math.max(1, Math.round(totalDur / totalWeight))

    // desired visible span = 4 * avgDur but not less than 0.1% the total range
    const desiredSpan = Math.max(Math.round(4 * avgDur), Math.round((fullRange[1] - fullRange[0]) * 0.001))

    const pct = Math.min(Math.max(userZoomPercent.value, minUserZoomPercent), maxUserZoomPercent) / 100
    const center = (fullRange[0] + fullRange[1]) / 2

    let halfSpan: number
    if (pct !== 1) {
      halfSpan = (fullRange[1] - fullRange[0]) / 2 / pct
    } else {
      halfSpan = Math.round(desiredSpan / 2)
    }
    const start = Math.round(center - halfSpan)
    const end = Math.round(center + halfSpan)
    sliderRange.value = [start, end]

    // sync zoom control to this computed range
    const computedPct = computeZoomPercentFromRange(sliderRange.value)
    ignoreZoomControlWatcher = true
    zoomControl.value = computedPct
    userZoomPercent.value = computedPct
    ignoreZoomControlWatcher = false
  } else {
    sliderRange.value = [dataMin.value, dataMax.value]
    const computedPct = computeZoomPercentFromRange(sliderRange.value)
    ignoreZoomControlWatcher = true
    zoomControl.value = computedPct
    userZoomPercent.value = computedPct
    ignoreZoomControlWatcher = false
  }

  adjustCanvasHeight(labels.length)
}


function updateChart() {
  if (!chart) return
  const {labels, data} = buildLabelsAndData()
  chart.data.labels = labels
  chart.data.datasets[0]!.data = data
  chart.options.scales!['x']!.min = sliderRange.value[0]
  chart.options.scales!['x']!.max = sliderRange.value[1]
  chart.update('none')
  adjustCanvasHeight(labels.length)
}

watch(sliderRange, ([min, max]) => {
  fromInput.value = formatTimeDisplay(min)
  toInput.value = formatTimeDisplay(max)
}, {deep: true})

watch(userGapThreshold, v => {
  userGapThreshold.value = Math.max(0, Math.round(Number(v ?? 0)))
}, {immediate: true})

watch(userCompressedGapMs, v => {
  userCompressedGapMs.value = Math.max(1, Math.round(Number(v ?? 1)))
}, {immediate: true})

watch([userGapThreshold, userCompressedGapMs], () => {
  if (loaded.value) updateChart()
})

watch(zoomControl, (v) => {
  if (ignoreZoomControlWatcher) return
  userZoomPercent.value = Math.min(Math.max(Number(v || 100), minUserZoomPercent), maxUserZoomPercent)
  if (loaded.value) resetView()
})

watch(userZoomPercent, (v) => {
  if (loaded.value) resetView()
})


function resetView() {
  if (mappingSegments.length) {
    const fullStart = mappingSegments[0].displayStart
    const fullEnd = mappingSegments[mappingSegments.length - 1].displayEnd
    const pct = Math.min(Math.max(userZoomPercent.value, minUserZoomPercent), maxUserZoomPercent) / 100
    const center = (fullStart + fullEnd) / 2
    const halfSpan = (fullEnd - fullStart) / 2 / pct
    sliderRange.value = [Math.round(center - halfSpan), Math.round(center + halfSpan)]
  } else {
    sliderRange.value = [dataMin.value, dataMax.value]
  }
  fromInput.value = formatTimeDisplay(sliderRange.value[0])
  toInput.value = formatTimeDisplay(sliderRange.value[1])

  // compute and sync zoom control
  const pct = computeZoomPercentFromRange(sliderRange.value)
  ignoreZoomControlWatcher = true
  zoomControl.value = pct
  userZoomPercent.value = pct
  ignoreZoomControlWatcher = false

  chart?.resetZoom()
}

function autoFit() {
  // set zoom to 100% (full span)
  const pct = 100
  userZoomPercent.value = pct

  // compute full-range based on mappingSegments or dataMin/dataMax
  const fullStart = mappingSegments.length ? mappingSegments[0].displayStart : dataMin.value
  const fullEnd = mappingSegments.length ? mappingSegments[mappingSegments.length - 1].displayEnd : dataMax.value
  const start = Math.round(fullStart)
  const end = Math.round(fullEnd)
  sliderRange.value = [start, end]

  fromInput.value = formatTimeDisplay(sliderRange.value[0])
  toInput.value = formatTimeDisplay(sliderRange.value[1])

  // update Chart.js scales directly to match sliderRange
  if (chart) {
    const scales = (chart.options.scales as any) || {}
    scales.x = scales.x || {}
    scales.x.min = sliderRange.value[0]
    scales.x.max = sliderRange.value[1]
    chart.options.scales = scales
    chart.update('none')
  }

  ignoreZoomControlWatcher = true
  zoomControl.value = pct
  ignoreZoomControlWatcher = false
}

const onPayload = throttle((payload: TaskOpMap) => {
  if (dataStore.pause) return

  rawOps.value = payload
  console.debug('rawOps payload', JSON.parse(JSON.stringify(rawOps.value)));
  if (!loaded.value) {
    loaded.value = true
    nextTick(initChart)
  } else {
    updateChart()
  }
}, 500)

// subscribe
const unlisten = listen<TaskOpMap>('update:tasks_ops', e => onPayload(e.payload))
onBeforeUnmount(async () => await unlisten.then(f => f()) && chart?.destroy())

const fromInput = ref('')
const toInput = ref('')

function formatTimeDisplay(iso: string): string {
  const dServer = new Date(iso)
  const displayMs = dServer.getTime()
  const real = displayToReal(displayMs)
  const d = new Date(real)

  const hh = String(d.getHours()).padStart(2, '0')
  const mm = String(d.getMinutes()).padStart(2, '0')
  const ss = String(d.getSeconds()).padStart(2, '0')
  const mss = String(d.getMilliseconds()).padStart(3, '0')
  return `${hh}:${mm}:${ss}.${mss}`
}


// convert to a real timestamp, then map to nearest display coordinate by searching mappingSegments.
function parseTimeOnBaseDisplay(baseMs: number, hhmmssSSS: string): number | null {
  const parts = hhmmssSSS.split(':')
  if (parts.length !== 3) return null

  const [hStr, mStr, sAndMs] = parts
  const [sStr, msStr = '0'] = sAndMs!.split('.')

  const h = Number(hStr)
  const m = Number(mStr)
  const s = Number(sStr)
  const ms = Number(msStr.padEnd(3, '0'))

  if (
      isNaN(h) || h < 0 || h > 23 ||
      isNaN(m) || m < 0 || m > 59 ||
      isNaN(s) || s < 0 || s > 59 ||
      isNaN(ms) || ms < 0 || ms > 999
  ) {
    return null
  }

  const base = new Date(baseMs)
  const out = new Date(base.getTime())
  out.setHours(h, m, s, ms)
  const realMs = out.getTime()

  let best: { display: number, delta: number } | null = null
  for (const seg of mappingSegments) {
    if (realMs >= seg.realStart && realMs <= seg.realEnd) {
      // interpolate
      const frac = (realMs - seg.realStart) / Math.max(1, (seg.realEnd - seg.realStart))
      const display = seg.displayStart + frac * (seg.displayEnd - seg.displayStart)
      return Math.round(display)
    } else {
      // if outside, pick nearest endpoint
      const d1 = Math.abs(realMs - seg.realStart)
      const d2 = Math.abs(realMs - seg.realEnd)
      const cand1 = {display: seg.displayStart, delta: d1}
      const cand2 = {display: seg.displayEnd, delta: d2}
      for (const c of [cand1, cand2]) {
        if (!best || c.delta < best.delta) best = c
      }
    }
  }
  return best ? Math.round(best.display) : null
}

function onFromBlur() {
  const ms = parseTimeOnBaseDisplay(dataMin.value, fromInput.value)
  if (ms !== null) {
    sliderRange.value[0] = Math.min(Math.max(mappingSegments[0]?.displayStart ?? dataMin.value, ms), sliderRange.value[1])
  } else {
    fromInput.value = formatTimeDisplay(sliderRange.value[0])
  }
}

function onToBlur() {
  const ms = parseTimeOnBaseDisplay(dataMin.value, toInput.value)
  if (ms !== null) {
    sliderRange.value[1] = Math.max(Math.min(mappingSegments[mappingSegments.length - 1]?.displayEnd ?? dataMax.value, ms), sliderRange.value[0])
  } else {
    toInput.value = formatTimeDisplay(sliderRange.value[1])
  }
}

// keep inputs in sync when slider moves
watch(sliderRange, ([min, max]) => {
  fromInput.value = formatTimeDisplay(min)
  toInput.value = formatTimeDisplay(max)
}, {deep: true})

watch(sliderRange, () => {
  updateChart()
}, {deep: true})
</script>

<template>
  <v-card elevation="2">
    <v-card-text>
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:1rem;">
        <h1 style="margin:0">Operations Timeline</h1>
        <div style="display:flex;flex-direction:column;">
          <label style="font-size:0.85em">Mark inactivity if the gap >= (ms)</label>
          <input v-model.number="userGapThreshold" min="0" style="width:7.5em" type="number"/>
        </div>

        <div style="display:flex;flex-direction:column;">
          <label style="font-size:0.85em">Compress if the gap >= (ms)</label>
          <input v-model.number="userCompressedGapMs" min="1" style="width:7.5em" type="number"/>
        </div>
        <div style="display:flex;align-items:center;gap:6px;">
          <button style="padding:6px 8px" title="Zoom out" type="button" @click="changeZoomBy(-zoomStep)">−</button>

          <input v-model="zoomEditValue"
                 style="width:4.5em;text-align:center;padding:6px"
                 type="text"
                 @blur="applyZoomFromEdit"
                 @keyup.enter.prevent="applyZoomFromEdit"/>

          <span style="font-size:0.9em">%</span>

          <button style="padding:6px 8px" title="Zoom in" type="button" @click="changeZoomBy(zoomStep)">+</button>
        </div>
      </div>

      <v-skeleton-loader v-if="!loaded" type="image, paragraph"/>

      <div v-else>
        <canvas ref="canvasRef" class="timeline-canvas"></canvas>

        <v-row align="center" class="mt-4">
          <v-col cols="10">
            <v-range-slider v-model="sliderRange"
                            :max="mappingSegments.length ? mappingSegments[mappingSegments.length - 1].displayEnd : dataMax"
                            :min="mappingSegments.length ? mappingSegments[0].displayStart : dataMin"
                            :step="stepSize" dense hide-details
                            thumb-label>
              <template #append>
                <div style="display: flex; align-items: center; gap: 4px;">
                  <input v-model="fromInput" :style="{ width: '7.5em', fontSize: '0.9em' }" type="text"
                         @blur="onFromBlur"
                         @keyup.enter.prevent="onFromBlur"/>
                  —
                  <input v-model="toInput" :style="{ width: '7.5em', fontSize: '0.9em' }" type="text" @blur="onToBlur"
                         @keyup.enter.prevent="onToBlur"/>
                </div>
              </template>
            </v-range-slider>
          </v-col>

          <v-col class="text-center" cols="2">
            <v-btn color="primary" small @click="autoFit">
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