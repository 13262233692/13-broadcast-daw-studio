import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { AudioStats, AudioDevice } from '@/types/audio'
import { useAudioEngine } from '@/composables/useAudioEngine'

export interface Channel {
  id: string
  name: string
  gain: number
  mute: boolean
  solo: boolean
  pan: number
  fader: number
  meterLevel: number
}

export interface AudioConfig {
  sampleRate: number
  bufferSize: number
  inputDeviceId: string
  outputDeviceId: string
}

function createDefaultChannels(): Channel[] {
  return Array.from({ length: 8 }, (_, i) => ({
    id: `ch-${i + 1}`,
    name: `Channel ${i + 1}`,
    gain: 1.0,
    mute: false,
    solo: false,
    pan: 0,
    fader: 0,
    meterLevel: -60,
  }))
}

export const useMixerStore = defineStore('mixer', () => {
  const channels = ref<Channel[]>(createDefaultChannels())
  const masterVolume = ref(1.0)
  const audioStats = ref<AudioStats>({
    cpu_usage: 0,
    xruns: 0,
    latency: 0,
    sample_rate: 48000,
    actual_buffer_size: 256,
    dsp_load: 0,
  })
  const audioConfig = ref<AudioConfig>({
    sampleRate: 48000,
    bufferSize: 256,
    inputDeviceId: '',
    outputDeviceId: '',
  })
  const audioDevices = ref<AudioDevice[]>([])
  const isEngineRunning = ref(false)

  const soloChannels = computed(() => channels.value.filter((ch) => ch.solo))
  const hasSolo = computed(() => soloChannels.value.length > 0)

  const isChannelAudible = (channelId: string) => {
    const ch = channels.value.find((c) => c.id === channelId)
    if (!ch) return false
    if (ch.mute) return false
    if (hasSolo.value && !ch.solo) return false
    return true
  }

  const engine = useAudioEngine()

  const setChannelGain = (channelId: string, gain: number) => {
    const ch = channels.value.find((c) => c.id === channelId)
    if (ch) ch.gain = gain
  }

  const setChannelMute = (channelId: string, mute: boolean) => {
    const ch = channels.value.find((c) => c.id === channelId)
    if (ch) ch.mute = mute
  }

  const setChannelSolo = (channelId: string, solo: boolean) => {
    const ch = channels.value.find((c) => c.id === channelId)
    if (ch) ch.solo = solo
  }

  const setChannelPan = (channelId: string, pan: number) => {
    const ch = channels.value.find((c) => c.id === channelId)
    if (ch) ch.pan = Math.max(-1, Math.min(1, pan))
  }

  const setChannelFader = (channelId: string, fader: number) => {
    const ch = channels.value.find((c) => c.id === channelId)
    if (ch) ch.fader = fader
  }

  const setChannelName = (channelId: string, name: string) => {
    const ch = channels.value.find((c) => c.id === channelId)
    if (ch) ch.name = name
  }

  const setMasterVolume = async (volume: number) => {
    masterVolume.value = volume
    await engine.setMasterVolume(volume)
  }

  const fetchAudioDevices = async () => {
    try {
      audioDevices.value = await engine.getAudioDevices()
    } catch (e) {
      console.error('Failed to fetch audio devices:', e)
    }
  }

  const startEngine = async () => {
    try {
      await engine.startEngine({
        inputDeviceId: audioConfig.value.inputDeviceId || undefined,
        outputDeviceId: audioConfig.value.outputDeviceId || undefined,
        sampleRate: audioConfig.value.sampleRate,
        bufferSize: audioConfig.value.bufferSize,
      })
      isEngineRunning.value = true
    } catch (e) {
      console.error('Failed to start audio engine:', e)
    }
  }

  const stopEngine = async () => {
    try {
      await engine.stopEngine()
      isEngineRunning.value = false
    } catch (e) {
      console.error('Failed to stop audio engine:', e)
    }
  }

  const fetchAudioStats = async () => {
    try {
      audioStats.value = await engine.getAudioStats()
    } catch (e) {
      console.error('Failed to fetch audio stats:', e)
    }
  }

  const updateAudioConfig = (config: Partial<AudioConfig>) => {
    Object.assign(audioConfig.value, config)
  }

  return {
    channels,
    masterVolume,
    audioStats,
    audioConfig,
    audioDevices,
    isEngineRunning,
    soloChannels,
    hasSolo,
    isChannelAudible,
    setChannelGain,
    setChannelMute,
    setChannelSolo,
    setChannelPan,
    setChannelFader,
    setChannelName,
    setMasterVolume,
    fetchAudioDevices,
    startEngine,
    stopEngine,
    fetchAudioStats,
    updateAudioConfig,
  }
})
