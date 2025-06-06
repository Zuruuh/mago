use crate::SourceIdentifier;

#[derive(Debug)]
pub enum SourceError {
    UnavailableSource(SourceIdentifier),
    IOError(std::io::Error),
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnavailableSource(source_identifier) => write!(f, "source is not available: {source_identifier:?}"),
            Self::IOError(error) => write!(f, "error loading source: {error}"),
        }
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnavailableSource(_) => None,
            Self::IOError(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for SourceError {
    fn from(error: std::io::Error) -> Self {
        Self::IOError(error)
    }
}
