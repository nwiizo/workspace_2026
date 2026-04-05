use std::path::Path;

use crate::error::{Error, Result};
use crate::probe_data::ProbeData;

pub fn read_probe_data(output_dir: &Path) -> Result<Vec<ProbeData>> {
    if !output_dir.exists() {
        return Err(Error::NoData(output_dir.to_path_buf()));
    }

    let mut results = Vec::new();

    for entry in std::fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let content = std::fs::read_to_string(&path)?;
            let data: ProbeData = serde_json::from_str(&content)?;
            results.push(data);
        }
    }

    if results.is_empty() {
        return Err(Error::NoData(output_dir.to_path_buf()));
    }

    Ok(results)
}

pub fn read_probe_file(path: &Path) -> Result<ProbeData> {
    let content = std::fs::read_to_string(path)?;
    let data: ProbeData = serde_json::from_str(&content)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn read_from_directory() {
        let dir = std::env::temp_dir().join("rustprobe_reader_test");
        let _ = std::fs::create_dir_all(&dir);

        let data = ProbeData {
            crate_name: "test_crate".to_string(),
            functions: vec![],
        };

        let json = serde_json::to_string_pretty(&data).expect("serialize");
        let file_path = dir.join("test_crate.json");
        let mut file = std::fs::File::create(&file_path).expect("create file");
        file.write_all(json.as_bytes()).expect("write");

        let results = read_probe_data(&dir).expect("should read");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].crate_name, "test_crate");

        let _ = std::fs::remove_file(&file_path);
    }

    #[test]
    fn missing_directory() {
        let result = read_probe_data(Path::new("/tmp/rustprobe_nonexistent_dir"));
        assert!(result.is_err());
    }
}
