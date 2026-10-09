use std::{error::Error, fmt};

/// An error encountered while loading normalized static game data.
#[derive(Debug)]
pub struct DexLoadError {
    source: serde_json::Error,
}

impl DexLoadError {
    pub(crate) const fn new(source: serde_json::Error) -> Self {
        Self { source }
    }
}

impl fmt::Display for DexLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid generated Dex data: {}", self.source)
    }
}

impl Error for DexLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
