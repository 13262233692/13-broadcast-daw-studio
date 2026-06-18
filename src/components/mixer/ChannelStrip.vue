<script setup lang="ts">
import { computed } from 'vue'
import Fader from '../shared/Fader.vue'
import Knob from '../shared/Knob.vue'
import Meter from '../shared/Meter.vue'

const props = withDefaults(defineProps<{
  name: string
  volume: number
  pan: number
  gain: number
  muted: boolean
  soloed: boolean
  peak: number
  rms: number
  color?: string
}>(), {
  color: '#00ff88',
})

const emit = defineEmits<{
  'update:volume': [value: number]
  'update:pan': [value: number]
  'update:gain': [value: number]
  'update:muted': [value: boolean]
  'update:soloed': [value: boolean]
}>()

const panLabel = computed(() => {
  if (props.pan === 0.5) return 'C'
  if (props.pan < 0.5) return `L${Math.round((0.5 - props.pan) * 2 * 50)}`
  return `R${Math.round((props.pan - 0.5) * 2 * 50)}`
})
</script>

<template>
  <div class="channel-strip flex flex-col items-center w-20 bg-daw-panel-deep border border-[#2a2a30] rounded py-2 px-1 gap-1">
    <div class="channel-name text-[10px] font-mono text-daw-text truncate w-full text-center px-1 py-0.5 rounded"
      :style="{ borderBottom: `2px solid ${color}` }"
    >
      {{ name }}
    </div>

    <div class="flex gap-1 items-end flex-1">
      <Meter :peak="peak" :rms="rms" :width="8" :height="160" />
      <Fader :model-value="volume" @update:model-value="emit('update:volume', $event)" :height="160" />
    </div>

    <Knob
      :model-value="pan"
      @update:model-value="emit('update:pan', $event)"
      :min="0"
      :max="1"
      :default-value="0.5"
      :label="'Pan'"
      :size="36"
      color="#00a8ff"
    />

    <div class="flex gap-1 mt-1">
      <button
        class="w-8 h-6 rounded text-[9px] font-mono font-bold tracking-wide transition-colors"
        :class="muted
          ? 'bg-daw-red text-white shadow-[0_0_6px_rgba(255,59,59,0.4)]'
          : 'bg-[#2a2a30] text-daw-muted hover:bg-[#333]'"
        @click="emit('update:muted', !muted)"
      >
        M
      </button>
      <button
        class="w-8 h-6 rounded text-[9px] font-mono font-bold tracking-wide transition-colors"
        :class="soloed
          ? 'bg-daw-solo text-black shadow-[0_0_6px_rgba(255,204,0,0.4)]'
          : 'bg-[#2a2a30] text-daw-muted hover:bg-[#333]'"
        @click="emit('update:soloed', !soloed)"
      >
        S
      </button>
    </div>

    <Knob
      :model-value="gain"
      @update:model-value="emit('update:gain', $event)"
      :min="0"
      :max="2"
      :default-value="1"
      :label="'Gain'"
      :size="36"
      color="#ff3b3b"
    />
  </div>
</template>
