<script setup lang="ts">
import { ref, reactive } from 'vue'
import ChannelStrip from './ChannelStrip.vue'
import MasterSection from './MasterSection.vue'

interface ChannelState {
  name: string
  volume: number
  pan: number
  gain: number
  muted: boolean
  soloed: boolean
  peak: number
  rms: number
  color: string
}

const engineRunning = ref(false)
const sampleRate = ref('48000')
const bufferSize = ref('256')

const channels = reactive<ChannelState[]>(
  Array.from({ length: 8 }, (_, i) => ({
    name: `CH ${i + 1}`,
    volume: 0.75,
    pan: 0.5,
    gain: 1.0,
    muted: false,
    soloed: false,
    peak: 0.3 + Math.random() * 0.3,
    rms: 0.15 + Math.random() * 0.15,
    color: ['#00ff88', '#00a8ff', '#ffcc00', '#ff3b3b', '#00ff88', '#00a8ff', '#ffcc00', '#ff3b3b'][i],
  }))
)

const master = reactive({
  volume: 0.75,
  peakL: 0.4 + Math.random() * 0.2,
  rmsL: 0.2 + Math.random() * 0.1,
  peakR: 0.35 + Math.random() * 0.2,
  rmsR: 0.18 + Math.random() * 0.1,
  cpuLoad: 12,
  latency: 5.3,
  dspLoad: 8,
})

function toggleEngine() {
  engineRunning.value = !engineRunning.value
}

function updateChannelProp(index: number, prop: keyof ChannelState, value: number | boolean) {
  ;(channels[index] as any)[prop] = value
}
</script>

<template>
  <div class="mixer-view flex flex-col h-full bg-daw-bg">
    <div class="toolbar flex items-center justify-between px-4 py-2 bg-daw-panel border-b border-[#2a2a30]">
      <div class="flex items-center gap-3">
        <button
          class="px-3 py-1 rounded text-xs font-mono font-bold tracking-wide transition-all"
          :class="engineRunning
            ? 'bg-daw-red text-white shadow-[0_0_10px_rgba(255,59,59,0.4)]'
            : 'bg-[#2a2a30] text-daw-muted hover:bg-[#333]'"
          @click="toggleEngine"
        >
          {{ engineRunning ? '■ STOP' : '▶ START' }}
        </button>

        <div class="h-4 w-px bg-[#2a2a30]" />

        <div class="flex items-center gap-2 text-[10px]">
          <label class="text-daw-muted">SR:</label>
          <select
            v-model="sampleRate"
            class="bg-daw-panel-deep text-daw-text border border-[#2a2a30] rounded px-1.5 py-0.5 text-[10px] font-mono"
          >
            <option value="44100">44.1k</option>
            <option value="48000">48k</option>
            <option value="96000">96k</option>
          </select>
        </div>

        <div class="flex items-center gap-2 text-[10px]">
          <label class="text-daw-muted">Buf:</label>
          <select
            v-model="bufferSize"
            class="bg-daw-panel-deep text-daw-text border border-[#2a2a30] rounded px-1.5 py-0.5 text-[10px] font-mono"
          >
            <option value="64">64</option>
            <option value="128">128</option>
            <option value="256">256</option>
            <option value="512">512</option>
            <option value="1024">1024</option>
          </select>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <div class="engine-indicator w-2 h-2 rounded-full" :class="engineRunning ? 'bg-daw-green shadow-[0_0_6px_#00ff88]' : 'bg-[#333]'" />
        <span class="text-[10px] font-mono text-daw-muted">
          {{ engineRunning ? 'RUNNING' : 'STOPPED' }}
        </span>
      </div>
    </div>

    <div class="flex-1 flex items-stretch overflow-x-auto">
      <div class="flex gap-px flex-1 p-2">
        <ChannelStrip
          v-for="(ch, i) in channels"
          :key="i"
          :name="ch.name"
          :volume="ch.volume"
          :pan="ch.pan"
          :gain="ch.gain"
          :muted="ch.muted"
          :soloed="ch.soloed"
          :peak="ch.peak"
          :rms="ch.rms"
          :color="ch.color"
          @update:volume="updateChannelProp(i, 'volume', $event)"
          @update:pan="updateChannelProp(i, 'pan', $event)"
          @update:gain="updateChannelProp(i, 'gain', $event)"
          @update:muted="updateChannelProp(i, 'muted', $event)"
          @update:soloed="updateChannelProp(i, 'soloed', $event)"
        />
      </div>

      <div class="p-2">
        <MasterSection
          :master-volume="master.volume"
          :peak-l="master.peakL"
          :rms-l="master.rmsL"
          :peak-r="master.peakR"
          :rms-r="master.rmsR"
          :cpu-load="master.cpuLoad"
          :latency="master.latency"
          :dsp-load="master.dspLoad"
          @update:master-volume="master.volume = $event"
        />
      </div>
    </div>

    <div class="status-bar flex items-center justify-between px-4 py-1.5 bg-daw-panel border-t border-[#2a2a30] text-[10px] font-mono">
      <div class="flex items-center gap-4">
        <span class="text-daw-muted">CPU: <span :class="master.cpuLoad > 80 ? 'text-daw-red' : 'text-daw-text'">{{ master.cpuLoad }}%</span></span>
        <span class="text-daw-muted">Latency: <span class="text-daw-text">{{ master.latency }}ms</span></span>
        <span class="text-daw-muted">Xruns: <span class="text-daw-text">0</span></span>
      </div>
      <div class="flex items-center gap-4">
        <span class="text-daw-muted">{{ sampleRate }}Hz / {{ bufferSize }}smp</span>
        <span class="text-daw-muted">Broadcast DAW Studio v0.1</span>
      </div>
    </div>
  </div>
</template>
