export type NodeType = 'input' | 'output' | 'mix' | 'gain' | 'eq' | 'compressor' | 'limiter' | 'vst3'

export interface Port {
  id: string
  node_id: string
  port_type: 'Input' | 'Output'
  channel: number
  name: string
}

export interface DAGNode {
  id: string
  node_type: NodeType
  name: string
  position: { x: number; y: number }
  input_ports: Port[]
  output_ports: Port[]
  config: Record<string, any>
  enabled: boolean
  bypassed: boolean
}

export interface Connection {
  id: string
  source_port: string
  target_port: string
  source_node: string
  target_node: string
  gain: number
}

export interface PatchbayState {
  nodes: DAGNode[]
  connections: Connection[]
}
