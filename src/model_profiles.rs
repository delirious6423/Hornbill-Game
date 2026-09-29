use anyhow::{Context, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn spec(profile: &str) -> Result<Value> {
    let manifest: Value = serde_json::from_str(include_str!("../workers/gemma/models.json"))?;
    manifest
        .get(profile)
        .cloned()
        .context("unknown story model")
}

pub fn path(runtime: &Path, profile: &str) -> Result<PathBuf> {
    let spec = spec(profile)?;
    Ok(runtime.join("models").join(
        spec["directory"]
            .as_str()
            .context("missing model directory")?,
    ))
}

pub fn installed(runtime: &Path, profile: &str) -> bool {
    (|| -> Result<bool> {
        let wanted = spec(profile)?;
        let receipt: Value = serde_json::from_str(&std::fs::read_to_string(
            path(runtime, profile)?.join("hornbill-model.json"),
        )?)?;
        if receipt != wanted {
            return Ok(false);
        }
        let directory = path(runtime, profile)?;
        for (name, detail) in wanted["files"].as_object().context("missing model files")? {
            let metadata = std::fs::metadata(directory.join(name))?;
            if !metadata.is_file() || Some(metadata.len()) != detail["bytes"].as_u64() {
                return Ok(false);
            }
        }
        Ok(true)
    })()
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_or_incomplete_installs_are_not_reported_ready() {
        let runtime = tempfile::tempdir().unwrap();
        let directory = path(runtime.path(), "12b").unwrap();
        std::fs::create_dir_all(&directory).unwrap();
        let receipt = directory.join("hornbill-model.json");
        std::fs::write(&receipt, "{\"repo\":\"old-profile\"}").unwrap();
        assert!(!installed(runtime.path(), "12b"));
        std::fs::write(&receipt, serde_json::to_vec(&spec("12b").unwrap()).unwrap()).unwrap();
        assert!(!installed(runtime.path(), "12b"));
        assert!(!installed(runtime.path(), "unknown"));
    }
}
