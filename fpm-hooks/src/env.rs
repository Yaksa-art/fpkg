use std::path::Path;

/// Environment variables passed into every hook script.
pub struct HookEnv {
    pub fpkg_root: String,
    pub fpkg_name: String,
    pub fpkg_version: String,
    pub fpkg_hook: String,
}

impl HookEnv {
    pub fn new(root: &Path, name: &str, version: &str, hook: &str) -> Self {
        Self {
            fpkg_root: root.to_string_lossy().into_owned(),
            fpkg_name: name.to_string(),
            fpkg_version: version.to_string(),
            fpkg_hook: hook.to_string(),
        }
    }

    pub fn as_pairs(&self) -> Vec<(&str, &str)> {
        vec![
            ("FPKG_ROOT", &self.fpkg_root),
            ("FPKG_NAME", &self.fpkg_name),
            ("FPKG_VERSION", &self.fpkg_version),
            ("FPKG_HOOK", &self.fpkg_hook),
            ("PATH", "/usr/bin:/bin"),
            ("HOME", "/root"),
        ]
    }
}
