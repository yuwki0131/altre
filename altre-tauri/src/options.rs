use std::path::{Path, PathBuf};

/// GUI バックエンド制御のオプション
#[derive(Debug, Clone, Default)]
pub struct BackendOptions {
    /// デバッグログ出力先（未指定時は `~/.altre-log/debug.log`）
    pub debug_log_path: Option<PathBuf>,
    /// 起動時に開くファイルパス
    pub initial_file: Option<PathBuf>,
    /// GUI 実行時のワーキングディレクトリ
    pub working_directory: Option<PathBuf>,
}

impl BackendOptions {
    pub fn resolve_log_path(&self) -> Option<PathBuf> {
        match &self.debug_log_path {
            Some(path) => Some(path.clone()),
            None => default_log_path(),
        }
    }

    pub fn merged_with(&self, overrides: &BackendOptions) -> BackendOptions {
        BackendOptions {
            debug_log_path: overrides
                .debug_log_path
                .clone()
                .or_else(|| self.debug_log_path.clone()),
            initial_file: overrides
                .initial_file
                .clone()
                .or_else(|| self.initial_file.clone()),
            working_directory: overrides
                .working_directory
                .clone()
                .or_else(|| self.working_directory.clone()),
        }
    }
}

fn default_log_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".altre-log").join("debug.log"))
}

/// ヘルパー：親ディレクトリを作成
pub(crate) fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
