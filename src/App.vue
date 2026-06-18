<script setup lang="ts">
import { ref, shallowRef } from 'vue'
import MixerView from './components/mixer/MixerView.vue'
import PatchbayView from './components/patchbay/PatchbayView.vue'
import SettingsView from './components/settings/SettingsView.vue'

type Tab = 'mixer' | 'patchbay' | 'plugins' | 'settings'

const activeTab = ref<Tab>('mixer')

const tabs: { id: Tab; label: string; icon: string }[] = [
  { id: 'mixer', label: '调音台', icon: '🎛' },
  { id: 'patchbay', label: '跳线板', icon: '🔌' },
  { id: 'plugins', label: '插件', icon: '🎵' },
  { id: 'settings', label: '设置', icon: '⚙' },
]
</script>

<template>
  <div class="app-root flex h-screen w-screen overflow-hidden bg-daw-bg">
    <nav class="sidebar flex flex-col w-12 bg-daw-panel border-r border-[#2a2a30] py-2 gap-1">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        class="sidebar-btn flex flex-col items-center justify-center py-2 transition-colors relative"
        :class="activeTab === tab.id
          ? 'text-daw-green bg-daw-green/5'
          : 'text-daw-muted hover:text-daw-text hover:bg-[#2a2a30]'"
        @click="activeTab = tab.id"
      >
        <span v-if="activeTab === tab.id" class="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-6 bg-daw-green rounded-r" />
        <span class="text-base leading-none">{{ tab.icon }}</span>
        <span class="text-[8px] mt-1 tracking-tight">{{ tab.label }}</span>
      </button>
    </nav>

    <main class="flex-1 overflow-hidden">
      <MixerView v-if="activeTab === 'mixer'" />
      <PatchbayView v-else-if="activeTab === 'patchbay'" />
      <div v-else-if="activeTab === 'plugins'" class="flex items-center justify-center h-full text-daw-muted text-sm font-mono">
        插件管理器 (即将推出)
      </div>
      <SettingsView v-else-if="activeTab === 'settings'" />
    </main>
  </div>
</template>
