use std::process::Command;

use aineer_engine::{LspManager, LspServerConfig};

pub(crate) fn detect_lsp_servers(workspace_root: &std::path::Path) -> Option<LspManager> {
    let mut configs = Vec::new();

    add_lsp_if_found(
        &mut configs,
        workspace_root,
        "rust-analyzer",
        "rust-analyzer",
        &[],
        &[(".rs", "rust")],
    );
    add_lsp_if_found(
        &mut configs,
        workspace_root,
        "typescript-language-server",
        "typescript-language-server",
        &["--stdio"],
        &[
            (".ts", "typescript"),
            (".tsx", "typescriptreact"),
            (".js", "javascript"),
            (".jsx", "javascriptreact"),
        ],
    );
    add_lsp_if_found(
        &mut configs,
        workspace_root,
        "pyright",
        "pyright-langserver",
        &["--stdio"],
        &[(".py", "python")],
    );
    add_lsp_if_found(
        &mut configs,
        workspace_root,
        "gopls",
        "gopls",
        &["serve"],
        &[(".go", "go")],
    );

    if configs.is_empty() {
        return None;
    }
    LspManager::new(configs).ok()
}

fn add_lsp_if_found(
    configs: &mut Vec<LspServerConfig>,
    workspace_root: &std::path::Path,
    name: &str,
    command: &str,
    args: &[&str],
    extensions: &[(&str, &str)],
) {
    use std::collections::BTreeMap;
    if which_command(command) {
        configs.push(LspServerConfig {
            name: name.to_string(),
            command: command.to_string(),
            args: args.iter().map(|a| (*a).to_string()).collect(),
            env: BTreeMap::new(),
            workspace_root: workspace_root.to_path_buf(),
            initialization_options: None,
            extension_to_language: extensions
                .iter()
                .map(|(ext, lang)| ((*ext).to_string(), (*lang).to_string()))
                .collect(),
        });
    }
}

fn which_command(name: &str) -> bool {
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
    #[cfg(not(windows))]
    {
        Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {name} >/dev/null 2>&1"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }
}
