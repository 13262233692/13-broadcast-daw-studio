import { invoke } from '@tauri-apps/api/core'
import type { AudioDevice, AudioStats } from '@/types/audio'
import type { PatchbayState } from '@/types/dag'
import type { PluginInfo } from '@/types/plugins'

export function useAudioEngine() {
  const getAudioDevices = () => invoke<AudioDevice[]>('get_audio_devices')

  const startEngine = (params: {
    inputDeviceId?: string
    outputDeviceId?: string
    sampleRate: number
    bufferSize: number
  }) =>
    invoke<void>('start_audio_engine', {
      inputDeviceId: params.inputDeviceId,
      outputDeviceId: params.outputDeviceId,
      sampleRate: params.sampleRate,
      bufferSize: params.bufferSize,
    })

  const stopEngine = () => invoke<void>('stop_audio_engine')

  const getAudioStats = () => invoke<AudioStats>('get_audio_stats')

  const getPatchbay = () => invoke<PatchbayState>('get_patchbay')

  const addNode = (params: {
    nodeType: string
    name: string
    x: number
    y: number
    config: Record<string, any>
  }) =>
    invoke<string>('add_node', {
      nodeType: params.nodeType,
      name: params.name,
      x: params.x,
      y: params.y,
      config: params.config,
    })

  const removeNode = (nodeId: string) =>
    invoke<void>('remove_node', { nodeId })

  const connectNodes = (params: {
    sourceNodeId: string
    sourcePortId: string
    targetNodeId: string
    targetPortId: string
    gain: number
  }) => invoke<string>('connect_nodes', params)

  const disconnectNodes = (connectionId: string) =>
    invoke<void>('disconnect_nodes', { connectionId })

  const setNodeBypass = (nodeId: string, bypassed: boolean) =>
    invoke<void>('set_node_bypass', { nodeId, bypassed })

  const scanPlugins = () => invoke<PluginInfo[]>('scan_vst3_plugins')

  const loadPlugin = (pluginId: string) =>
    invoke<string>('load_vst3_plugin', { pluginId })

  const setPluginParameter = (
    instanceId: string,
    paramId: string,
    value: number,
  ) => invoke<void>('set_vst3_parameter', { instanceId, paramId, value })

  const setMasterVolume = (volume: number) =>
    invoke<void>('set_master_volume', { volume })

  return {
    getAudioDevices,
    startEngine,
    stopEngine,
    getAudioStats,
    getPatchbay,
    addNode,
    removeNode,
    connectNodes,
    disconnectNodes,
    setNodeBypass,
    scanPlugins,
    loadPlugin,
    setPluginParameter,
    setMasterVolume,
  }
}
