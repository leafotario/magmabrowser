use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct BrowserConfig {
    pub search_engine: String,
    pub hardware_acceleration: bool,
}

impl BrowserConfig {
    pub fn default() -> Self {
        Self {
            search_engine: "https://duckduckgo.com/?q={}".to_string(),
            hardware_acceleration: true,
        }
    }

    fn path() -> PathBuf {
        let mut p = std::env::current_dir().unwrap_or_default();
        p.push(".magma_browser_config");
        p
    }

    pub fn load() -> Self {
        let mut config = Self::default();
        if let Ok(content) = fs::read_to_string(Self::path()) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
                if let Some((k, v)) = trimmed.split_once('=') {
                    let k = k.trim();
                    let v = v.trim();
                    match k {
                        "search_engine" => config.search_engine = v.to_string(),
                        "hardware_acceleration" => config.hardware_acceleration = v == "true",
                        _ => {}
                    }
                }
            }
        }
        config
    }

    pub fn save(&self) {
        let content = format!(
            "search_engine={}\nhardware_acceleration={}\n",
            self.search_engine, self.hardware_acceleration
        );
        let _ = fs::write(Self::path(), content);
    }
}
