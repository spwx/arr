use log::{error, info};
use walkdir::{DirEntry, WalkDir};

use crate::error::ArrError;
use std::path::{Path, PathBuf};

pub fn find_file(technique: &str, art_path: &Path) -> Result<PathBuf, ArrError> {
    let search_string = format!("{}.yaml", technique);

    let yaml_file = WalkDir::new(art_path)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| entry.file_name().eq_ignore_ascii_case(&search_string));

    match yaml_file {
        Some(f) => {
            info!(
                "Located matching YAML file for {}: {}",
                &technique,
                &f.clone().into_path().to_string_lossy()
            );
            Ok(f.into_path())
        }
        None => {
            error!(
                "Could not locate a YAML file for the technique: {}",
                &technique.to_uppercase()
            );
            Err(ArrError::CannotLocateYamlFile)
        }
    }
}

pub fn all_techniques(art_path: &Path) -> impl Iterator<Item = DirEntry> {
    WalkDir::new(art_path)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|e| {
            if let Some(f) = e.file_name().to_str() {
                if f.starts_with('T') && f.ends_with(".yaml") {
                    return Some(e);
                }
            }
            None
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_file() {
        assert_eq!(
            find_file("t1574.006", Path::new("atomic-red-team-master"))
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap(),
            "T1574.006.yaml"
        );
    }
}
