use std::fmt;

const RELEASE_CHANNEL: &str = include_str!("../../../RELEASE_CHANNEL");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReleaseChannel {
    Dev,
    Nightly,
    Preview,
    Stable,
}

impl ReleaseChannel {
    pub fn current() -> Self {
        match RELEASE_CHANNEL.trim() {
            "nightly" => Self::Nightly,
            "preview" => Self::Preview,
            "stable" => Self::Stable,
            _ => Self::Dev,
        }
    }

    pub fn is_dev(self) -> bool {
        self == Self::Dev
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Dev => "Aineer Dev",
            Self::Nightly => "Aineer Nightly",
            Self::Preview => "Aineer Preview",
            Self::Stable => "Aineer",
        }
    }

    /// Version suffix for display strings, e.g. "-dev", "-nightly", "" for stable.
    pub fn version_suffix(self) -> &'static str {
        match self {
            Self::Dev => "-dev",
            Self::Nightly => "-nightly",
            Self::Preview => "-preview",
            Self::Stable => "",
        }
    }

    pub fn app_id(self) -> &'static str {
        match self {
            Self::Dev => "dev.aineer.Aineer-Dev",
            Self::Nightly => "dev.aineer.Aineer-Nightly",
            Self::Preview => "dev.aineer.Aineer-Preview",
            Self::Stable => "dev.aineer.Aineer",
        }
    }
}

impl fmt::Display for ReleaseChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Nightly => write!(f, "nightly"),
            Self::Preview => write!(f, "preview"),
            Self::Stable => write!(f, "stable"),
        }
    }
}
