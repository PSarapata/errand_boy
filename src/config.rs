use crate::model::AppConfig;
use std::path::PathBuf;

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("errand_boy")
        .join("config.toml")
}

pub fn load() -> Option<AppConfig> {
    let path = config_path();
    let contents = std::fs::read_to_string(path).ok()?;
    toml::from_str(&contents).ok()
}

pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let contents = toml::to_string(config).map_err(|e| e.to_string())?;
    std::fs::write(path, contents).map_err(|e| e.to_string())
}
