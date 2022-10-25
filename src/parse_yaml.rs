use std::{collections::HashMap, fs::File, path::Path};

use log::{error, info};
use serde::Deserialize;

use crate::error::ArrError;

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct AtomicReadTeamTechnique {
    pub attack_technique: String,
    pub display_name: String,
    pub atomic_tests: Vec<AtomicTest>,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct AtomicTest {
    pub name: String,
    pub auto_generated_guid: String,
    pub description: String,
    pub supported_platforms: Vec<String>,
    pub executor: AtomicExecutor,
    #[serde(default = "HashMap::new")]
    pub input_arguments: HashMap<String, AtomicInputArg>,
    pub dependency_executor_name: Option<String>,
    pub dependencies: Option<Vec<AtomicDependency>>,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct AtomicExecutor {
    pub name: String,
    pub elevation_required: Option<bool>,
    pub command: Option<String>,
    pub cleanup_command: Option<String>,
    pub steps: Option<String>,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct AtomicDependency {
    pub description: String,
    pub prereq_command: String,
    pub get_prereq_command: String,
}

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct AtomicInputArg {
    pub description: String,
    pub default: String,
    #[serde(rename = "type")]
    pub arg_type: String,
}

pub fn parse_art_file(art_technique_file: &Path) -> Result<AtomicReadTeamTechnique, ArrError> {
    let f = std::fs::File::open(art_technique_file)
        .map_err(|_| ArrError::FileNotFound(art_technique_file.to_string_lossy().to_string()))?;

    match serde_yaml::from_reader::<File, AtomicReadTeamTechnique>(f) {
        Ok(y) => {
            info!(
                "Successfully parsed: {}",
                &art_technique_file.to_string_lossy()
            );
            Ok(y)
        }
        Err(e) => {
            error!("Failed to parse: {}", &art_technique_file.to_string_lossy());
            Err(ArrError::CannotParseYaml(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_file() {
        assert!(parse_art_file(Path::new("T1574.006.yaml")).is_ok())
    }
}
