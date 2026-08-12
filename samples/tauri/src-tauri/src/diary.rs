use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use tempfile::NamedTempFile;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiaryError {
    #[error("日付は実在する YYYY-MM-DD 形式で指定してください")]
    InvalidDate,
    #[error("日記ファイルを読み書きできません: {0}")]
    Io(#[from] std::io::Error),
    #[error("日記ファイルを置き換えられません: {0}")]
    Persist(#[from] tempfile::PersistError),
}

pub fn list_entry_dates(directory: &Path) -> Result<Vec<String>, DiaryError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut dates = Vec::new();

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        if let Some(date) = path.file_stem().and_then(|stem| stem.to_str())
            && is_valid_date(date)
        {
            dates.push(date.to_owned());
        }
    }

    dates.sort_unstable_by(|left, right| right.cmp(left));
    Ok(dates)
}

pub fn read_entry(directory: &Path, date: &str) -> Result<Option<String>, DiaryError> {
    let path = entry_path(directory, date)?;
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn save_entry(directory: &Path, date: &str, content: &str) -> Result<(), DiaryError> {
    let path = entry_path(directory, date)?;
    fs::create_dir_all(directory)?;

    let mut temporary_file = NamedTempFile::new_in(directory)?;
    temporary_file.write_all(content.as_bytes())?;
    temporary_file.as_file().sync_all()?;
    temporary_file.persist(path)?;
    Ok(())
}

fn entry_path(directory: &Path, date: &str) -> Result<PathBuf, DiaryError> {
    if !is_valid_date(date) {
        return Err(DiaryError::InvalidDate);
    }
    Ok(directory.join(format!("{date}.md")))
}

fn is_valid_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return false;
    }

    let Some(year) = date[0..4].parse::<u16>().ok() else {
        return false;
    };
    let Some(month) = date[5..7].parse::<u8>().ok() else {
        return false;
    };
    let Some(day) = date[8..10].parse::<u8>().ok() else {
        return false;
    };

    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };

    year != 0 && (1..=days_in_month).contains(&day)
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{list_entry_dates, read_entry, save_entry};

    #[test]
    fn saves_and_reads_markdown_without_changing_it() {
        let directory = tempdir().expect("temporary directory should be created");
        let markdown = "# 朝の散歩\n\n今日は川沿いを歩いた。\n";

        save_entry(directory.path(), "2026-08-12", markdown).expect("entry should be saved");

        assert_eq!(
            read_entry(directory.path(), "2026-08-12").expect("entry should be read"),
            Some(markdown.to_owned())
        );
        assert_eq!(
            fs::read_to_string(directory.path().join("2026-08-12.md"))
                .expect("Markdown file should exist"),
            markdown
        );
    }

    #[test]
    fn lists_only_valid_entry_files_newest_first() {
        let directory = tempdir().expect("temporary directory should be created");
        fs::write(directory.path().join("2026-08-11.md"), "old")
            .expect("fixture should be written");
        fs::write(directory.path().join("2026-08-12.md"), "new")
            .expect("fixture should be written");
        fs::write(directory.path().join("notes.md"), "ignore").expect("fixture should be written");
        fs::write(directory.path().join("2026-08-13.txt"), "ignore")
            .expect("fixture should be written");

        assert_eq!(
            list_entry_dates(directory.path()).expect("entries should be listed"),
            vec!["2026-08-12", "2026-08-11"]
        );
    }

    #[test]
    fn overwrites_the_entry_for_the_same_day() {
        let directory = tempdir().expect("temporary directory should be created");
        save_entry(directory.path(), "2026-08-12", "first").expect("entry should be saved");

        save_entry(directory.path(), "2026-08-12", "second").expect("entry should be updated");

        assert_eq!(
            read_entry(directory.path(), "2026-08-12").expect("entry should be read"),
            Some("second".to_owned())
        );
        assert_eq!(
            list_entry_dates(directory.path()).expect("entries should be listed"),
            vec!["2026-08-12"]
        );
    }

    #[test]
    fn rejects_invalid_dates_before_building_a_path() {
        let directory = tempdir().expect("temporary directory should be created");

        for invalid_date in ["../secrets", "2026-02-30", "2025-02-29", "2026-8-12"] {
            assert!(
                save_entry(directory.path(), invalid_date, "unsafe").is_err(),
                "{invalid_date} should be rejected"
            );
        }
        assert!(!directory.path().join("secrets.md").exists());
    }

    #[test]
    fn accepts_a_leap_day() {
        let directory = tempdir().expect("temporary directory should be created");

        save_entry(directory.path(), "2024-02-29", "leap day")
            .expect("valid leap day should be saved");

        assert_eq!(
            list_entry_dates(directory.path()).expect("entries should be listed"),
            vec!["2024-02-29"]
        );
    }
}
