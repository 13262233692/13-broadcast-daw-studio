<script setup lang="ts">
import { ref, reactive } from 'vue'

const settings = reactive({
  audioDevice: 'default',
  sampleRate: '48000',
  bufferSize: '256',
  exclusiveMode: false,
})

const devices = [
  { id: 'default', name: '系统默认' },
  { id: 'wasapi', name: 'WASAPI 共享模式' },
  { id: 'wasapi_excl', name: 'WASAPI 独占模式' },
  { id: 'asio', name: 'ASIO' },
]

const perfMetrics = reactive({
  cpuLoad: 12,
  dspLoad: 8,
  latency: 5.3,
  bufferUsage: 34,
  xruns: 0,
  uptime: '00:12:34',
})
</script>

<template>
  <div class="settings-view flex flex-col h-full bg-daw-bg overflow-y-auto">
    <div class="px-6 py-4 border-b border-[#2a2a30]">
      <h2 class="text-sm font-mono text-daw-text tracking-wider uppercase">音频设置</h2>
    </div>

    <div class="p-6 space-y-6 max-w-2xl">
      <section class="space-y-4">
        <h3 class="text-xs font-mono text-daw-blue uppercase tracking-wider border-b border-[#2a2a30] pb-2">
          音频设备
        </h3>

        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <label class="text-xs text-daw-muted">音频设备</label>
            <select
              v-model="settings.audioDevice"
              class="w-56 bg-daw-panel-deep text-daw-text text-xs font-mono border border-[#2a2a30] rounded px-2 py-1.5"
            >
              <option v-for="d in devices" :key="d.id" :value="d.id">{{ d.name }}</option>
            </select>
          </div>

          <div class="flex items-center justify-between">
            <label class="text-xs text-daw-muted">采样率</label>
            <select
              v-model="settings.sampleRate"
              class="w-56 bg-daw-panel-deep text-daw-text text-xs font-mono border border-[#2a2a30] rounded px-2 py-1.5"
            >
              <option value="44100">44,100 Hz</option>
              <option value="48000">48,000 Hz</option>
              <option value="88200">88,200 Hz</option>
              <option value="96000">96,000 Hz</option>
              <option value="192000">192,000 Hz</option>
            </select>
          </div>

          <div class="flex items-center justify-between">
            <label class="text-xs text-daw-muted">缓冲区大小</label>
            <select
              v-model="settings.bufferSize"
              class="w-56 bg-daw-panel-deep text-daw-text text-xs font-mono border border-[#2a2a30] rounded px-2 py-1.5"
            >
              <option value="32">32 samples (0.7ms)</option>
              <option value="64">64 samples (1.3ms)</option>
              <option value="128">128 samples (2.7ms)</option>
              <option value="256">256 samples (5.3ms)</option>
              <option value="512">512 samples (10.7ms)</option>
              <option value="1024">1024 samples (21.3ms)</option>
              <option value="2048">2048 samples (42.7ms)</option>
            </select>
          </div>

          <div class="flex items-center justify-between">
            <label class="text-xs text-daw-muted">独占模式</label>
            <button
              class="w-10 h-5 rounded-full transition-colors relative"
              :class="settings.exclusiveMode ? 'bg-daw-blue' : 'bg-[#2a2a30]'"
              @click="settings.exclusiveMode = !settings.exclusiveMode"
            >
              <span
                class="absolute top-0.5 w-4 h-4 rounded-full bg-white transition-transform shadow"
                :class="settings.exclusiveMode ? 'translate-x-5' : 'translate-x-0.5'"
              />
            </button>
          </div>
        </div>
      </section>

      <section class="space-y-4">
        <h3 class="text-xs font-mono text-daw-blue uppercase tracking-wider border-b border-[#2a2a30] pb-2">
          性能监控
        </h3>

        <div class="bg-daw-panel-deep rounded border border-[#2a2a30] p-4 space-y-3">
          <div class="grid grid-cols-3 gap-4">
            <div class="text-center">
              <div class="text-2xl font-mono font-bold" :class="perfMetrics.cpuLoad > 80 ? 'text-daw-red' : 'text-daw-green'">
                {{ perfMetrics.cpuLoad }}%
              </div>
              <div class="text-[9px] text-daw-muted uppercase tracking-wider mt-1">CPU</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-mono font-bold" :class="perfMetrics.dspLoad > 80 ? 'text-daw-red' : 'text-daw-blue'">
                {{ perfMetrics.dspLoad }}%
              </div>
              <div class="text-[9px] text-daw-muted uppercase tracking-wider mt-1">DSP</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-mono font-bold text-daw-text">
                {{ perfMetrics.latency }}ms
              </div>
              <div class="text-[9px] text-daw-muted uppercase tracking-wider mt-1">延迟</div>
            </div>
          </div>

          <div class="space-y-2 pt-2 border-t border-[#2a2a30]">
            <div class="flex items-center justify-between text-[10px]">
              <span class="text-daw-muted">缓冲区使用率</span>
              <div class="flex items-center gap-2">
                <div class="w-32 h-1.5 bg-daw-bg rounded-full overflow-hidden">
                  <div
                    class="h-full rounded-full transition-all duration-500"
                    :style="{
                      width: `${perfMetrics.bufferUsage}%`,
                      background: perfMetrics.bufferUsage > 80 ? '#ff3b3b' : perfMetrics.bufferUsage > 50 ? '#ffcc00' : '#00ff88',
                    }"
                  />
                </div>
                <span class="w-8 text-right font-mono text-daw-muted">{{ perfMetrics.bufferUsage }}%</span>
              </div>
            </div>

            <div class="flex items-center justify-between text-[10px]">
              <span class="text-daw-muted">Xrun 计数</span>
              <span class="font-mono" :class="perfMetrics.xruns > 0 ? 'text-daw-red' : 'text-daw-muted'">{{ perfMetrics.xruns }}</span>
            </div>

            <div class="flex items-center justify-between text-[10px]">
              <span class="text-daw-muted">运行时间</span>
              <span class="font-mono text-daw-text">{{ perfMetrics.uptime }}</span>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
