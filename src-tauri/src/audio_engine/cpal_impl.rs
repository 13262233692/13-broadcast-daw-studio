use crate::shared::AudioDeviceInfo;
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AudioDevice {
    Input,
    Output,
}

pub struct CpalHost {
    host: cpal::Host,
}

impl std::fmt::Debug for CpalHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CpalHost").finish()
    }
}

impl CpalHost {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        Ok(Self { host })
    }

    pub fn scan_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        let mut devices = Vec::new();
        let mut seen_names = HashSet::new();

        let input_devices = self.host.input_devices()?;
        for device in input_devices {
            if let Ok(name) = device.name() {
                if seen_names.contains(&name) {
                    continue;
                }
                seen_names.insert(name.clone());

                let info = self.get_device_info(&device, AudioDevice::Input)?;
                devices.push(info);
            }
        }

        let output_devices = self.host.output_devices()?;
        for device in output_devices {
            if let Ok(name) = device.name() {
                if seen_names.contains(&name) {
                    continue;
                }
                seen_names.insert(name.clone());

                let info = self.get_device_info(&device, AudioDevice::Output)?;
                devices.push(info);
            }
        }

        Ok(devices)
    }

    fn get_device_info(
        &self,
        device: &cpal::Device,
        device_type: AudioDevice,
    ) -> Result<AudioDeviceInfo> {
        let name = device.name()?;
        let id = format!("{:?}", device.name().unwrap_or_default());

        let mut channels = 0;
        let mut sample_rates = Vec::new();
        let mut buffer_sizes = Vec::new();

        if let Ok(configs) = device.supported_input_configs() {
            for config in configs {
                channels = config.channels() as usize;
                sample_rates.push(config.min_sample_rate().0);
                sample_rates.push(config.max_sample_rate().0);
            }
        }

        if let Ok(configs) = device.supported_output_configs() {
            for config in configs {
                if channels == 0 {
                    channels = config.channels() as usize;
                }
                sample_rates.push(config.min_sample_rate().0);
                sample_rates.push(config.max_sample_rate().0);
            }
        }

        sample_rates.sort();
        sample_rates.dedup();

        buffer_sizes.push(64);
        buffer_sizes.push(128);
        buffer_sizes.push(256);
        buffer_sizes.push(512);
        buffer_sizes.push(1024);
        buffer_sizes.push(2048);

        Ok(AudioDeviceInfo {
            id,
            name,
            device_type: match device_type {
                AudioDevice::Input => "input".to_string(),
                AudioDevice::Output => "output".to_string(),
            },
            channels,
            sample_rates,
            buffer_sizes,
            is_exclusive: false,
        })
    }

    pub fn build_input_stream<F>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        mut data_callback: F,
    ) -> Result<cpal::Stream>
    where
        F: FnMut(&[f32], &cpal::InputCallbackInfo) + Send + 'static,
    {
        let stream = device.build_input_stream(
            config,
            move |data: &[f32], info: &cpal::InputCallbackInfo| {
                data_callback(data, info);
            },
            move |err| {
                eprintln!("Input stream error: {}", err);
            },
            None,
        )?;

        Ok(stream)
    }

    pub fn build_output_stream<F>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        mut data_callback: F,
    ) -> Result<cpal::Stream>
    where
        F: FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static,
    {
        let stream = device.build_output_stream(
            config,
            move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
                data_callback(data, info);
            },
            move |err| {
                eprintln!("Output stream error: {}", err);
            },
            None,
        )?;

        Ok(stream)
    }

    pub fn get_default_input_device(&self) -> Option<cpal::Device> {
        self.host.default_input_device()
    }

    pub fn get_default_output_device(&self) -> Option<cpal::Device> {
        self.host.default_output_device()
    }
}
