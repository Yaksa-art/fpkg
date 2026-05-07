use std::fs;
use fpm_sandbox::{
    overlay::OverlaySandbox,
    sandbox::{SandboxConfig, SandboxLevel},
};

#[test]
fn overlay_dirs_created() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = SandboxConfig::new("test-pkg", SandboxLevel::Overlay);
    cfg.base_dir    = tmp.path().join("test-pkg");
    cfg.overlay_dir = cfg.base_dir.join("overlay");

    fs::create_dir_all(cfg.upper_dir()).unwrap();
    fs::create_dir_all(cfg.work_dir()).unwrap();
    fs::create_dir_all(cfg.merge_dir()).unwrap();
    fs::create_dir_all(cfg.lower_dir()).unwrap();

    assert!(cfg.upper_dir().exists());
    assert!(cfg.work_dir().exists());
    assert!(cfg.merge_dir().exists());
    assert!(cfg.lower_dir().exists());
}

#[test]
fn overlay_removed_on_uninstall() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = SandboxConfig::new("rm-pkg", SandboxLevel::Overlay);
    cfg.base_dir    = tmp.path().join("rm-pkg");
    cfg.overlay_dir = cfg.base_dir.join("overlay");

    fs::create_dir_all(&cfg.overlay_dir).unwrap();
    assert!(cfg.overlay_dir.exists());

    OverlaySandbox::remove_overlay(&cfg).unwrap();
    assert!(!cfg.overlay_dir.exists());
}

#[test]
fn sandbox_level_roundtrip() {
    use std::str::FromStr;
    use fpm_sandbox::sandbox::SandboxLevel;

    assert_eq!(SandboxLevel::from_str("none").unwrap(),    SandboxLevel::None);
    assert_eq!(SandboxLevel::from_str("overlay").unwrap(), SandboxLevel::Overlay);
    assert_eq!(SandboxLevel::from_str("bubble").unwrap(),  SandboxLevel::Bubble);
    assert_eq!(SandboxLevel::from_str("full").unwrap(),    SandboxLevel::Full);
    assert_eq!(SandboxLevel::from_str("1").unwrap(),       SandboxLevel::Overlay);
    assert_eq!(SandboxLevel::from_str("2").unwrap(),       SandboxLevel::Bubble);
}

#[test]
fn sandbox_config_dirs_are_under_base() {
    let mut cfg = SandboxConfig::new("mypkg", SandboxLevel::Bubble);
    cfg.overlay_dir = std::path::PathBuf::from("/tmp/fpm-test/mypkg/overlay");

    assert!(cfg.upper_dir().starts_with(&cfg.overlay_dir));
    assert!(cfg.work_dir().starts_with(&cfg.overlay_dir));
    assert!(cfg.merge_dir().starts_with(&cfg.overlay_dir));
    assert!(cfg.lower_dir().starts_with(&cfg.overlay_dir));
}

#[test]
fn level2_bubble_config_has_no_network_by_default() {
    let cfg = SandboxConfig::new("netpkg", SandboxLevel::Bubble);
    assert!(!cfg.network, "network must be off by default");
}
