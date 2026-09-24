<script setup lang="ts">
/** 指标迷你折线图（echarts 按需封装）：用于总览与实例详情资源曲线。 */

import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import * as echarts from 'echarts/core';
import { LineChart } from 'echarts/charts';
import { GridComponent } from 'echarts/components';
import { CanvasRenderer } from 'echarts/renderers';

echarts.use([LineChart, GridComponent, CanvasRenderer]);

const props = withDefaults(
  defineProps<{
    /** 0..1 序列，最新值在末尾。 */
    data: number[];
    color?: string;
    height?: number;
  }>(),
  { color: '#4c9ffe', height: 64 },
);

const container = ref<HTMLDivElement | null>(null);
let chart: echarts.ECharts | null = null;
let observer: ResizeObserver | null = null;

function render(): void {
  if (!chart) return;
  chart.setOption({
    animation: false,
    grid: { left: 0, right: 0, top: 2, bottom: 2 },
    xAxis: { type: 'category', show: false, data: props.data.map((_, i) => i) },
    yAxis: { type: 'value', show: false, min: 0, max: 1 },
    series: [
      {
        type: 'line',
        data: props.data,
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

watch(() => props.data, render, { deep: true });

onBeforeUnmount(() => {
  observer?.disconnect();
  chart?.dispose();
});
</script>

<template>
  <div ref="container" :style="{ width: '100%', height: `${height}px` }" />
</template>
