pub(crate) const EXTERNAL_MARKETPLACE: &str = "external";
pub(crate) const BUILTIN_MARKETPLACE: &str = "builtin";
pub(crate) const BUNDLED_MARKETPLACE: &str = "bundled";
pub(crate) const SETTINGS_FILE_NAME: &str = "settings.json";
pub(crate) const REGISTRY_FILE_NAME: &str = "installed.json";
pub(crate) const MANIFEST_FILE_NAME: &str = "plugin.json";
pub(crate) const MANIFEST_RELATIVE_PATH: &str = ".codineer-plugin/plugin.json";

pub(crate) fn is_literal_command(entry: &str) -> bool {
    !entry.starts_with("./")
        && !entry.starts_with("../")
        && !std::path::Path::new(entry).is_absolute()
}
