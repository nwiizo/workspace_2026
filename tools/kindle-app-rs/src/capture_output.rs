use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use chrono::Local;

use crate::image_ops::list_page_images;

pub(crate) fn prepare(output: &Path, overwrite: bool) -> Result<()> {
    fs::create_dir_all(output)
        .with_context(|| format!("failed to create capture output: {}", output.display()))?;
    let pages = list_page_images(output)?;
    let metadata = output.join("metadata.json");
    let pending = fs::read_dir(output)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            name.to_string_lossy()
                .contains(".pending")
                .then_some(entry.path())
        })
        .collect::<Vec<_>>();
    if pages.is_empty() && !metadata.exists() && pending.is_empty() {
        return Ok(());
    }
    ensure!(
        overwrite,
        "capture output already contains pages; pass --overwrite to archive and replace them: {}",
        output.display()
    );

    let backup = output
        .join(".capture-backup")
        .join(Local::now().format("%Y%m%dT%H%M%S%.3f").to_string());
    ensure!(
        !backup.exists(),
        "capture backup path already exists: {}",
        backup.display()
    );
    fs::create_dir_all(&backup)
        .with_context(|| format!("failed to create capture backup: {}", backup.display()))?;
    for path in pages
        .into_iter()
        .chain(metadata.exists().then_some(metadata))
        .chain(pending)
    {
        let destination = backup.join(path.file_name().context("capture file has no name")?);
        fs::rename(&path, &destination).with_context(|| {
            format!(
                "failed to archive {} to {}",
                path.display(),
                destination.display()
            )
        })?;
    }
    eprintln!("previous capture archived at {}", backup.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::prepare;

    #[test]
    fn overwrite_archives_existing_capture_instead_of_deleting_it() {
        let directory = tempdir().unwrap();
        fs::write(directory.path().join("page_0001.png"), b"old page").unwrap();
        fs::write(directory.path().join("metadata.json"), b"old metadata").unwrap();
        fs::write(directory.path().join(".0002.pending.png"), b"pending 2").unwrap();
        fs::write(directory.path().join(".0003.pending.png"), b"pending 3").unwrap();

        assert!(prepare(directory.path(), false).is_err());
        prepare(directory.path(), true).unwrap();

        let backup_root = directory.path().join(".capture-backup");
        let backup = fs::read_dir(backup_root)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(fs::read(backup.join("page_0001.png")).unwrap(), b"old page");
        assert_eq!(
            fs::read(backup.join("metadata.json")).unwrap(),
            b"old metadata"
        );
        assert_eq!(
            fs::read(backup.join(".0002.pending.png")).unwrap(),
            b"pending 2"
        );
        assert_eq!(
            fs::read(backup.join(".0003.pending.png")).unwrap(),
            b"pending 3"
        );
    }
}
