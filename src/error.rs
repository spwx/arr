use std::{error::Error, fmt};

#[derive(Debug, PartialEq)]
pub enum ArrError {
    ArgValueNotFound(String),
    OtherNomError(String),
    OsNotSupported,
    FileNotFound(String),
    CannotParseYaml(String),
    CannotLocateYamlFile,
    FilePathNotSet(String),
    CommandIoFailure(String),
    CommandExecutionFailed,
    RootRequired,
    Other(String),
}

impl Error for ArrError {}

impl fmt::Display for ArrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ArrError::ArgValueNotFound(s) => write!(f, "{}", s),
            ArrError::OtherNomError(s) => write!(f, "{}", s),
            ArrError::OsNotSupported => write!(f, "The test does not support this OS"),
            ArrError::FileNotFound(s) => write!(f, "{}", s),
            ArrError::CannotParseYaml(s) => write!(f, "{}", s),
            ArrError::CannotLocateYamlFile => write!(f, "Cannot Locate YAML file"),
            ArrError::FilePathNotSet(s) => write!(f, "{}", s),
            ArrError::Other(s) => write!(f, "{}", s),
            ArrError::RootRequired => write!(f, "Root required"),
            ArrError::CommandIoFailure(s) => write!(f, "{}", s),
            ArrError::CommandExecutionFailed => {
                write!(f, "Command ran, but returned an unsuccess return code")
            }
        }
    }
}
