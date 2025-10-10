<script lang="ts" setup>
import {onMounted, onUnmounted, ref, UnwrapRef} from 'vue';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';
import {listen} from '@tauri-apps/api/event';
import {useDataStore} from "@/stores/data.ts";
import {TimeStamp} from "@/types/async_ops";
import debounce from 'lodash.debounce';
import {wheelZoomPlugin} from "@/utils/WheelZoomUplotPlugin.ts";
import {sliderZoomPlugin} from "@/utils/SliderZoomUplotPlugin.ts";

export interface CPUOverview {
  started_at: TimeStamp | undefined;
  stopped_at: TimeStamp | undefined;
  resource_target: string | undefined;
  location: string | undefined;
  pid?: number | undefined;
}

export interface TaskOp {
  task_id: string;
  task_name: string;
  task_colour: string;
  operations: CPUOverview[];
}

export type TaskOpMap = Record<string, TaskOp>;

interface OpDatum {
  x: [number, number];
  y: number;
  label: string;
  backgroundColor: string;
  borderColor: string;
  borderWidth: number;
  taskId: string;
  resourceTarget?: string;
  location?: string;
  pid?: number;
}

const dataStore = useDataStore();
const chart = ref<uPlot | null>(null);
const plotContainer = ref<HTMLDivElement | null>(null);
const zoomSliderContainer = ref<HTMLDivElement | null>(null);

// Zoom-specific refs
const zoomLevel = ref(1);
const zoomMin = ref(0.1);
const zoomMax = ref(10);
const xMin = ref<number | null>(null);
const xMax = ref<number | null>(null);

function debugLog(...args: any[]) {
  console.log('[uPlot Debug]', ...args);
}

// Manage chart lifecycle
const unlistenRef: { fn: (() => void) | null } = {fn: null};

function adjustZoom(direction: 'in' | 'out') {
  if (!chart.value || !xMin.value || !xMax.value) return;

  const currentRange = xMax.value - xMin.value;
  const centerPoint = (xMin.value + xMax.value) / 2;

  let newZoomLevel: UnwrapRef<number>;
  if (direction === 'in') {
    newZoomLevel = Math.min(zoomMax.value, zoomLevel.value * 1.5);
  } else {
    newZoomLevel = Math.max(zoomMin.value, zoomLevel.value / 1.5);
  }

  const newHalfRange = (currentRange / zoomLevel.value) / (2 * newZoomLevel);

  const newMin = centerPoint - newHalfRange;
  const newMax = centerPoint + newHalfRange;

  chart.value.setScale('x', {
    min: newMin,
    max: newMax
  });

  zoomLevel.value = newZoomLevel;
}

function resetZoom() {
  if (!chart.value || !xMin.value || !xMax.value) return;

  chart.value.setScale('x', {
    min: xMin.value,
    max: xMax.value
  });
  zoomLevel.value = 1;
}

function createGanttChart(ops: OpDatum[]) {
  if (!plotContainer.value) {
    console.error('Plot container not available');
    return null;
  }

  let data: uPlot.AlignedData;
  try {
    data = prepareUplotData(ops);
  } catch (error) {
    console.error('Failed to prepare uPlot data:', error);
    return null;
  }

  if (!data || data.length < 2) {
    console.error('Insufficient data for uPlot');
    return null;
  }

  const minTime = Math.min(...ops.map(op => op.x[0]));
  const maxTime = Math.max(...ops.map(op => op.x[1]));

  // Create a set of unique resource types for color mapping
  const resourceTypes = [...new Set(ops.map(op => op.resourceTarget || 'Unknown'))];

  const uniqueYLevels = new Set(ops.map(op => op.label)).size;

  const plugins: uPlot.Plugin[] = [
    {
      hooks: {
        // Double-click to reset zoom
        dblclick: (u) => {
          const [dataMin, dataMax] = u.scales.x.range(u, null, null);
          u.setScale('x', {
            min: dataMin,
            max: dataMax
          });
        },
        setCursor: (u) => {
          const cursor = u.cursor;
          if (!cursor.idx) return;

          // Find the operation at the current cursor position
          const op = ops.find(o =>
              cursor.idx !== null &&
              cursor.idx >= o.x[0] &&
              cursor.idx <= o.x[1]
          );

          if (op) {
            const tooltipContent = `
              Task: ${op.label}
              Resource: ${op.resourceTarget || 'Unknown'}
              Location: ${op.location || 'N/A'}
              PID: ${op.pid || 'N/A'}
            `;

            let tooltip = document.getElementById('uplot-tooltip');
            if (!tooltip) {
              tooltip = document.createElement('div');
              tooltip.id = 'uplot-tooltip';
              tooltip.style.position = 'absolute';
              tooltip.style.background = 'rgba(0,0,0,0.7)';
              tooltip.style.color = 'white';
              tooltip.style.padding = '5px';
              tooltip.style.borderRadius = '3px';
              tooltip.style.pointerEvents = 'none';
              document.body.appendChild(tooltip);
            }

            tooltip.innerHTML = tooltipContent;
            tooltip.style.left = `${cursor.left}px`;
            tooltip.style.top = `${cursor.top}px`;
            tooltip.style.display = 'block';
          }
        },
      }
    },
    wheelZoomPlugin({factor: 0.95}),
    ...(zoomSliderContainer.value ? [sliderZoomPlugin({
      container: zoomSliderContainer.value,
      init: [minTime, maxTime],
      height: 30,
    })] : [])
  ];

  const opts: uPlot.Options = {
    width: plotContainer.value.clientWidth || 800,
    height: Math.min(
        Math.max(uniqueYLevels * 50 + 100, 300),  // Limit height based on unique tasks
        800
    ),
    title: 'Operations Timeline',
    scales: {
      x: {
        time: false,
        range: [minTime, maxTime]
      },
      y: {
        auto: false,
        range: [0, uniqueYLevels + 2]
      }
    },
    interaction: {
      cursor: {
        zoom: {
          x: true,
          y: false
        },
        drag: {
          x: true,
          y: false
        },
        sync: {
          key: 'zoom',
          setSeries: true
        },
        points: {
          show: true,
          size: 5,
          width: 2,
          stroke: 'red'
        }
      }
    },
    axes: [
      {
        scale: 'x',
        values: (u, vals) => vals.map(v => {
          const date = new Date(Number(v));
          return date.toLocaleTimeString('en-US', {
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            fractionalSecondDigits: 3
          });
        })
      },
      {
        scale: 'y',
        values: (u, vals) => {
          return vals.map(v => {
            const op = ops.find(o => o.y === v);
            return op ? op.label : '';
          });
        }
      }
    ],
    series: [
      {label: 'Time'},
      {label: 'Base'},
      ...resourceTypes.map((resourceType, idx) => ({
        label: resourceType,
        stroke: generateColor(resourceType),
        fill: generateColor(resourceType),
        width: 1,
        paths: uPlot.paths.bars!({
          size: [0.9, Infinity],
          gap: 2,
          align: 1
        }),
        points: {
          show: true,
          fill: generateColor(resourceType),
          stroke: 'black',
          size: 5
        }
      }))
    ],
    data: data,
    plugins: plugins
  };

  if (plotContainer.value) {
    plotContainer.value.style.maxHeight = '800px';
    plotContainer.value.style.overflowY = 'auto';
  }

  try {
    debugLog('Creating uPlot instance');
    return new uPlot(opts, data, plotContainer.value as unknown as HTMLElement);
  } catch (error) {
    console.error('uPlot Creation Error:', error);

    if (plotContainer.value) {
      plotContainer.value.innerHTML = `
        <div style="color: red; text-align: center;">
        Chart Rendering Error: ${error instanceof Error ? error.message : 'Unknown error'}
        </div>
      `;
    }

    return null;
  }
}

function prepareUplotData(ops: OpDatum[]): uPlot.AlignedData {
  const minTime = Math.min(...ops.map(op => op.x[0]));
  const maxTime = Math.max(...ops.map(op => op.x[1]));

  // Create x-axis with timestamps
  const xAxis = [minTime];
  ops.forEach(op => {
    xAxis.push(op.x[0], op.x[1]);
  });
  xAxis.push(maxTime);
  xAxis.sort((a, b) => a - b);

  // Remove duplicates while preserving order
  const uniqueXAxis = [...new Set(xAxis)];

  // Get unique resource types
  const resourceTypes = [...new Set(ops.map(op => op.resourceTarget || 'Unknown'))];

  const yAxis = new Array(uniqueXAxis.length).fill(0);
  const seriesData: number[][] = resourceTypes.map(resourceType => {
    return uniqueXAxis.map(ts => {
      const activeOps = ops.filter(op =>
          op.resourceTarget === resourceType &&
          ts >= op.x[0] &&
          ts <= op.x[1]
      );

      return activeOps.length > 0 ? activeOps[0].y : 0;
    });
  });

  return [
    uniqueXAxis,
    yAxis,
    ...seriesData
  ];
}

function transformOperations(taskOpMap: TaskOpMap): OpDatum[] {
  debugLog('Transforming Operations from:', taskOpMap);

  const transformedOps: OpDatum[] = [];
  const resourceGroups: Record<string, OpDatum[]> = {};

  Object.values(taskOpMap).forEach(taskOp => {
    const taskName = taskOp.task_name || `Task ${taskOp.task_id}`;

    taskOp.operations.forEach((op) => {
      if (!op.started_at || !op.stopped_at) {
        debugLog('Skipping operation due to missing timestamps:', op);
        return;
      }

      const startTime = new Date(op.started_at).getTime();
      const endTime = new Date(op.stopped_at).getTime();

      if (isNaN(startTime) || isNaN(endTime)) {
        debugLog('Invalid timestamp conversion:', op);
        return;
      }

      // Determine color based on resource target
      const resourceType = op.resource_target || 'Unknown';
      const backgroundColor = generateColor(resourceType);

      if (!resourceGroups[taskName]) {
        resourceGroups[taskName] = [];
      }

      resourceGroups[taskName].push({
        x: [startTime, endTime],
        y: 1, // Placeholder, will be updated later
        label: taskName,
        backgroundColor: backgroundColor,
        borderColor: 'black',
        borderWidth: 1,
        taskId: taskOp.task_id,
        resourceTarget: op.resource_target,
        location: op.location,
        pid: op.pid
      });
    });
  });

  let currentYLevel = 1;
  Object.entries(resourceGroups).forEach(([taskName, ops]) => {
    ops.sort((a, b) => a.x[0] - b.x[0]);

    const groupedOps = ops.map(op => ({
      ...op,
      y: currentYLevel,
      label: taskName
    }));

    transformedOps.push(...groupedOps);
    currentYLevel++;
  });

  debugLog('Transformed Operations Result:', transformedOps);
  return transformedOps;
}

function generateColor(seed: string): string {
  let hash = 0;
  for (let i = 0; i < seed.length; i++) {
    hash = ((hash << 5) - hash) + seed.charCodeAt(i);
    hash = hash & hash;
  }

  const h = Math.abs(hash) % 360; // Hue
  const s = 70 + (Math.abs(hash) % 30); // Saturation
  const l = 50 + (Math.abs(hash) % 20); // Lightness

  return `hsl(${h}, ${s}%, ${l}%)`;
}

onMounted(async () => {
  try {
    unlistenRef.fn = await listen<TaskOpMap>("update:tasks_ops", (e) => {
      if ((dataStore as any)?.pause || !e.payload) return;
      throttledProcessPayload(e.payload);
    });
  } catch (err) {
    console.error("Failed to set up listener:", err);
  }
});

onUnmounted(() => {
  if (chart.value) {
    chart.value.destroy();
  }

  if (unlistenRef.fn) {
    unlistenRef.fn();
  }
});

const throttledProcessPayload = debounce((payload: TaskOpMap) => {
  const transformedOps = transformOperations(payload);

  if (!chart.value && plotContainer.value) {
    chart.value = createGanttChart(transformedOps);
  } else if (chart.value) {
    updateChartData(chart.value, transformedOps);
  }
}, 200);

function updateChartData(chartInstance: uPlot, ops: OpDatum[]) {
  const newData = prepareUplotData(ops);

  try {
    chartInstance.setData(newData);
    chartInstance.redraw();
  } catch (error) {
    console.error('Chart update error:', error);
  }
}
</script>

<template>
  <div class="gantt-chart-container">
    <div class="zoom-controls">
      <div class="zoom-buttons">
        <button :disabled="zoomLevel <= zoomMin" @click="adjustZoom('out')">
          -
        </button>
        <span class="zoom-level">
          Zoom: {{ zoomLevel.toFixed(2) }}x
        </span>
        <button :disabled="zoomLevel >= zoomMax" @click="adjustZoom('in')">
          +
        </button>
        <button @click="resetZoom">
          Reset
        </button>
      </div>
    </div>
    <div
        ref="plotContainer"
        class="uplot-container"
    ></div>
  </div>
</template>

<style scoped>
.gantt-chart-container {
  width: 100%;
  max-width: 100%;
  background-color: #f5f5f5;
  border-radius: 8px;
  padding: 10px;
}

.zoom-controls {
  display: flex;
  justify-content: center;
  margin-bottom: 10px;
  align-items: center;
}

.zoom-buttons {
  display: flex;
  align-items: center;
  gap: 10px;
}

.zoom-buttons button {
  background-color: #f0f0f0;
  border: 1px solid #ddd;
  border-radius: 4px;
  padding: 5px 10px;
  cursor: pointer;
  transition: background-color 0.3s ease;
}

.zoom-buttons button:hover:not(:disabled) {
  background-color: #e0e0e0;
}

.zoom-buttons button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.zoom-level {
  font-size: 0.9rem;
  color: #333;
  min-width: 100px;
  text-align: center;
}

.uplot-container {
  width: 100%;
  height: 400px;
  overflow-x: auto;
}

@media (max-width: 768px) {
  .zoom-controls {
    flex-direction: column;
  }

  .zoom-buttons {
    flex-wrap: wrap;
    justify-content: center;
  }
}
</style>
