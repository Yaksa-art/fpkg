use std::path::PathBuf;

/// Filesystem layout for fpm — system mode vs user mode.
///
/// System mode (root):
///   /var/lib/fpm/generations/<id>/
///   /var/lib/fpm/current -> <id>   (symlink)
///   /var/cache/fpm/
///   /var/lib/fpm/db.sqlite
///
/// User mode:
///   ~/.local/share/fpm/generations/<id>/
///   ~/.local/share/fpm/current -> <id>
///   ~/.cache/fpm/
///   ~/.local/share/fpm/db.sqlite
#[derive(Debug, Clone)]
pub struct FpmPaths {
    pub lib_dir: PathBuf,    // /var/lib/fpm  or  ~/.local/share/fpm
    pub cache_dir: PathBuf,  // /var/cache/fpm or  ~/.cache/fpm
    pub log_dir: PathBuf,    // /var/log       or  ~/.local/share/fpm/logs
}

impl FpmPaths {
    pub fn system() -> Self {
        Self {
            lib_dir:   PathBuf::from("/var/lib/fpm"),
            cache_dir: PathBuf::from("/var/cache/fpm"),
            log_dir:   PathBuf::from("/var/log"),
        }
    }

    pub fn user() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        let base = PathBuf::from(&home);
        Self {
            lib_dir:   base.join(".local/share/fpm"),
            cache_dir: base.join(".cache/fpm"),
            log_dir:   base.join(".local/share/fpm/logs"),
        }
    }

    /// Root directory for all generations
    pub fn generations_dir(&self) -> PathBuf {
        self.lib_dir.join("generations")
    }

    /// Directory for a specific generation id
    pub fn generation_dir(&self, id: u64) -> PathBuf {
        self.generations_dir().join(id.to_string())
    }

    /// The "pending" generation directory (being built right now)
    pub fn pending_dir(&self) -> PathBuf {
        self.lib_dir.join("pending")
    }

    /// Symlink: current -> <id>
    pub fn current_link(&self) -> PathBuf {
        self.lib_dir.join("current")
    }

    /// SQLite database path (used by M8)
    pub fn db_path(&self) -> PathBuf {
        self.lib_dir.join("db.sqlite")
    }

    /// Log path for transactions
    pub fn trx_log(&self) -> PathBuf {
        self.log_dir.join("fpm-trx.log")
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.generations_dir())?;
        std::fs::create_dir_all(&self.cache_dir)?;
        std::fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }
}
