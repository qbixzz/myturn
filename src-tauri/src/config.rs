use std::path::Path;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::color_rules::{ColorRule, default_color_rules};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default = "default_provider")]
    pub active_provider: String,
    #[serde(default = "default_metric")]
    pub display_metric: String,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default = "default_color_rules")]
    pub color_rules: Vec<ColorRule>,
    #[serde(default)]
    pub providers: Value,
}

fn default_provider() -> String { "claude_cli".into() }
fn default_metric()   -> String { "tokens_percentage".into() }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_provider: default_provider(),
            display_metric:  default_metric(),
            auto_start:      false,
            color_rules:     default_color_rules(),
            providers:       Value::Object(Default::default()),
        }
    }
}

/// Loads AppConfig from `path`. Returns default config on any error
/// (missing file, invalid JSON, partial fields).
pub fn load(path: &Path) -> AppConfig {
    let raw = match std::fs::read_to_string(path) {
        Ok(s)  => s,
        Err(_) => return AppConfig::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn valid_config_loads_correctly() {
        let json = r##"{
            "active_provider": "claude_cli",
            "display_metric":  "tokens_percentage",
            "auto_start": true,
            "color_rules": [
                { "min": 0,  "max": 44,  "hex": "#2ECC71", "label": "Normal"   },
                { "min": 45, "max": 64,  "hex": "#F1C40F", "label": "Moderate" },
                { "min": 65, "max": 84,  "hex": "#E67E22", "label": "Warning"  },
                { "min": 85, "max": 100, "hex": "#E74C3C", "label": "Critical" }
            ],
            "providers": {}
        }"##;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(json.as_bytes()).unwrap();

        let cfg = load(f.path());
        assert_eq!(cfg.active_provider, "claude_cli");
        assert_eq!(cfg.display_metric,  "tokens_percentage");
        assert!(cfg.auto_start);
        assert_eq!(cfg.color_rules.len(), 4);
        assert_eq!(cfg.color_rules[0].hex, "#2ECC71");
    }

    #[test]
    fn missing_file_returns_default_config() {
        let cfg = load(Path::new("/nonexistent/path/config.json"));
        assert_eq!(cfg.active_provider, "claude_cli");
        assert_eq!(cfg.color_rules.len(), 4);
    }

    #[test]
    fn invalid_json_returns_default_config() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"{ this is not valid json }").unwrap();

        let cfg = load(f.path());
        assert_eq!(cfg.active_provider, "claude_cli");
        assert_eq!(cfg.color_rules.len(), 4);
    }
}
