import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { DAGNode, Connection, PatchbayState } from '@/types/dag'
import { useAudioEngine } from '@/composables/useAudioEngine'

export const usePatchbayStore = defineStore('patchbay', () => {
  const nodes = ref<DAGNode[]>([])
  const connections = ref<Connection[]>([])
  const selectedNodeIds = ref<Set<string>>(new Set())
  const selectedConnectionIds = ref<Set<string>>(new Set())

  const selectedNodes = computed(() =>
    nodes.value.filter((n) => selectedNodeIds.value.has(n.id)),
  )

  const selectedConnections = computed(() =>
    connections.value.filter((c) => selectedConnectionIds.value.has(c.id)),
  )

  const engine = useAudioEngine()

  const fetchPatchbay = async () => {
    try {
      const state: PatchbayState = await engine.getPatchbay()
      nodes.value = state.nodes
      connections.value = state.connections
    } catch (e) {
      console.error('Failed to fetch patchbay:', e)
    }
  }

  const addNode = async (
    nodeType: string,
    name: string,
    x: number,
    y: number,
    config: Record<string, any> = {},
  ) => {
    try {
      const nodeId = await engine.addNode({ nodeType, name, x, y, config })
      await fetchPatchbay()
      return nodeId
    } catch (e) {
      console.error('Failed to add node:', e)
    }
  }

  const removeNode = async (nodeId: string) => {
    try {
      await engine.removeNode(nodeId)
      selectedNodeIds.value.delete(nodeId)
      await fetchPatchbay()
    } catch (e) {
      console.error('Failed to remove node:', e)
    }
  }

  const connectNodes = async (
    sourceNodeId: string,
    sourcePortId: string,
    targetNodeId: string,
    targetPortId: string,
    gain: number = 1.0,
  ) => {
    try {
      const connectionId = await engine.connectNodes({
        sourceNodeId,
        sourcePortId,
        targetNodeId,
        targetPortId,
        gain,
      })
      await fetchPatchbay()
      return connectionId
    } catch (e) {
      console.error('Failed to connect nodes:', e)
    }
  }

  const disconnectNodes = async (connectionId: string) => {
    try {
      await engine.disconnectNodes(connectionId)
      selectedConnectionIds.value.delete(connectionId)
      await fetchPatchbay()
    } catch (e) {
      console.error('Failed to disconnect nodes:', e)
    }
  }

  const setNodeBypass = async (nodeId: string, bypassed: boolean) => {
    try {
      await engine.setNodeBypass(nodeId, bypassed)
      const node = nodes.value.find((n) => n.id === nodeId)
      if (node) node.bypassed = bypassed
    } catch (e) {
      console.error('Failed to set node bypass:', e)
    }
  }

  const selectNode = (nodeId: string, multi: boolean = false) => {
    if (multi) {
      if (selectedNodeIds.value.has(nodeId)) {
        selectedNodeIds.value.delete(nodeId)
      } else {
        selectedNodeIds.value.add(nodeId)
      }
    } else {
      selectedNodeIds.value.clear()
      selectedConnectionIds.value.clear()
      selectedNodeIds.value.add(nodeId)
    }
  }

  const selectConnection = (connectionId: string, multi: boolean = false) => {
    if (multi) {
      if (selectedConnectionIds.value.has(connectionId)) {
        selectedConnectionIds.value.delete(connectionId)
      } else {
        selectedConnectionIds.value.add(connectionId)
      }
    } else {
      selectedNodeIds.value.clear()
      selectedConnectionIds.value.clear()
      selectedConnectionIds.value.add(connectionId)
    }
  }

  const clearSelection = () => {
    selectedNodeIds.value.clear()
    selectedConnectionIds.value.clear()
  }

  return {
    nodes,
    connections,
    selectedNodeIds,
    selectedConnectionIds,
    selectedNodes,
    selectedConnections,
    fetchPatchbay,
    addNode,
    removeNode,
    connectNodes,
    disconnectNodes,
    setNodeBypass,
    selectNode,
    selectConnection,
    clearSelection,
  }
})
