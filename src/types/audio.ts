export interface AudioDevice {
  id: string
  name: string
  device_type: string
  channels: number
  sample_rates: number[]
  buffer_sizes: number[]
  is_exclusive: boolean
}

export interface AudioStats {
  cpu_usage: number
  xruns: number
  latency: number
  sample_rate: number
  actual_buffer_size: number
  dsp_load: number
}
