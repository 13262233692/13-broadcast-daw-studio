<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'

const props = withDefaults(defineProps<{
  peak: number
  rms: number
  width?: number
  height?: number
}>(), {
  width: 10,
  height: 200,
})

const canvasRef = ref<HTMLCanvasElement | null>(null)
let animFrame = 0
let peakHold = -Infinity
let peakDecay = 0

function ampToDb(amp: number): number {
  if (amp <= 0) return -Infinity
  return 20 * Math.log10(amp)
}

function dbToNorm(db: number): number {
  const minDb = -60
  if (db === -Infinity) return 0
  return Math.max(0, Math.min(1, (db - minDb) / (0 - minDb)))
}

function getColor(db: number): string {
  if (db > -3) return '#ff3b3b'
  if (db > -12) return '#ffcc00'
  return '#00ff88'
}

function draw() {
  const canvas = canvasRef.value
  if (!canvas) return
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  const dpr = window.devicePixelRatio || 1
  const w = props.width * dpr
  const h = props.height * dpr
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w
    canvas.height = h
  }
  ctx.scale(dpr, dpr)

  ctx.clearRect(0, 0, props.width, props.height)

  ctx.fillStyle = '#0d0d11'
  ctx.fillRect(0, 0, props.width, props.height)

  const segGap = 1
  const segH = 2
  const totalSegs = Math.floor(props.height / (segH + segGap))

  const peakDb = ampToDb(props.peak)
  const rmsDb = ampToDb(props.rms)

  const peakNorm = dbToNorm(peakDb)
  const rmsNorm = dbToNorm(rmsDb)

  if (peakDb > peakHold) {
    peakHold = peakDb
    peakDecay = 0
  } else {
    peakDecay++
    if (peakDecay > 120) {
      peakHold -= (peakDecay - 120) * 0.05
    }
  }
  if (peakHold < -60) peakHold = -60

  const peakHoldNorm = dbToNorm(peakHold)

  for (let i = 0; i < totalSegs; i++) {
    const norm = 1 - i / totalSegs
    const y = i * (segH + segGap)
    const db = -60 + norm * 60

    const segColor = getColor(db)

    if (norm <= rmsNorm) {
      ctx.fillStyle = segColor
      ctx.globalAlpha = 1
    } else if (norm <= peakNorm) {
      ctx.fillStyle = segColor
      ctx.globalAlpha = 0.35
    } else {
      ctx.fillStyle = '#1a1a1e'
      ctx.globalAlpha = 0.4
    }
    ctx.fillRect(0, y, props.width, segH)
  }

  ctx.globalAlpha = 1
  const peakHoldY = (1 - peakHoldNorm) * props.height
  ctx.fillStyle = peakHold > -3 ? '#ff3b3b' : peakHold > -12 ? '#ffcc00' : '#00ff88'
  ctx.fillRect(0, peakHoldY - 1, props.width, 2)

  animFrame = requestAnimationFrame(draw)
}

watch(() => [props.peak, props.rms], () => {})

onMounted(() => {
  animFrame = requestAnimationFrame(draw)
})

onUnmounted(() => {
  cancelAnimationFrame(animFrame)
})
</script>

<template>
  <div class="meter-wrapper">
    <canvas
      ref="canvasRef"
      :style="{ width: `${width}px`, height: `${height}px` }"
      class="rounded-sm"
    />
  </div>
</template>

<style scoped>
.meter-wrapper {
  display: inline-block;
  border: 1px solid #2a2a30;
  border-radius: 2px;
  overflow: hidden;
}
</style>
