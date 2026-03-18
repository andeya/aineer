use std::fs;
use std::path::{Path, PathBuf};

const STARTER_CODINEER_JSON: &str = concat!(
    "{\n",
    "  \"permissions\": {\n",
    "    \"defaultMode\": \"dontAsk\"\n",
    "  }\n",
    "}\n",
);
const GITIGNORE_COMMENT: &str = "# Codineer local artifacts";
const GITIGNORE_ENTRIES: [&str; 2] = [".codineer/settings.local.json", ".codineer/sessions/"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InitStatus {
    Created,
    Updated,
    Skipped,
}

impl InitStatus {
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Skipped => "skipped (already exists)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitArtifact {
    pub(crate) name: &'static str,
    pub(crate) status: InitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitReport {
    pub(crate) project_root: PathBuf,
    pub(crate) artifacts: Vec<InitArtifact>,
}

impl InitReport {
    #[must_use]
    pub(crate) fn render(&self) -> String {
        let mut lines = vec![
            "Init".to_string(),
            format!("  Project          {}", self.project_root.display()),
        ];
        for artifact in &self.artifacts {
            lines.push(format!(
                "  {:<16} {}",
                artifact.name,
                artifact.status.label()
            ));
        }
        lines.push("  Next step        Review and tailor the generated guidance".to_string());
        lines.join("\n")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RepoFeature {
    RustWorkspace,
    RustRoot,
    Python,
    PackageJson,
    TypeScript,
    NextJs,
    React,
    Vite,
    NestJs,
    SrcDir,
    TestsDir,
    RustDir,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RepoDetection {
    features: std::collections::HashSet<RepoFeature>,
}

impl RepoDetection {
    fn has(&self, feature: RepoFeature) -> bool {
        self.features.contains(&feature)
    }
}

pub(crate) fn initialize_repo(cwd: &Path) -> Result<InitReport, Box<dyn std::error::Error>> {
    let mut artifacts = Vec::new();

    let config_dir = cwd.join(".codineer");
    artifacts.push(InitArtifact {
        name: ".codineer/",
        status: ensure_dir(&config_dir)?,
    });

    let config_json = cwd.join(".codineer.json");
    artifacts.push(InitArtifact {
        name: ".codineer.json",
        status: write_file_if_missing(&config_json, STARTER_CODINEER_JSON)?,
    });

    let gitignore = cwd.join(".gitignore");
    artifacts.push(InitArtifact {
