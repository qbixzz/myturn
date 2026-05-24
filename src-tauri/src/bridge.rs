use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContextUsage {
    pub used_tokens: u64,
    pub max_tokens:  u64,
    pub percent:     f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BridgeData {
    pub schema_version: u32,
    pub updated_at:     String,
    pub session_id:     String,
    pub model:          Option<String>,
    pub context:        ContextUsage,
    // `metrics` intentionally omitted — reserved for Phase 2, ignored on read.
}

/// Reads and deserializes the bridge file. Returns None if the file is
/// missing, unreadable, or contains invalid JSON.
pub fn read_bridge(path: &Path) -> Option<BridgeData> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_bridge(content: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        f
    }

    #[test]
    fn valid_bridge_file_deserializes_correctly() {
        let json = br##"{
            "schema_version": 1,
            "updated_at": "2026-05-24T10:00:00Z",
            "session_id": "abc-123",
            "model": "claude-sonnet-4-6",
            "context": { "used_tokens": 33810, "max_tokens": 200000, "percent": 16.91 },
            "metrics": {}
        }"##;
        let f = write_bridge(json);

        let data = read_bridge(f.path()).expect("should parse valid bridge file");
        assert_eq!(data.schema_version, 1);
        assert_eq!(data.session_id, "abc-123");
        assert_eq!(data.context.used_tokens, 33810);
        assert_eq!(data.context.max_tokens, 200_000);
        assert!((data.context.percent - 16.91).abs() < 0.001);
    }

    #[test]
    fn missing_file_returns_none() {
        assert!(read_bridge(Path::new("/nonexistent/myturn-bridge.json")).is_none());
    }

    #[test]
    fn malformed_json_returns_none() {
        let f = write_bridge(b"{ not valid json }");
        assert!(read_bridge(f.path()).is_none());
    }

    #[test]
    fn schema_version_is_one() {
        let json = br##"{
            "schema_version": 1,
            "updated_at": "2026-05-24T10:00:00Z",
            "session_id": "xyz",
            "model": null,
            "context": { "used_tokens": 1000, "max_tokens": 200000, "percent": 0.5 },
            "metrics": {}
        }"##;
        let f = write_bridge(json);
        let data = read_bridge(f.path()).unwrap();
        assert_eq!(data.schema_version, 1);
    }
}
