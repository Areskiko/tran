pub enum TranError {
    ConfigError(String),
    FileReadError(String),
    FileNotFoundError(String),
    WritingConfigError(String),
    PngFormatError(String),
    UnsupportedError(String),
}

impl std::fmt::Display for TranError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranError::ConfigError(reason) => write!(f, "Error reading config: {}", reason),
            TranError::FileReadError(contents) => write!(f, "Error reading file {}", contents),
            TranError::FileNotFoundError(file_name) => {
                write!(f, "Could not find file {}", file_name)
            }
            TranError::WritingConfigError(contents) => {
                write!(f, "Could not write config file {}", contents)
            }
            TranError::PngFormatError(reason) => write!(f, "Error reading png file: {}", reason),
            TranError::UnsupportedError(reason) => write!(f, "{}", reason),
        }
    }
}

impl std::fmt::Debug for TranError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranError::ConfigError(reason) => write!(f, "Error reading config: {}", reason),
            TranError::FileReadError(contents) => write!(f, "Error reading file {}", contents),
            TranError::FileNotFoundError(file_name) => {
                write!(f, "Could not find file {}", file_name)
            }
            TranError::WritingConfigError(contents) => {
                write!(f, "Could not write config file {}", contents)
            }
            TranError::PngFormatError(reason) => write!(f, "Error reading png file: {}", reason),
            TranError::UnsupportedError(reason) => write!(f, "{}", reason),
        }
    }
}

impl std::error::Error for TranError {}

impl From<std::io::Error> for TranError {
    fn from(value: std::io::Error) -> Self {
        TranError::FileReadError(value.to_string())
    }
}


impl From<std::num::ParseIntError> for TranError {
    fn from(value: std::num::ParseIntError) -> Self {
        TranError::ConfigError(value.to_string())
    }
}
