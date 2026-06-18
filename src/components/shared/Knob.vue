<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'

const props = withDefaults(defineProps<{
  modelValue: number
  min?: number
  max?: number
  defaultValue?: number
  label?: string
  size?: number
  color?: string
}>(), {
  min: 0,
  max: 1,
  defaultValue: 0.5,
  label: '',
  size: 48,
  color: '#00ff88',
})

const emit = defineEmits<{
  'update:modelValue': [value: number]
}>()

const knobRef = ref<SVGSVGElement | null>(null)
const isDragging = ref(false)

const normalized = computed(() => {
  const range = props.max - props.min
  if (range === 0) return 0
  return (props.modelValue - props.min) / range
})

const angle = computed(() => -135 + normalized.value * 270)

const displayValue = computed(() => {
  if (props.max <= 1 && props.min >= 0) {
    return (props.modelValue * 100).toFixed(0)
  }
  return props.modelValue.toFixed(1)
})

const r = computed(() => (props.size - 8) / 2)
const cx = computed(() => props.size / 2)
const cy = computed(() => props.size / 2)
const strokeWidth = computed(() => Math.max(3, props.size / 14))

function polarToCartesian(centerX: number, centerY: number, radius: number, angleDeg: number) {
  const rad = ((angleDeg - 90) * Math.PI) / 180
  return {
    x: centerX + radius * Math.cos(rad),
    y: centerY + radius * Math.sin(rad),
  }
}

function describeArc(startAngle: number, endAngle: number) {
  const start = polarToCartesian(cx.value, cy.value, r.value, endAngle)
  const end = polarToCartesian(cx.value, cy.value, r.value, startAngle)
  const largeArc = endAngle - startAngle > 180 ? 1 : 0
  return `M ${start.x} ${start.y} A ${r.value} ${r.value} 0 ${largeArc} 0 ${end.x} ${end.y}`
}

const bgArc = computed(() => describeArc(-135, 135))
const fgArc = computed(() => {
  const endAngle = -135 + normalized.value * 270
  if (endAngle <= -135) return ''
  return describeArc(-135, Math.min(endAngle, 135))
})

const indicatorPos = computed(() => {
  return polarToCartesian(cx.value, cy.value, r.value, angle.value)
})

function handlePointerDown(e: PointerEvent) {
  isDragging.value = true
  e.preventDefault()
  ;(e.target as Element).setPointerCapture(e.pointerId)
  handlePointerMove(e)
}

function handlePointerMove(e: PointerEvent) {
  if (!isDragging.value || !knobRef.value) return
  const rect = knobRef.value.getBoundingClientRect()
  const centerX = rect.left + rect.width / 2
  const centerY = rect.top + rect.height / 2
  const dx = e.clientX - centerX
  const dy = e.clientY - centerY
  let deg = Math.atan2(dx, -dy) * (180 / Math.PI)
  deg = Math.max(-135, Math.min(135, deg))
  const norm = (deg + 135) / 270
  const val = props.min + norm * (props.max - props.min)
  emit('update:modelValue', Math.round(val * 1000) / 1000)
}

function handlePointerUp() {
  isDragging.value = false
}

function onDblClick() {
  emit('update:modelValue', props.defaultValue)
}

onMounted(() => {
  window.addEventListener('pointermove', handlePointerMove)
  window.addEventListener('pointerup', handlePointerUp)
})

onUnmounted(() => {
  window.removeEventListener('pointermove', handlePointerMove)
  window.removeEventListener('pointerup', handlePointerUp)
})
</script>

<template>
  <div class="knob-wrapper flex flex-col items-center gap-1" @dblclick="onDblClick">
    <svg
      ref="knobRef"
      :width="size"
      :height="size"
      class="cursor-pointer"
      @pointerdown="handlePointerDown"
    >
      <path :d="bgArc" fill="none" stroke="#2a2a30" :stroke-width="strokeWidth" stroke-linecap="round" />
      <path
        v-if="fgArc"
        :d="fgArc"
        fill="none"
        :stroke="color"
        :stroke-width="strokeWidth"
        stroke-linecap="round"
        :opacity="isDragging ? 1 : 0.85"
      />
      <circle
        :cx="indicatorPos.x"
        :cy="indicatorPos.y"
        :r="strokeWidth + 1"
        :fill="color"
        :opacity="isDragging ? 1 : 0.7"
      />
      <circle
        :cx="cx"
        :cy="cy"
        :r="r * 0.55"
        fill="#1a1a1e"
        stroke="#2a2a30"
        :stroke-width="1"
      />
      <text
        :x="cx"
        :y="cy + 4"
        text-anchor="middle"
        fill="#e0e0e0"
        font-size="10"
        font-family="JetBrains Mono, Consolas, monospace"
      >
        {{ displayValue }}
      </text>
    </svg>
    <span v-if="label" class="text-[9px] text-daw-muted tracking-wider uppercase">
      {{ label }}
    </span>
  </div>
</template>
