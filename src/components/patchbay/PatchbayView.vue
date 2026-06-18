<script setup lang="ts">
import { ref, onMounted, onUnmounted, reactive } from 'vue'

interface Port {
  id: string
  name: string
  type: 'input' | 'output'
  x: number
  y: number
}

interface Node {
  id: string
  type: string
  label: string
  x: number
  y: number
  width: number
  height: number
  ports: Port[]
  color: string
}

interface Connection {
  id: string
  fromNodeId: string
  fromPortId: string
  toNodeId: string
  toPortId: string
}

const canvasRef = ref<HTMLCanvasElement | null>(null)
let ctx: CanvasRenderingContext2D | null = null
let animFrame = 0
let dashOffset = 0

const NODE_TYPES: Record<string, { label: string; color: string; inputs: string[]; outputs: string[] }> = {
  Input: { label: 'Input', color: '#00a8ff', inputs: [], outputs: ['Out'] },
  Output: { label: 'Output', color: '#ff3b3b', inputs: ['In'], outputs: [] },
  EQ: { label: 'EQ', color: '#00ff88', inputs: ['In'], outputs: ['Out'] },
  Compressor: { label: 'Comp', color: '#ffcc00', inputs: ['In', 'SC'], outputs: ['Out'] },
  Gain: { label: 'Gain', color: '#00a8ff', inputs: ['In'], outputs: ['Out'] },
  Mix: { label: 'Mix', color: '#ff3b3b', inputs: ['A', 'B'], outputs: ['Out'] },
}

const nodes = reactive<Node[]>([
  createNode('Input', 80, 100),
  createNode('EQ', 300, 80),
  createNode('Compressor', 520, 100),
  createNode('Gain', 300, 280),
  createNode('Mix', 520, 280),
  createNode('Output', 740, 180),
])

const connections = reactive<Connection[]>([
  { id: 'c1', fromNodeId: nodes[0].id, fromPortId: nodes[0].ports[0].id, toNodeId: nodes[1].id, toPortId: nodes[1].ports[0].id },
  { id: 'c2', fromNodeId: nodes[1].id, fromPortId: nodes[1].ports[1].id, toNodeId: nodes[2].id, toPortId: nodes[2].ports[0].id },
  { id: 'c3', fromNodeId: nodes[2].id, fromPortId: nodes[2].ports[1].id, toNodeId: nodes[5].id, toPortId: nodes[5].ports[0].id },
])

let nodeIdCounter = 10

let draggingNodeId: string | null = null
let dragOffsetX = 0
let dragOffsetY = 0

let connectingFrom: { nodeId: string; portId: string; isOutput: boolean } | null = null
let mousePos = { x: 0, y: 0 }

function createNode(type: string, x: number, y: number): Node {
  const def = NODE_TYPES[type]
  const id = `node_${nodeIdCounter++}`
  const width = 120
  const headerH = 28
  const portSpacing = 22
  const maxPorts = Math.max(def.inputs.length, def.outputs.length, 1)
  const height = headerH + maxPorts * portSpacing + 10

  const ports: Port[] = []
  def.inputs.forEach((name, i) => {
    ports.push({
      id: `${id}_in_${name}`,
      name,
      type: 'input',
      x: 0,
      y: headerH + i * portSpacing + 14,
    })
  })
  def.outputs.forEach((name, i) => {
    ports.push({
      id: `${id}_out_${name}`,
      name,
      type: 'output',
      x: width,
      y: headerH + i * portSpacing + 14,
    })
  })

  return { id, type, label: def.label, x, y, width, height, ports, color: def.color }
}

function getPortWorldPos(node: Node, port: Port) {
  return {
    x: node.x + port.x,
    y: node.y + port.y,
  }
}

function drawGrid(w: number, h: number) {
  if (!ctx) return
  ctx.strokeStyle = '#1a1a1e'
  ctx.lineWidth = 1
  const gridSize = 20
  for (let x = 0; x < w; x += gridSize) {
    ctx.beginPath()
    ctx.moveTo(x + 0.5, 0)
    ctx.lineTo(x + 0.5, h)
    ctx.stroke()
  }
  for (let y = 0; y < h; y += gridSize) {
    ctx.beginPath()
    ctx.moveTo(0, y + 0.5)
    ctx.lineTo(w, y + 0.5)
    ctx.stroke()
  }
  ctx.strokeStyle = '#222228'
  const majorGrid = 100
  for (let x = 0; x < w; x += majorGrid) {
    ctx.beginPath()
    ctx.moveTo(x + 0.5, 0)
    ctx.lineTo(x + 0.5, h)
    ctx.stroke()
  }
  for (let y = 0; y < h; y += majorGrid) {
    ctx.beginPath()
    ctx.moveTo(0, y + 0.5)
    ctx.lineTo(w, y + 0.5)
    ctx.stroke()
  }
}

function drawConnection(fromPos: { x: number; y: number }, toPos: { x: number; y: number }, animated: boolean) {
  if (!ctx) return
  const dx = Math.abs(toPos.x - fromPos.x)
  const cpOffset = Math.max(dx * 0.5, 50)

  ctx.beginPath()
  ctx.moveTo(fromPos.x, fromPos.y)
  ctx.bezierCurveTo(
    fromPos.x + cpOffset, fromPos.y,
    toPos.x - cpOffset, toPos.y,
    toPos.x, toPos.y,
  )

  if (animated) {
    ctx.strokeStyle = '#00a8ff'
    ctx.lineWidth = 2
    ctx.setLineDash([8, 4])
    ctx.lineDashOffset = -dashOffset
  } else {
    ctx.strokeStyle = '#00a8ff88'
    ctx.lineWidth = 1.5
    ctx.setLineDash([])
  }
  ctx.stroke()
  ctx.setLineDash([])
}

function drawNode(node: Node) {
  if (!ctx) return

  ctx.fillStyle = '#1a1a1e'
  ctx.strokeStyle = '#2a2a30'
  ctx.lineWidth = 1
  const r = 4
  ctx.beginPath()
  ctx.moveTo(node.x + r, node.y)
  ctx.lineTo(node.x + node.width - r, node.y)
  ctx.quadraticCurveTo(node.x + node.width, node.y, node.x + node.width, node.y + r)
  ctx.lineTo(node.x + node.width, node.y + node.height - r)
  ctx.quadraticCurveTo(node.x + node.width, node.y + node.height, node.x + node.width - r, node.y + node.height)
  ctx.lineTo(node.x + r, node.y + node.height)
  ctx.quadraticCurveTo(node.x, node.y + node.height, node.x, node.y + node.height - r)
  ctx.lineTo(node.x, node.y + r)
  ctx.quadraticCurveTo(node.x, node.y, node.x + r, node.y)
  ctx.closePath()
  ctx.fill()
  ctx.stroke()

  ctx.fillStyle = node.color + '22'
  ctx.beginPath()
  ctx.moveTo(node.x + r, node.y)
  ctx.lineTo(node.x + node.width - r, node.y)
  ctx.quadraticCurveTo(node.x + node.width, node.y, node.x + node.width, node.y + r)
  ctx.lineTo(node.x + node.width, node.y + 26)
  ctx.lineTo(node.x, node.y + 26)
  ctx.lineTo(node.x, node.y + r)
  ctx.quadraticCurveTo(node.x, node.y, node.x + r, node.y)
  ctx.closePath()
  ctx.fill()

  ctx.strokeStyle = node.color + '66'
  ctx.beginPath()
  ctx.moveTo(node.x, node.y + 26)
  ctx.lineTo(node.x + node.width, node.y + 26)
  ctx.stroke()

  ctx.fillStyle = node.color
  ctx.font = '11px "JetBrains Mono", Consolas, monospace'
  ctx.textAlign = 'center'
  ctx.fillText(node.label, node.x + node.width / 2, node.y + 18)

  node.ports.forEach((port) => {
    const px = node.x + port.x
    const py = node.y + port.y

    ctx!.beginPath()
    ctx!.arc(px, py, 5, 0, Math.PI * 2)
    ctx!.fillStyle = port.type === 'input' ? '#00a8ff44' : '#00ff8844'
    ctx!.fill()
    ctx!.strokeStyle = port.type === 'input' ? '#00a8ff' : '#00ff88'
    ctx!.lineWidth = 1.5
    ctx!.stroke()

    ctx!.fillStyle = '#888'
    ctx!.font = '9px "JetBrains Mono", Consolas, monospace'
    ctx!.textAlign = port.type === 'input' ? 'left' : 'right'
    const textX = port.type === 'input' ? px + 10 : px - 10
    ctx!.fillText(port.name, textX, py + 3)
  })
}

function render() {
  const canvas = canvasRef.value
  if (!canvas || !ctx) return

  const dpr = window.devicePixelRatio || 1
  const rect = canvas.getBoundingClientRect()
  const w = rect.width
  const h = rect.height

  if (canvas.width !== Math.floor(w * dpr) || canvas.height !== Math.floor(h * dpr)) {
    canvas.width = Math.floor(w * dpr)
    canvas.height = Math.floor(h * dpr)
  }

  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, w, h)

  ctx.fillStyle = '#0d0d11'
  ctx.fillRect(0, 0, w, h)

  drawGrid(w, h)

  connections.forEach((conn) => {
    const fromNode = nodes.find((n) => n.id === conn.fromNodeId)
    const toNode = nodes.find((n) => n.id === conn.toNodeId)
    const fromPort = fromNode?.ports.find((p) => p.id === conn.fromPortId)
    const toPort = toNode?.ports.find((p) => p.id === conn.toPortId)
    if (fromNode && toNode && fromPort && toPort) {
      const from = getPortWorldPos(fromNode, fromPort)
      const to = getPortWorldPos(toNode, toPort)
      drawConnection(from, to, true)
    }
  })

  if (connectingFrom) {
    const fromNode = nodes.find((n) => n.id === connectingFrom.nodeId)
    const fromPort = fromNode?.ports.find((p) => p.id === connectingFrom.portId)
    if (fromNode && fromPort) {
      const from = getPortWorldPos(fromNode, fromPort)
      const to = connectingFrom.isOutput ? mousePos : mousePos
      drawConnection(
        connectingFrom.isOutput ? from : to,
        connectingFrom.isOutput ? to : from,
        false,
      )
    }
  }

  nodes.forEach(drawNode)

  dashOffset += 0.5
  if (dashOffset > 12) dashOffset = 0

  animFrame = requestAnimationFrame(render)
}

function getMousePos(e: MouseEvent) {
  const canvas = canvasRef.value
  if (!canvas) return { x: 0, y: 0 }
  const rect = canvas.getBoundingClientRect()
  return { x: e.clientX - rect.left, y: e.clientY - rect.top }
}

function hitTestNode(x: number, y: number): Node | null {
  for (let i = nodes.length - 1; i >= 0; i--) {
    const n = nodes[i]
    if (x >= n.x && x <= n.x + n.width && y >= n.y && y <= n.y + n.height) {
      return n
    }
  }
  return null
}

function hitTestPort(x: number, y: number, node: Node): Port | null {
  for (const port of node.ports) {
    const px = node.x + port.x
    const py = node.y + port.y
    const dist = Math.sqrt((x - px) ** 2 + (y - py) ** 2)
    if (dist <= 8) return port
  }
  return null
}

function onMouseDown(e: MouseEvent) {
  const pos = getMousePos(e)
  const hitNode = hitTestNode(pos.x, pos.y)
  if (!hitNode) return

  const hitPort = hitTestPort(pos.x, pos.y, hitNode)
  if (hitPort) {
    connectingFrom = {
      nodeId: hitNode.id,
      portId: hitPort.id,
      isOutput: hitPort.type === 'output',
    }
    return
  }

  draggingNodeId = hitNode.id
  dragOffsetX = pos.x - hitNode.x
  dragOffsetY = pos.y - hitNode.y
}

function onMouseMove(e: MouseEvent) {
  const pos = getMousePos(e)
  mousePos = pos

  if (draggingNodeId) {
    const node = nodes.find((n) => n.id === draggingNodeId)
    if (node) {
      node.x = pos.x - dragOffsetX
      node.y = pos.y - dragOffsetY
    }
  }
}

function onMouseUp(e: MouseEvent) {
  if (connectingFrom) {
    const pos = getMousePos(e)
    const hitNode = hitTestNode(pos.x, pos.y)
    if (hitNode) {
      const hitPort = hitTestPort(pos.x, pos.y, hitNode)
      if (hitPort && hitNode.id !== connectingFrom.nodeId) {
        const isOutputHit = hitPort.type === 'output'
        if (connectingFrom.isOutput !== isOutputHit) {
          const fromNodeId = connectingFrom.isOutput ? connectingFrom.nodeId : hitNode.id
          const fromPortId = connectingFrom.isOutput ? connectingFrom.portId : hitPort.id
          const toNodeId = connectingFrom.isOutput ? hitNode.id : connectingFrom.nodeId
          const toPortId = connectingFrom.isOutput ? hitPort.id : connectingFrom.portId

          const exists = connections.some(
            (c) => c.fromPortId === fromPortId && c.toPortId === toPortId,
          )
          if (!exists) {
            connections.push({
              id: `c_${Date.now()}`,
              fromNodeId,
              fromPortId,
              toNodeId,
              toPortId,
            })
          }
        }
      }
    }
    connectingFrom = null
  }
  draggingNodeId = null
}

function addNode(type: string) {
  const canvas = canvasRef.value
  const cx = canvas ? canvas.clientWidth / 2 - 60 : 300
  const cy = canvas ? canvas.clientHeight / 2 - 40 : 200
  nodes.push(createNode(type, cx + Math.random() * 60 - 30, cy + Math.random() * 60 - 30))
}

onMounted(() => {
  const canvas = canvasRef.value
  if (canvas) {
    ctx = canvas.getContext('2d')
    animFrame = requestAnimationFrame(render)
  }
})

onUnmounted(() => {
  cancelAnimationFrame(animFrame)
})
</script>

<template>
  <div class="patchbay-view flex flex-col h-full bg-daw-bg">
    <div class="toolbar flex items-center gap-2 px-4 py-2 bg-daw-panel border-b border-[#2a2a30]">
      <span class="text-[10px] text-daw-muted uppercase tracking-wider mr-2">Add Node:</span>
      <button
        v-for="(def, type) in NODE_TYPES"
        :key="type"
        class="px-2 py-1 rounded text-[10px] font-mono transition-colors border"
        :style="{ borderColor: def.color + '44', color: def.color }"
        @click="addNode(type)"
      >
        {{ type }}
      </button>
    </div>

    <div class="flex-1 relative overflow-hidden">
      <canvas
        ref="canvasRef"
        class="w-full h-full"
        @mousedown="onMouseDown"
        @mousemove="onMouseMove"
        @mouseup="onMouseUp"
        @mouseleave="draggingNodeId = null; connectingFrom = null"
      />
    </div>
  </div>
</template>
