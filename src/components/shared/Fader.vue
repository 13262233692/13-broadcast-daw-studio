<script setup lang="ts">
import { computed, ref } from 'vue'

const props = withDefaults(defineProps<{
  modelValue: number
  height?: number
  disabled?: boolean
}>(), {
  height: 200,
  disabled: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: number]
}>()

const isDragging = ref(false)

const dbValue = computed(() => {
  if (props.modelValue <= 0) return -Infinity
  return 20 * Math.log10(props.modelValue)
})

const dbDisplay = computed(() => {
  const db = dbValue.value
  if (db === -Infinity) return '-∞'
  return db >= 0 ? `+${db.toFixed(1)}` : db.toFixed(1)
})

const UNITY_GAIN = 0.75

const percentValue = computed(() => props.modelValue * 100)

function onInput(e: Event) {
  const target = e.target as HTMLInputElement
  emit('update:modelValue', Number(target.value) / 100)
}

function onDblClick() {
  emit('update:modelValue', UNITY_GAIN)
}
</script>

<template>
  <div
    class="fader-wrapper flex flex-col items-center"
    :class="{ 'opacity-50 pointer-events-none': disabled }"
  >
    <div class="db-display text-[10px] font-mono mb-1 h-4" :style="{ color: dbValue > 0 ? '#ff3b3b' : dbValue > -12 ? '#ffcc00' : '#00ff88' }">
      {{ dbDisplay }}
    </div>
    <div
      class="fader-track relative rounded-sm"
      :style="{ height: `${height}px`, width: '28px' }"
      @dblclick="onDblClick"
    >
      <div
        class="fader-fill absolute bottom-0 left-0 right-0 rounded-sm transition-[height] duration-75"
        :style="{
          height: `${percentValue}%`,
          background: `linear-gradient(to top, #00ff88 ${70}%, #ffcc00 ${90}%, #ff3b3b 100%)`,
        }"
      />
      <div
        class="fader-zero-line absolute left-0 right-0 h-px"
        :style="{ bottom: `${UNITY_GAIN * 100}%`, background: 'rgba(0, 168, 255, 0.5)' }"
      />
      <input
        type="range"
        :value="percentValue"
        min="0"
        max="100"
        step="0.1"
        class="fader-input absolute inset-0 w-full h-full opacity-0 cursor-ns-resize"
        :style="{ writingMode: 'vertical-lr', direction: 'rtl' }"
        @input="onInput"
        @dblclick.stop="onDblClick"
        @mousedown="isDragging = true"
        @mouseup="isDragging = false"
      />
      <div
        class="fader-thumb absolute left-1/2 -translate-x-1/2 w-7 h-3 rounded-sm pointer-events-none transition-[bottom] duration-75"
        :style="{
          bottom: `calc(${percentValue}% - 6px)`,
          background: isDragging ? '#00ff88' : '#c0c0c0',
          boxShadow: isDragging ? '0 0 8px rgba(0, 255, 136, 0.5)' : '0 1px 3px rgba(0,0,0,0.5)',
        }"
      />
    </div>
  </div>
</template>

<style scoped>
.fader-track {
  background: #121217;
  border: 1px solid #2a2a30;
}

.fader-thumb {
  z-index: 2;
}

.fader-input {
  z-index: 3;
}
</style>
