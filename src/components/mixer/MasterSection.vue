<script setup lang="ts">
import { ref } from 'vue'
import Fader from '../shared/Fader.vue'
import Meter from '../shared/Meter.vue'

const props = defineProps<{
  masterVolume: number
  peakL: number
  rmsL: number
  peakR: number
  rmsR: number
  cpuLoad: number
  latency: number
  dspLoad: number
}>()

const emit = defineEmits<{
  'update:masterVolume': [value: number]
}>()

const xrunCount = ref(0)
</script>

<template>
  <div class="master-section flex flex-col items-center w-28 bg-daw-panel border border-[#2a2a30] rounded py-3 px-2 gap-2">
    <div class="text-[10px] font-mono text-daw-blue uppercase tracking-widest border-b border-daw-blue/30 pb-1 w-full text-center">
      Master
    </div>

    <div class="flex gap-1 items-end">
      <div class="flex flex-col items-center gap-0.5">
        <span class="text-[8px] text-daw-muted">L</span>
        <Meter :peak="peakL" :rms="rmsL" :width="10" :height="180" />
      </div>
      <div class="flex flex-col items-center gap-0.5">
        <span class="text-[8px] text-daw-muted">R</span>
        <Meter :peak="peakR" :rms="rmsR" :width="10" :height="180" />
      </div>
      <Fader :model-value="masterVolume" @update:model-value="emit('update:masterVolume', $event)" :height="180" />
    </div>

    <div class="w-full border-t border-[#2a2a30] pt-2 space-y-1.5">
      <div class="flex justify-between items-center text-[9px]">
        <span class="text-daw-muted">CPU</span>
        <div class="flex items-center gap-1">
          <div class="w-14 h-1.5 bg-daw-panel-deep rounded-full overflow-hidden">
            <div
              class="h-full rounded-full transition-all duration-300"
              :style="{
                width: `${cpuLoad}%`,
                background: cpuLoad > 80 ? '#ff3b3b' : cpuLoad > 50 ? '#ffcc00' : '#00ff88',
              }"
            />
          </div>
          <span class="w-7 text-right font-mono" :class="cpuLoad > 80 ? 'text-daw-red' : 'text-daw-muted'">{{ cpuLoad }}%</span>
        </div>
      </div>

      <div class="flex justify-between items-center text-[9px]">
        <span class="text-daw-muted">DSP</span>
        <div class="flex items-center gap-1">
          <div class="w-14 h-1.5 bg-daw-panel-deep rounded-full overflow-hidden">
            <div
              class="h-full rounded-full transition-all duration-300"
              :style="{
                width: `${dspLoad}%`,
                background: dspLoad > 80 ? '#ff3b3b' : dspLoad > 50 ? '#ffcc00' : '#00a8ff',
              }"
            />
          </div>
          <span class="w-7 text-right font-mono text-daw-muted">{{ dspLoad }}%</span>
        </div>
      </div>

      <div class="flex justify-between items-center text-[9px]">
        <span class="text-daw-muted">Lat</span>
        <span class="font-mono text-daw-muted">{{ latency }}ms</span>
      </div>

      <div class="flex justify-between items-center text-[9px]">
        <span class="text-daw-muted">Xrun</span>
        <span class="font-mono" :class="xrunCount > 0 ? 'text-daw-red' : 'text-daw-muted'">{{ xrunCount }}</span>
      </div>
    </div>
  </div>
</template>
