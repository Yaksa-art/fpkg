use serde::{Deserialize, Serialize};
use fpm_solver::ResolvedPackage;
use fpm_fetcher::FetchResult;

/// What operation a plan entry represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanOp {
    Install,
    Remove,
    Upgrade { from_version: String },
    Reinstall,
    /// No change needed (already installed at correct version)
    AlreadyInstalled,
}

/// One package action in the install plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanEntry {
    pub name: String,
    pub version: String,
    pub op: PlanOp,
    /// Path to the verified .fpkg (set after M2 Fetcher completes; None for Remove)
    pub fpkg_path: Option<std::path::PathBuf>,
    /// BLAKE3 of the .fpkg
    pub blake3: Option<String>,
    /// Total installed size in bytes (from manifest)
    pub installed_size: Option<u64>,
}

/// The full plan for one `fpm install` / `fpm remove` / `fpm upgrade` invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallPlan {
    pub entries: Vec<PlanEntry>,
    /// Human description shown to the user before confirmation
    pub description: String,
}

impl InstallPlan {
    /// Build a plan from M1 resolved packages and M2 fetch results.
    ///
    /// Packages already installed at the correct version are marked AlreadyInstalled.
    /// `installed` is a map of currently installed name → version strings.
    pub fn from_resolved(
        resolved: &[ResolvedPackage],
        fetch_results: &[FetchResult],
        installed: &std::collections::HashMap<String, String>,
        description: impl Into<String>,
    ) -> Self {
        let fetch_map: std::collections::HashMap<&str, &FetchResult> = fetch_results
            .iter()
            .map(|r| (r.package.as_str(), r))
            .collect();

        let entries = resolved
            .iter()
            .map(|pkg| {
                let ver = pkg.version.to_string();
                let op = match installed.get(&pkg.name) {
                    None => PlanOp::Install,
                    Some(existing) if existing == &ver => PlanOp::AlreadyInstalled,
                    Some(existing) => PlanOp::Upgrade { from_version: existing.clone() },
                };
                let fetch = fetch_map.get(pkg.name.as_str());
                PlanEntry {
                    name: pkg.name.clone(),
                    version: ver,
                    op,
                    fpkg_path: fetch.map(|f| f.path.clone()),
                    blake3: pkg.blake3.clone(),
                    installed_size: None,
                }
            })
            .collect();

        Self { entries, description: description.into() }
    }

    /// Packages that need actual disk work (not AlreadyInstalled).
    pub fn actionable(&self) -> impl Iterator<Item = &PlanEntry> {
        self.entries.iter().filter(|e| e.op != PlanOp::AlreadyInstalled)
    }

    /// Download size summary string for user confirmation prompt.
    pub fn summary(&self) -> String {
        let install: Vec<_> = self.entries.iter()
            .filter(|e| matches!(e.op, PlanOp::Install))
            .collect();
        let upgrade: Vec<_> = self.entries.iter()
            .filter(|e| matches!(e.op, PlanOp::Upgrade { .. }))
            .collect();
        let skip: Vec<_> = self.entries.iter()
            .filter(|e| e.op == PlanOp::AlreadyInstalled)
            .collect();

        let mut lines = vec![];
        for e in &install {
            lines.push(format!("  {} {}  [new]", e.name, e.version));
        }
        for e in &upgrade {
            if let PlanOp::Upgrade { from_version } = &e.op {
                lines.push(format!("  {} {} -> {}  [upgrade]", e.name, from_version, e.version));
            }
        }
        for e in &skip {
            lines.push(format!("  {} {}  [already installed]", e.name, e.version));
        }
        lines.join("\n")
    }
}
