use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use anyhow::{Result, anyhow};
use libloading::Library;
use serde::{Serialize, Deserialize};

use super::scanner::{PluginInfo, Vst3Scanner};
use super::plugin::{PluginParameter, PluginInstance};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedPluginLibrary {
    pub path: PathBuf,
    pub plugin_id: String,
}

pub struct Vst3Host {
    pub scanner: Vst3Scanner,
    pub loaded_plugins: HashMap<String, PluginInstance>,
    loaded_libraries: HashMap<String, Arc<Mutex<Option<SendableLibrary>>>>,
}

struct SendableLibrary(Library);

unsafe impl Send for SendableLibrary {}
unsafe impl Sync for SendableLibrary {}

impl Vst3Host {
    pub fn new() -> Self {
        Self {
            scanner: Vst3Scanner::new(),
            loaded_plugins: HashMap::new(),
            loaded_libraries: HashMap::new(),
        }
    }

    pub fn scan_plugins(&mut self) -> Result<Vec<PluginInfo>> {
        self.scanner.scan_plugins()
    }

    pub fn get_scanned_plugins(&self) -> Vec<&PluginInfo> {
        self.scanner.plugin_registry.values().collect()
    }

    pub fn get_plugin_info(&self, plugin_id: &str) -> Option<&PluginInfo> {
        self.scanner.get_plugin(plugin_id)
    }

    unsafe fn load_vst3_library(path: &PathBuf) -> Result<SendableLibrary> {
        let lib = Library::new(path)?;
        Ok(SendableLibrary(lib))
    }

    fn create_default_parameters(plugin_info: &PluginInfo) -> Vec<PluginParameter> {
        let mut params = Vec::new();

        params.push(PluginParameter::new(
            "bypass".to_string(),
            "Bypass".to_string(),
            0.0,
            1.0,
            0.0,
            "".to_string(),
        ));

        params.push(PluginParameter::new(
            "gain".to_string(),
            "Gain".to_string(),
            -60.0,
            12.0,
            0.0,
            "dB".to_string(),
        ));

        match plugin_info.category {
            super::scanner::PluginCategory::Eq => {
                params.push(PluginParameter::new(
                    "low_gain".to_string(),
                    "Low Gain".to_string(),
                    -12.0,
                    12.0,
                    0.0,
                    "dB".to_string(),
                ));
                params.push(PluginParameter::new(
                    "mid_gain".to_string(),
                    "Mid Gain".to_string(),
                    -12.0,
                    12.0,
                    0.0,
                    "dB".to_string(),
                ));
                params.push(PluginParameter::new(
                    "high_gain".to_string(),
                    "High Gain".to_string(),
                    -12.0,
                    12.0,
                    0.0,
                    "dB".to_string(),
                ));
            }
            super::scanner::PluginCategory::Compressor => {
                params.push(PluginParameter::new(
                    "threshold".to_string(),
                    "Threshold".to_string(),
                    -60.0,
                    0.0,
                    -20.0,
                    "dB".to_string(),
                ));
                params.push(PluginParameter::new(
                    "ratio".to_string(),
                    "Ratio".to_string(),
                    1.0,
                    20.0,
                    4.0,
                    ":1".to_string(),
                ));
                params.push(PluginParameter::new(
                    "attack".to_string(),
                    "Attack".to_string(),
                    0.1,
                    100.0,
                    10.0,
                    "ms".to_string(),
                ));
                params.push(PluginParameter::new(
                    "release".to_string(),
                    "Release".to_string(),
                    10.0,
                    1000.0,
                    100.0,
                    "ms".to_string(),
                ));
            }
            super::scanner::PluginCategory::Reverb => {
                params.push(PluginParameter::new(
                    "mix".to_string(),
                    "Mix".to_string(),
                    0.0,
                    100.0,
                    30.0,
                    "%".to_string(),
                ));
                params.push(PluginParameter::new(
                    "decay".to_string(),
                    "Decay".to_string(),
                    0.1,
                    10.0,
                    2.0,
                    "s".to_string(),
                ));
                params.push(PluginParameter::new(
                    "pre_delay".to_string(),
                    "Pre-Delay".to_string(),
                    0.0,
                    100.0,
                    10.0,
                    "ms".to_string(),
                ));
            }
            super::scanner::PluginCategory::Delay => {
                params.push(PluginParameter::new(
                    "delay_time".to_string(),
                    "Delay Time".to_string(),
                    1.0,
                    2000.0,
                    250.0,
                    "ms".to_string(),
                ));
                params.push(PluginParameter::new(
                    "feedback".to_string(),
                    "Feedback".to_string(),
                    0.0,
                    100.0,
                    30.0,
                    "%".to_string(),
                ));
                params.push(PluginParameter::new(
                    "mix".to_string(),
                    "Mix".to_string(),
                    0.0,
                    100.0,
                    30.0,
                    "%".to_string(),
                ));
            }
            super::scanner::PluginCategory::Utility => {
                params.push(PluginParameter::new(
                    "mix".to_string(),
                    "Mix".to_string(),
                    0.0,
                    100.0,
                    100.0,
                    "%".to_string(),
                ));
            }
        }

        params
    }

    pub fn load_plugin(&mut self, plugin_id: String) -> Result<String> {
        let plugin_info = self.scanner.get_plugin(&plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?
            .clone();

        let library = unsafe { Self::load_vst3_library(&plugin_info.path) }?;
        let library_arc = Arc::new(Mutex::new(Some(library)));

        self.loaded_libraries.insert(plugin_id.clone(), library_arc);

        let default_params = Self::create_default_parameters(&plugin_info);
        let instance = PluginInstance::with_parameters(plugin_id.clone(), default_params);
        let instance_id = instance.instance_id.clone();

        self.loaded_plugins.insert(instance_id.clone(), instance);

        Ok(instance_id)
    }

    pub fn unload_plugin(&mut self, instance_id: &str) -> Result<()> {
        let instance = self.loaded_plugins.remove(instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        if let Some(library) = self.loaded_libraries.remove(&instance.plugin_id) {
            if let Some(lib_guard) = library.lock().take() {
                drop(lib_guard);
            }
        }

        Ok(())
    }

    pub fn get_plugin_instance(&self, instance_id: &str) -> Option<&PluginInstance> {
        self.loaded_plugins.get(instance_id)
    }

    pub fn get_plugin_instance_mut(&mut self, instance_id: &str) -> Option<&mut PluginInstance> {
        self.loaded_plugins.get_mut(instance_id)
    }

    pub fn set_parameter(&mut self, instance_id: String, param_id: String, value: f32) -> Result<()> {
        let instance = self.loaded_plugins.get_mut(&instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        instance.set_parameter(param_id, value);
        Ok(())
    }

    pub fn get_parameter(&self, instance_id: &str, param_id: &str) -> Result<f32> {
        let instance = self.loaded_plugins.get(instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        instance.get_parameter(param_id)
            .ok_or_else(|| anyhow!("Parameter not found: {}", param_id))
    }

    pub fn get_parameters(&self, instance_id: &str) -> Result<HashMap<String, f32>> {
        let instance = self.loaded_plugins.get(instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        Ok(instance.get_parameter_values())
    }

    pub fn get_parameter_infos(&self, instance_id: &str) -> Result<Vec<&PluginParameter>> {
        let instance = self.loaded_plugins.get(instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        Ok(instance.get_parameter_list())
    }

    pub fn set_parameter_values(&mut self, instance_id: &str, values: &HashMap<String, f32>) -> Result<()> {
        let instance = self.loaded_plugins.get_mut(instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        instance.set_parameter_values(values);
        Ok(())
    }

    pub fn get_all_loaded_instances(&self) -> Vec<&PluginInstance> {
        self.loaded_plugins.values().collect()
    }

    pub fn reset_plugin_parameters(&mut self, instance_id: &str) -> Result<()> {
        let instance = self.loaded_plugins.get_mut(instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        instance.reset_all_parameters();
        Ok(())
    }

    pub fn bypass_plugin(&mut self, instance_id: &str, bypassed: bool) -> Result<()> {
        let instance = self.loaded_plugins.get_mut(instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        instance.bypassed = bypassed;
        Ok(())
    }

    pub fn enable_plugin(&mut self, instance_id: &str, enabled: bool) -> Result<()> {
        let instance = self.loaded_plugins.get_mut(instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        instance.enabled = enabled;
        Ok(())
    }

    pub fn set_plugin_preset(&mut self, instance_id: &str, preset: String) -> Result<()> {
        let instance = self.loaded_plugins.get_mut(instance_id)
            .ok_or_else(|| anyhow!("Plugin instance not found: {}", instance_id))?;

        instance.set_preset(preset);
        Ok(())
    }

    pub fn get_plugin_paths() -> Vec<PathBuf> {
        Vst3Scanner::get_plugin_paths()
    }

    pub fn get_loaded_plugin_count(&self) -> usize {
        self.loaded_plugins.len()
    }

    pub fn get_scanned_plugin_count(&self) -> usize {
        self.scanner.plugin_registry.len()
    }

    pub fn clear(&mut self) {
        self.loaded_plugins.clear();
        for (_, library) in self.loaded_libraries.drain() {
            if let Some(lib_guard) = library.lock().take() {
                drop(lib_guard);
            }
        }
    }
}

impl Default for Vst3Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Vst3Host {
    fn drop(&mut self) {
        self.clear();
    }
}
