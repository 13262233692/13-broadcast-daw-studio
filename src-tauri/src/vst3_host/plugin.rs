use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginParameter {
    pub id: String,
    pub name: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
    pub steps: u32,
    pub unit: String,
    pub is_bypass: bool,
    pub is_automated: bool,
}

impl PluginParameter {
    pub fn new(
        id: String,
        name: String,
        min: f32,
        max: f32,
        default_value: f32,
        unit: String,
    ) -> Self {
        Self {
            id,
            name,
            value: default_value,
            min,
            max,
            default_value,
            steps: 0,
            unit,
            is_bypass: false,
            is_automated: false,
        }
    }

    pub fn normalized_value(&self) -> f32 {
        if self.max - self.min == 0.0 {
            return 0.0;
        }
        (self.value - self.min) / (self.max - self.min)
    }

    pub fn set_normalized_value(&mut self, normalized: f32) {
        let clamped = normalized.clamp(0.0, 1.0);
        self.value = self.min + clamped * (self.max - self.min);
    }

    pub fn reset_to_default(&mut self) {
        self.value = self.default_value;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstance {
    pub instance_id: String,
    pub plugin_id: String,
    pub parameters: HashMap<String, PluginParameter>,
    pub bypassed: bool,
    pub enabled: bool,
    pub preset: Option<String>,
}

impl PluginInstance {
    pub fn new(plugin_id: String) -> Self {
        Self {
            instance_id: Uuid::new_v4().to_string(),
            plugin_id,
            parameters: HashMap::new(),
            bypassed: false,
            enabled: true,
            preset: None,
        }
    }

    pub fn with_parameters(plugin_id: String, parameters: Vec<PluginParameter>) -> Self {
        let mut param_map = HashMap::new();
        for param in parameters {
            param_map.insert(param.id.clone(), param);
        }
        Self {
            instance_id: Uuid::new_v4().to_string(),
            plugin_id,
            parameters: param_map,
            bypassed: false,
            enabled: true,
            preset: None,
        }
    }

    pub fn set_parameter(&mut self, param_id: String, value: f32) {
        if let Some(param) = self.parameters.get_mut(&param_id) {
            param.value = value.clamp(param.min, param.max);
        }
    }

    pub fn get_parameter(&self, param_id: &str) -> Option<f32> {
        self.parameters.get(param_id).map(|p| p.value)
    }

    pub fn get_parameter_info(&self, param_id: &str) -> Option<&PluginParameter> {
        self.parameters.get(param_id)
    }

    pub fn set_normalized_parameter(&mut self, param_id: String, normalized: f32) {
        if let Some(param) = self.parameters.get_mut(&param_id) {
            param.set_normalized_value(normalized);
        }
    }

    pub fn get_normalized_parameter(&self, param_id: &str) -> Option<f32> {
        self.parameters.get(param_id).map(|p| p.normalized_value())
    }

    pub fn add_parameter(&mut self, parameter: PluginParameter) {
        self.parameters.insert(parameter.id.clone(), parameter);
    }

    pub fn remove_parameter(&mut self, param_id: &str) -> Option<PluginParameter> {
        self.parameters.remove(param_id)
    }

    pub fn reset_all_parameters(&mut self) {
        for param in self.parameters.values_mut() {
            param.reset_to_default();
        }
    }

    pub fn toggle_bypass(&mut self) {
        self.bypassed = !self.bypassed;
    }

    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn set_preset(&mut self, preset: String) {
        self.preset = Some(preset);
    }

    pub fn clear_preset(&mut self) {
        self.preset = None;
    }

    pub fn get_parameter_list(&self) -> Vec<&PluginParameter> {
        let mut params: Vec<&PluginParameter> = self.parameters.values().collect();
        params.sort_by(|a, b| a.name.cmp(&b.name));
        params
    }

    pub fn get_parameter_values(&self) -> HashMap<String, f32> {
        self.parameters
            .iter()
            .map(|(id, param)| (id.clone(), param.value))
            .collect()
    }

    pub fn set_parameter_values(&mut self, values: &HashMap<String, f32>) {
        for (id, value) in values {
            if let Some(param) = self.parameters.get_mut(id) {
                param.value = value.clamp(param.min, param.max);
            }
        }
    }
}
