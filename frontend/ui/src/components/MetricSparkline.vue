<script setup lang="ts">
/**
 * 指标迷你折线图（echarts 按需封装）：固定时间窗口折线。
 * 输入为带时间戳的点序列；窗口内无数据的采样桶补 0，保证曲线如实反映时间轴。
 */

import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import * as echarts from 'echarts/core';
import { LineChart } from 'echarts/charts';
import { GridComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([LineChart, GridComponent, CanvasRenderer]);

export interface MetricPoint {
  at: number;
  /** 0..1 */
  value: number;
}

const props = withDefaults(
  defineProps<{
    points: MetricPoint[];
    /** 时间窗口长度（毫秒）。 */
    windowMs?: number;
    /** 采样桶宽度（毫秒）。 */
    bucketMs?: number;
    color?: string;
    height?: number;
  }>(),
  { windowMs: 120_000, bucketMs: 2_000, color: '#4c9ffe', height: 64 },
);

// 桶位时钟：即使没有新数据，窗口也要随时间前移
const now = ref(Date.now());
const clock = setInterval(() => {
  now.value = Date.now();
}, props.bucketMs);
onBeforeUnmount(() => clearInterval(clock));

/** 按时间窗口聚合为固定长度序列；无数据的桶补 0。 */
const series = computed(() => {
  const bucketCount = Math.max(1, Math.ceil(props.windowMs / props.bucketMs));
  const end = Math.floor(now.value / props.bucketMs) * props.bucketMs;
  const start = end - props.windowMs;
  const buckets = new Array<number>(bucketCount).fill(0);
  for (const point of props.points) {
    if (point.at < start) continue;
    const idx = Math.min(bucketCount - 1, Math.floor((point.at - start) / props.bucketMs));
    buckets[idx] = point.value; // 同桶取最新
  }
  return buckets;
});

const container = ref<HTMLDivElement | null>(null);
let chart: echarts.ECharts | null = null;
let observer: ResizeObserver | null = null;

function render(): void {
  if (!chart) return;
  chart.setOption({
    animation: false,
    grid: { left: 0, right: 0, top: 2, bottom: 2 },
    xAxis: { type: 'category', show: false, data: series.value.map((_, i) => i) },
    yAxis: { type: 'value', show: false, min: 0, max: 1 },
    series: [
      {
        type: 'line',
        data: series.value,
        showSymbol: false,
        smooth: true,
        lineStyle: { color: props.color, width: 1.5 },
        areaStyle: { color: props.color, opacity: 0.15 },
      },
    ],
  });
}

onMounted(() => {
  if (!container.value) return;
  chart = echarts.init(container.value);
  render();
  observer = new ResizeObserver(() => chart?.resize());
  observer.observe(container.value);
});

watch(series, render);

onBeforeUnmount(() => {
  observer?.disconnect();
  chart?.dispose();
});
</script>

<template>
  <div ref="container" :style="{ width: '100%', height: `${height}px` }" />
</template>
