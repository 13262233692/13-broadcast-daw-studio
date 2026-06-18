use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::Result;
use libloading::Library;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginCategory {
    Eq,
    Compressor,
    Reverb,
    Delay,
    Utility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub category: PluginCategory,
    pub version: String,
    pub path: PathBuf,
    pub plugin_format: String,
    pub has_editor: bool,
    pub input_bus_count: u32,
    pub output_bus_count: u32,
}

pub struct Vst3Scanner {
    pub plugin_registry: HashMap<String, PluginInfo>,
}

impl Vst3Scanner {
    pub fn new() -> Self {
        Self {
            plugin_registry: HashMap::new(),
        }
    }

    pub fn get_plugin_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        #[cfg(target_os = "windows")]
        {
            let program_files = PathBuf::from(r"C:\Program Files\Common Files\VST3");
            let program_files_x86 = PathBuf::from(r"C:\Program Files (x86)\Common Files\VST3");
            if program_files.exists() {
                paths.push(program_files);
            }
            if program_files_x86.exists() {
                paths.push(program_files_x86);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let system_plugins = PathBuf::from("/Library/Audio/Plug-Ins/VST3");
            if let Some(home) = std::env::var_os("HOME") {
                let user_plugins = PathBuf::from(home).join("Library/Audio/Plug-Ins/VST3");
                if user_plugins.exists() {
                    paths.push(user_plugins);
                }
            }
            if system_plugins.exists() {
                paths.push(system_plugins);
            }
        }

        #[cfg(target_os = "linux")]
        {
            let system_plugins = PathBuf::from("/usr/lib/vst3");
            let local_plugins = PathBuf::from("/usr/local/lib/vst3");
            if let Some(home) = std::env::var_os("HOME") {
                let user_plugins = PathBuf::from(home).join(".vst3");
                if user_plugins.exists() {
                    paths.push(user_plugins);
                }
            }
            if system_plugins.exists() {
                paths.push(system_plugins);
            }
            if local_plugins.exists() {
                paths.push(local_plugins);
            }
        }

        paths
    }

    fn collect_vst3_files(dir: &Path, files: &mut Vec<PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::collect_vst3_files(&path, files);
                } else if let Some(ext) = path.extension() {
                    #[cfg(target_os = "windows")]
                    let vst3_ext = "vst3";
                    #[cfg(target_os = "macos")]
                    let vst3_ext = "vst3";
                    #[cfg(target_os = "linux")]
                    let vst3_ext = "so";

                    if ext == vst3_ext {
                        files.push(path);
                    }
                }
            }
        }
    }

    unsafe fn load_plugin_info(path: &Path) -> Result<PluginInfo> {
        let lib = Library::new(path)?;

        let plugin_id = format!("vst3_{}", path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string());

        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Plugin")
            .to_string();

        let category = Self::detect_category(&name);

        let info = PluginInfo {
            id: plugin_id,
            name,
            vendor: "Unknown".to_string(),
            category,
            version: "1.0.0".to_string(),
            path: path.to_path_buf(),
            plugin_format: "VST3".to_string(),
            has_editor: true,
            input_bus_count: 1,
            output_bus_count: 1,
        };

        std::mem::forget(lib);

        Ok(info)
    }

    fn detect_category(name: &str) -> PluginCategory {
        let name_lower = name.to_lowercase();
        if name_lower.contains("eq") || name_lower.contains("equalizer") {
            PluginCategory::Eq
        } else if name_lower.contains("comp") || name_lower.contains("compress") {
            PluginCategory::Compressor
        } else if name_lower.contains("reverb") || name_lower.contains("hall") || name_lower.contains("room") {
            PluginCategory::Reverb
        } else if name_lower.contains("delay") || name_lower.contains("echo") {
            PluginCategory::Delay
        } else {
            PluginCategory::Utility
        }
    }

    pub fn scan_plugins(&mut self) -> Result<Vec<PluginInfo>> {
        let mut plugins = Vec::new();
        let search_paths = Self::get_plugin_paths();

        for search_path in &search_paths {
            let mut vst3_files = Vec::new();
            Self::collect_vst3_files(search_path, &mut vst3_files);

            for file_path in vst3_files {
                match unsafe { Self::load_plugin_info(&file_path) } {
                    Ok(info) => {
                        self.plugin_registry.insert(info.id.clone(), info.clone());
                        plugins.push(info);
                    }
                    Err(e) => {
                        eprintln!("Failed to load plugin {:?}: {}", file_path, e);
                    }
                }
            }
        }

        Ok(plugins)
    }

    pub fn get_plugin(&self, id: &str) -> Option<&PluginInfo> {
        self.plugin_registry.get(id)
    }
}

impl Default for Vst3Scanner {
    fn default() -> Self {
        Self::new()
    }
}
