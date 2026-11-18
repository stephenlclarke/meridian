use std::{fmt, io, path::Path};

#[derive(Debug)]
pub enum ImportError {
    Identity(&'static str),
    Geometry(&'static str),
    Decode(String),
    Io {
        action: &'static str,
        path: String,
        source: io::Error,
    },
    Git(String),
    Mismatch(String),
    DestinationExists(String),
    Usage(String),
}

impl ImportError {
    pub(crate) fn io(action: &'static str, path: &Path, source: io::Error) -> Self {
        Self::Io {
            action,
            path: path.display().to_string(),
            source,
        }
    }
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(message) | Self::Geometry(message) => f.write_str(message),
            Self::Decode(message) => write!(f, "Invalid source document: {message}"),
            Self::Io {
                action,
                path,
                source,
            } => write!(f, "Cannot {action} {path}: {source}"),
            Self::Git(message) => write!(f, "Pinned Git lookup failed: {message}"),
            Self::Mismatch(message) => write!(f, "Reference mismatch: {message}"),
            Self::DestinationExists(path) => write!(f, "Destination already exists: {path}"),
            Self::Usage(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, ImportError>;
