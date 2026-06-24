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

    fn path() -> Result<PathBuf, String> {
        let mut base = if cfg!(target_os = "windows") {
            if let Ok(appdata) = std::env::var("APPDATA") {
                PathBuf::from(appdata)
            } else {
                std::env::current_dir().unwrap_or_default()
            }
        } else if cfg!(target_os = "macos") {
            if let Ok(home) = std::env::var("HOME") {
                let mut p = PathBuf::from(home);
                p.push("Library");
                p.push("Application Support");
                p
            } else {
                std::env::current_dir().unwrap_or_default()
            }
        } else {
            if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
                PathBuf::from(xdg_config)
            } else if let Ok(home) = std::env::var("HOME") {
                let mut p = PathBuf::from(home);
                p.push(".config");
                p
            } else {
                std::env::current_dir().unwrap_or_default()
            }
        };
        base.push("PetalBrowser");
        if let Err(e) = fs::create_dir_all(&base) {
            return Err(format!("Falha de permissão ao criar diretório de configuração ({:?}): {}", base, e));
        }
        base.push("config.ini");
        Ok(base)
    }

    pub fn load() -> Self {
        let mut config = Self::default();
        match Self::path() {
            Ok(path) => {
                if let Ok(content) = fs::read_to_string(&path) {
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
                } else {
                    if let Err(e) = config.save() {
                        println!("⚠️ Aviso: O Petal rodará com configurações padrão, falha na persistência. Detalhe: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Aviso Crítico de Configuração: {}", e);
                println!("⚠️ O Petal rodará apenas com configurações temporárias.");
            }
        }
        config
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path()?;
        let content = format!(
            "search_engine={}\nhardware_acceleration={}\n",
            self.search_engine, self.hardware_acceleration
        );
        fs::write(&path, content).map_err(|e| format!("Erro de disco ao escrever no arquivo ({:?}): {}", path, e))
    }
}
