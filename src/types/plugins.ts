export interface PluginInfo {
  id: string
  name: string
  vendor: string
  category: string
  version: string
  path: string
  plugin_format: string
  has_editor: boolean
  input_bus_count: number
  output_bus_count: number
}

export interface PluginParameter {
  id: string
  name: string
  value: number
  min: number
  max: number
  default_value: number
  steps: number
  unit: string
  is_bypass: boolean
  is_automated: boolean
}
