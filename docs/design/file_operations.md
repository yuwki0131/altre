# ファイル操作設計書

## 概要

altreエディタのMVPで実装するファイル操作システムの詳細設計書です。Emacsスタイルのファイル操作インターフェースを提供し、UTF-8エンコーディングとLF改行コードの統一対応を行います。

## 設計方針

### 基本原則
1. **シンプル性優先**: MVPに必要な機能のみを実装
2. **Emacs風操作**: `C-x C-f`、`C-x C-s` 等の標準キーバインド
3. **UTF-8/LF統一**: エンコーディングと改行コードの強制統一
4. **エラー透明性**: エラー原因を明確にユーザーに伝達

### QA回答に基づく仕様
- **バックアップ**: なし（将来alisp設定可能）
- **大きなファイル**: 制限なしで読み込み（将来調整）
- **シンボリックリンク**: 基本対応（リンク先ファイル直接編集）
- **権限不足**: エラー表示（エディタ継続）
- **同時編集検出**: 不要（MVP非対応）

## ファイルオープン操作設計

### 1. `C-x C-f` (find-file) インターフェース

#### 操作フロー
```
1. C-x C-f キー入力
   ↓
2. ミニバッファに "Find file: " プロンプト表示
   ↓
3. ユーザーがファイルパス入力（Tab補完利用可能）
   ↓
4. Enter キーで確定
   ↓
5. ファイル検証・読み込み処理
   ↓
6. 成功: エディタにファイル内容表示
   失敗: エラーメッセージ表示（5秒間）
```

#### パス処理仕様
```rust
pub struct PathProcessor {
    current_directory: PathBuf,
    home_directory: PathBuf,
}

impl PathProcessor {
    /// パス展開・正規化処理
    pub fn expand_path(&self, input: &str) -> Result<PathBuf, FileError> {
        let expanded = match input {
            // 空文字列は現在ディレクトリ
            "" => self.current_directory.clone(),

            // ~/ はホームディレクトリ展開
            path if path.starts_with("~/") => {
                self.home_directory.join(&path[2..])
            }

            // 絶対パス
            path if path.starts_with('/') => {
                PathBuf::from(path)
            }

            // 相対パス
            path => {
                self.current_directory.join(path)
            }
        };

        // パス正規化
        let normalized = expanded.canonicalize()
            .or_else(|_| {
                // ファイルが存在しない場合は親ディレクトリまでを正規化
                if let Some(parent) = expanded.parent() {
                    if parent.exists() {
                        Ok(parent.canonicalize()?.join(expanded.file_name().unwrap()))
                    } else {
                        Err(FileError::InvalidPath { path: input.to_string() })
                    }
                } else {
                    Err(FileError::InvalidPath { path: input.to_string() })
                }
            })?;

        Ok(normalized)
    }
}
```

### 2. ファイル検証・読み込み処理

#### ファイル状態検証
```rust
pub struct FileInfo {
    pub path: PathBuf,
    pub exists: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub is_readable: bool,
    pub is_writable: bool,
    pub size: u64,
    pub modified: SystemTime,
}

impl FileInfo {
    pub fn analyze(path: &Path) -> Result<Self, FileError> {
        let metadata = match path.symlink_metadata() {
            Ok(meta) => meta,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                // 新規ファイル
                return Ok(FileInfo {
                    path: path.to_path_buf(),
                    exists: false,
                    is_file: false,
                    is_symlink: false,
                    is_readable: false,
                    is_writable: Self::can_create_file(path)?,
                    size: 0,
                    modified: SystemTime::UNIX_EPOCH,
                });
            }
            Err(e) => return Err(FileError::Io { source: e }),
        };

        // シンボリックリンク処理（QA Q18: 基本対応）
        let (actual_path, is_symlink) = if metadata.is_symlink() {
            match path.canonicalize() {
                Ok(target) => (target, true),
                Err(_) => return Err(FileError::InvalidPath {
                    path: format!("Broken symlink: {}", path.display())
                }),
            }
        } else {
            (path.to_path_buf(), false)
        };

        // 実際のファイルメタデータ取得
        let file_metadata = actual_path.metadata()
            .map_err(|e| FileError::Io { source: e })?;

        Ok(FileInfo {
            path: actual_path,
            exists: true,
            is_file: file_metadata.is_file(),
            is_symlink,
            is_readable: Self::test_readable(&actual_path),
            is_writable: Self::test_writable(&actual_path),
            size: file_metadata.len(),
            modified: file_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        })
    }

    fn test_readable(path: &Path) -> bool {
        std::fs::File::open(path).is_ok()
    }

    fn test_writable(path: &Path) -> bool {
        std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(path)
            .is_ok()
    }

    fn can_create_file(path: &Path) -> Result<bool, FileError> {
        if let Some(parent) = path.parent() {
            if parent.exists() {
                Ok(Self::test_writable(parent))
            } else {
                Err(FileError::InvalidPath { path: parent.display().to_string() })
            }
        } else {
            Ok(false)
        }
    }
}
```

#### ファイル内容読み込み
```rust
pub struct FileReader {
    max_size: Option<u64>,
}

impl FileReader {
    pub fn read_file(&self, path: &Path) -> Result<String, FileError> {
        let file_info = FileInfo::analyze(path)?;

        // 存在チェック
        if !file_info.exists {
            return Ok(String::new()); // 新規ファイル
        }

        // ファイル種別チェック
        if !file_info.is_file {
            return Err(FileError::InvalidPath {
                path: format!("Not a regular file: {}", path.display())
            });
        }

        // 権限チェック（QA Q19: エラー終了）
        if !file_info.is_readable {
            return Err(FileError::PermissionDenied {
                path: path.display().to_string()
            });
        }

        // ファイル読み込み（QA Q17: 制限なし）
        let content = std::fs::read_to_string(path)
            .map_err(|e| FileError::Io { source: e })?;

        // UTF-8検証（既にread_to_stringで検証済み）
        // 改行コード統一
        let normalized_content = self.normalize_line_endings(&content);

        Ok(normalized_content)
    }

    /// 改行コードをLFに統一
    fn normalize_line_endings(&self, content: &str) -> String {
        // CRLF → LF, CR → LF
        content.replace("\r\n", "\n").replace('\r', "\n")
    }

    /// ファイル内容の検証
    pub fn validate_content(&self, content: &str) -> Result<(), FileError> {
        // UTF-8妥当性チェック（read_to_stringで既に保証）

        // 制御文字チェック（タブと改行以外）
        for (pos, ch) in content.char_indices() {
            if ch.is_control() && ch != '\t' && ch != '\n' {
                log::warn!("Control character found at position {}: {:?}", pos, ch);
                // 警告のみ、エラーにはしない
            }
        }

        Ok(())
    }
}
```

### 3. 新規ファイル作成フロー

#### 新規ファイル処理
```rust
pub struct NewFileHandler;

impl NewFileHandler {
    pub fn handle_new_file(path: &Path) -> Result<String, FileError> {
        // 親ディレクトリの存在確認
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(FileError::InvalidPath {
                    path: format!("Directory does not exist: {}", parent.display())
                });
            }

            // 書き込み権限確認
            if !FileInfo::test_writable(parent) {
                return Err(FileError::PermissionDenied {
                    path: parent.display().to_string()
                });
            }
        }

        // 新規ファイルは空文字列で開始
        Ok(String::new())
    }
}
```

## ファイル保存操作設計

### 1. `C-x C-s` (save-buffer) 機能

#### 保存フロー
```
1. C-x C-s キー入力
   ↓
2. 変更状態チェック
   ↓
3. 変更あり: 保存処理実行
   変更なし: "No changes to save" メッセージ表示
   ↓
4. 保存成功: "Saved: filename" メッセージ表示
   保存失敗: エラーメッセージ表示
```

#### 保存処理実装
```rust
pub struct FileSaver {
    atomic_save: bool,
}

impl FileSaver {
    pub fn save_file(&self, path: &Path, content: &str) -> Result<(), FileError> {
        // バックアップなし（QA Q16の回答）

        // アトミック保存実装
        if self.atomic_save {
            self.atomic_save_impl(path, content)
        } else {
            self.direct_save_impl(path, content)
        }
    }

    /// アトミック保存（一時ファイル経由）
    fn atomic_save_impl(&self, path: &Path, content: &str) -> Result<(), FileError> {
        let temp_path = self.generate_temp_path(path)?;

        // 一時ファイルに書き込み
        std::fs::write(&temp_path, content.as_bytes())
            .map_err(|e| FileError::Io { source: e })?;

        // 原子的にリネーム
        std::fs::rename(&temp_path, path)
            .map_err(|e| {
                // 一時ファイル削除を試行
                let _ = std::fs::remove_file(&temp_path);
                FileError::Io { source: e }
            })?;

        Ok(())
    }

    /// 直接保存
    fn direct_save_impl(&self, path: &Path, content: &str) -> Result<(), FileError> {
        std::fs::write(path, content.as_bytes())
            .map_err(|e| FileError::Io { source: e })
    }

    fn generate_temp_path(&self, original: &Path) -> Result<PathBuf, FileError> {
        let parent = original.parent().ok_or_else(|| FileError::InvalidPath {
            path: original.display().to_string()
        })?;

        let filename = original.file_name().ok_or_else(|| FileError::InvalidPath {
            path: original.display().to_string()
        })?;

        // 一意な一時ファイル名生成
        let temp_name = format!(".{}_{}",
            filename.to_string_lossy(),
            std::process::id()
        );

        Ok(parent.join(temp_name))
    }

    /// 内容の事前検証
    pub fn validate_save_content(&self, content: &str) -> Result<(), FileError> {
        // UTF-8妥当性は保証済み

        // 改行コード統一確認
        if content.contains("\r\n") || content.contains('\r') {
            log::warn!("Non-LF line endings detected, will be normalized");
        }

        Ok(())
    }
}
```

### 2. 変更検出・保存確認

#### 変更状態管理
```rust
pub struct FileChangeTracker {
    original_content: String,
    original_hash: u64,
    last_saved: SystemTime,
}

impl FileChangeTracker {
    pub fn new(content: &str) -> Self {
        Self {
            original_content: content.to_string(),
            original_hash: Self::calculate_hash(content),
            last_saved: SystemTime::now(),
        }
    }

    pub fn is_modified(&self, current_content: &str) -> bool {
        Self::calculate_hash(current_content) != self.original_hash
    }

    pub fn mark_saved(&mut self, content: &str) {
        self.original_content = content.to_string();
        self.original_hash = Self::calculate_hash(content);
        self.last_saved = SystemTime::now();
    }

    fn calculate_hash(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}
```

## パス補完システム設計

### 1. 補完エンジン

#### 基本補完機能
```rust
pub struct PathCompletion {
    max_candidates: usize,
    show_hidden: bool,
}

impl PathCompletion {
    pub fn new() -> Self {
        Self {
            max_candidates: 50, // QA Q8の回答
            show_hidden: false,
        }
    }

    pub fn complete_path(&self, input: &str) -> Result<CompletionResult, FileError> {
        let expanded_path = self.expand_input_path(input)?;

        let (directory, partial_name) = self.split_path_for_completion(&expanded_path);

        let candidates = self.scan_directory(&directory, &partial_name)?;

        Ok(CompletionResult {
            candidates: candidates.into_iter().take(self.max_candidates).collect(),
            common_prefix: self.find_common_prefix(&candidates),
            is_directory_completion: directory.is_dir(),
        })
    }

    fn expand_input_path(&self, input: &str) -> Result<PathBuf, FileError> {
        // ~/ 展開
        if input.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                Ok(home.join(&input[2..]))
            } else {
                Err(FileError::InvalidPath { path: input.to_string() })
            }
        } else if input.starts_with('/') {
            Ok(PathBuf::from(input))
        } else {
            Ok(std::env::current_dir()
                .map_err(|e| FileError::Io { source: e })?
                .join(input))
        }
    }

    fn split_path_for_completion(&self, path: &Path) -> (PathBuf, String) {
        if path.is_dir() {
            (path.to_path_buf(), String::new())
        } else {
            let parent = path.parent()
                .unwrap_or_else(|| Path::new("/"))
                .to_path_buf();
            let filename = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            (parent, filename)
        }
    }

    fn scan_directory(&self, dir: &Path, partial: &str) -> Result<Vec<String>, FileError> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let entries = std::fs::read_dir(dir)
            .map_err(|e| FileError::Io { source: e })?;

        let mut candidates = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| FileError::Io { source: e })?;
            let filename = entry.file_name().to_string_lossy().to_string();

            // 隠しファイルフィルタ
            if !self.show_hidden && filename.starts_with('.') {
                continue;
            }

            // 部分マッチフィルタ
            if !filename.starts_with(partial) {
                continue;
            }

            // ディレクトリには / サフィックス追加
            let display_name = if entry.file_type()
                .map_err(|e| FileError::Io { source: e })?
                .is_dir() {
                format!("{}/", filename)
            } else {
                filename
            };

            candidates.push(display_name);
        }

        // ソート
        candidates.sort();

        Ok(candidates)
    }

    fn find_common_prefix(&self, candidates: &[String]) -> String {
        if candidates.is_empty() {
            return String::new();
        }

        if candidates.len() == 1 {
            return candidates[0].clone();
        }

        let first = &candidates[0];
        let mut common_len = 0;

        for (i, ch) in first.char_indices() {
            if candidates.iter().all(|candidate| {
                candidate.chars().nth(i).map_or(false, |c| c == ch)
            }) {
                common_len = i + ch.len_utf8();
            } else {
                break;
            }
        }

        first[..common_len].to_string()
    }
}

pub struct CompletionResult {
    pub candidates: Vec<String>,
    pub common_prefix: String,
    pub is_directory_completion: bool,
}
```

### 2. ミニバッファ統合

#### 補完表示システム
```rust
pub struct CompletionDisplay {
    max_display_candidates: usize,
    selected_index: Option<usize>,
}

impl CompletionDisplay {
    pub fn format_completion_text(&self, result: &CompletionResult) -> Vec<String> {
        let mut lines = Vec::new();

        if result.candidates.is_empty() {
            lines.push("[No matches]".to_string());
            return lines;
        }

        // 候補数表示
        if result.candidates.len() > self.max_display_candidates {
            lines.push(format!("[{} candidates (showing {})]",
                result.candidates.len(),
                self.max_display_candidates
            ));
        }

        // 候補一覧
        for (i, candidate) in result.candidates.iter()
            .take(self.max_display_candidates)
            .enumerate() {
            let prefix = if Some(i) == self.selected_index {
                "► "
            } else {
                "  "
            };
            lines.push(format!("{}{}", prefix, candidate));
        }

        lines
    }

    pub fn select_next(&mut self, total_candidates: usize) {
        self.selected_index = match self.selected_index {
            Some(i) => Some((i + 1) % total_candidates),
            None => Some(0),
        };
    }

    pub fn select_previous(&mut self, total_candidates: usize) {
        self.selected_index = match self.selected_index {
            Some(i) => Some(if i == 0 { total_candidates - 1 } else { i - 1 }),
            None => Some(total_candidates - 1),
        };
    }

    pub fn get_selected_candidate<'a>(&self, candidates: &'a [String]) -> Option<&'a String> {
        self.selected_index.and_then(|i| candidates.get(i))
    }
}
```

## エラーハンドリング方針

### 1. ファイル操作エラー分類

#### エラー型定義（既存error.rsとの統合）
```rust
// 既存のFileErrorを拡張
impl FileError {
    /// ユーザーフレンドリーなエラーメッセージ生成
    pub fn user_friendly_message(&self) -> String {
        match self {
            FileError::NotFound { path } => {
                format!("ファイルが見つかりません: {}", path)
            }
            FileError::PermissionDenied { path } => {
                format!("ファイルへのアクセス権限がありません: {}", path)
            }
            FileError::InvalidPath { path } => {
                format!("無効なパスです: {}", path)
            }
            FileError::Encoding { message } => {
                format!("ファイルエンコーディングエラー: {}", message)
            }
            FileError::Io { source } => {
                match source.kind() {
                    ErrorKind::NotFound => "ファイルまたはディレクトリが見つかりません".to_string(),
                    ErrorKind::PermissionDenied => "アクセス権限がありません".to_string(),
                    ErrorKind::AlreadyExists => "ファイルが既に存在します".to_string(),
                    ErrorKind::InvalidInput => "無効な入力です".to_string(),
                    ErrorKind::InvalidData => "ファイルデータが無効です".to_string(),
                    ErrorKind::TimedOut => "操作がタイムアウトしました".to_string(),
                    ErrorKind::WriteZero => "書き込みに失敗しました".to_string(),
                    ErrorKind::Interrupted => "操作が中断されました".to_string(),
                    ErrorKind::UnexpectedEof => "ファイルが予期せず終了しています".to_string(),
                    _ => format!("ファイル操作エラー: {}", source),
                }
            }
        }
    }

    /// 復旧可能性の判定
    pub fn is_recoverable(&self) -> bool {
        match self {
            FileError::NotFound { .. } => true,  // 新規作成可能
            FileError::PermissionDenied { .. } => false, // 権限変更が必要
            FileError::InvalidPath { .. } => true,       // パス修正可能
            FileError::Encoding { .. } => false,         // ファイル形式問題
            FileError::Io { source } => {
                match source.kind() {
                    ErrorKind::NotFound => true,
                    ErrorKind::PermissionDenied => false,
                    ErrorKind::AlreadyExists => true,
                    ErrorKind::InvalidInput => true,
                    ErrorKind::InvalidData => false,
                    ErrorKind::TimedOut => true,
                    ErrorKind::WriteZero => false,
                    ErrorKind::Interrupted => true,
                    ErrorKind::UnexpectedEof => false,
                    _ => false,
                }
            }
        }
    }
}
```

### 2. エラー処理フロー

#### ファイル操作失敗時の処理
```rust
pub struct FileOperationHandler {
    error_display: ErrorDisplay,
}

impl FileOperationHandler {
    pub fn handle_file_open_error(&mut self, error: FileError, attempted_path: &str) {
        let message = match &error {
            FileError::NotFound { .. } => {
                format!("新規ファイルとして作成しますか？ {}", attempted_path)
            }
            FileError::PermissionDenied { .. } => {
                // QA Q19: エラー表示、エディタ継続
                format!("読み取り権限がありません: {}", attempted_path)
            }
            _ => error.user_friendly_message(),
        };

        // エラーメッセージをミニバッファに表示（5秒間）
        self.error_display.show_error(message);
    }

    pub fn handle_file_save_error(&mut self, error: FileError, path: &str) {
        let message = error.user_friendly_message();

        // 保存失敗の場合は詳細なエラー情報を提供
        let detailed_message = format!("保存に失敗しました: {}", message);

        self.error_display.show_error(detailed_message);
    }

    pub fn handle_completion_error(&mut self, error: FileError) {
        // 補完エラーは控えめに表示
        log::debug!("Path completion error: {:?}", error);
        // ユーザーには補完候補なしとして表示（エラーメッセージは出さない）
    }
}
```

## UTF-8/LF強制仕様

### 1. エンコーディング処理

#### UTF-8強制変換
```rust
pub struct EncodingProcessor;

impl EncodingProcessor {
    /// ファイル読み込み時のUTF-8検証・変換
    pub fn process_file_content(raw_content: &[u8]) -> Result<String, FileError> {
        // UTF-8として解釈を試行
        match std::str::from_utf8(raw_content) {
            Ok(content) => Ok(content.to_string()),
            Err(utf8_error) => {
                // UTF-8でない場合はエラー
                Err(FileError::Encoding {
                    message: format!("ファイルはUTF-8エンコーディングである必要があります: {}", utf8_error)
                })
            }
        }
    }

    /// 保存時のUTF-8変換
    pub fn prepare_save_content(content: &str) -> Vec<u8> {
        // 既にUTF-8文字列なのでそのままバイト列に変換
        content.as_bytes().to_vec()
    }

    /// BOM除去処理
    pub fn remove_bom(content: &str) -> &str {
        // UTF-8 BOM (EF BB BF) を除去
        if content.starts_with('\u{FEFF}') {
            &content[3..]
        } else {
            content
        }
    }
}
```

### 2. 改行コード処理

#### LF統一システム
```rust
pub struct LineEndingProcessor;

impl LineEndingProcessor {
    /// 改行コードをLFに統一
    pub fn normalize_to_lf(content: &str) -> String {
        // CRLF (\r\n) を LF (\n) に変換
        let step1 = content.replace("\r\n", "\n");

        // 残りの CR (\r) を LF (\n) に変換
        step1.replace('\r', "\n")
    }

    /// 改行コード検出
    pub fn detect_line_endings(content: &str) -> LineEndingStyle {
        let has_crlf = content.contains("\r\n");
        let has_lf = content.contains('\n');
        let has_cr = content.contains('\r');

        match (has_crlf, has_lf, has_cr) {
            (true, _, _) => LineEndingStyle::Crlf,
            (false, true, false) => LineEndingStyle::Lf,
            (false, false, true) => LineEndingStyle::Cr,
            (false, true, true) => LineEndingStyle::Mixed,
            (false, false, false) => LineEndingStyle::None,
        }
    }

    /// 保存時のLF確認
    pub fn ensure_lf_endings(content: &str) -> String {
        // 常にLFに統一
        Self::normalize_to_lf(content)
    }
}

#[derive(Debug, PartialEq)]
pub enum LineEndingStyle {
    Lf,     // \n (Unix)
    Crlf,   // \r\n (Windows)
    Cr,     // \r (Classic Mac)
    Mixed,  // 混在
    None,   // 改行なし
}
```

## ファイル管理システム

### 1. バッファとファイルの関連付け

#### ファイルバッファ管理
```rust
pub struct FileBuffer {
    /// ファイルパス
    pub path: Option<PathBuf>,
    /// バッファ名
    pub name: String,
    /// テキスト内容
    pub content: String,
    /// 変更追跡
    pub change_tracker: FileChangeTracker,
    /// ファイル情報
    pub file_info: Option<FileInfo>,
    /// 読み取り専用フラグ
    pub read_only: bool,
}

impl FileBuffer {
    /// ファイルから新しいバッファを作成
    pub fn from_file(path: PathBuf) -> Result<Self, FileError> {
        let file_info = FileInfo::analyze(&path)?;

        let content = if file_info.exists {
            FileReader::new().read_file(&path)?
        } else {
            String::new()
        };

        let normalized_content = LineEndingProcessor::normalize_to_lf(&content);

        Ok(FileBuffer {
            name: Self::generate_buffer_name(&path),
            path: Some(path),
            content: normalized_content.clone(),
            change_tracker: FileChangeTracker::new(&normalized_content),
            file_info: Some(file_info),
            read_only: false,
        })
    }

    /// 新規バッファを作成
    pub fn new_empty(name: String) -> Self {
        FileBuffer {
            name,
            path: None,
            content: String::new(),
            change_tracker: FileChangeTracker::new(""),
            file_info: None,
            read_only: false,
        }
    }

    /// バッファ名生成
    fn generate_buffer_name(path: &Path) -> String {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    /// 変更状態チェック
    pub fn is_modified(&self) -> bool {
        self.change_tracker.is_modified(&self.content)
    }

    /// 保存処理
    pub fn save(&mut self) -> Result<(), FileError> {
        let path = self.path.as_ref().ok_or_else(|| FileError::InvalidPath {
            path: "No file associated with buffer".to_string()
        })?;

        // LF改行コード統一
        let save_content = LineEndingProcessor::ensure_lf_endings(&self.content);

        // 保存実行
        FileSaver::new().save_file(path, &save_content)?;

        // 変更状態リセット
        self.change_tracker.mark_saved(&save_content);

        Ok(())
    }

    /// ファイル情報更新
    pub fn refresh_file_info(&mut self) -> Result<(), FileError> {
        if let Some(path) = &self.path {
            self.file_info = Some(FileInfo::analyze(path)?);
        }
        Ok(())
    }
}
```

### 2. メタデータ管理

#### ファイルメタデータ追跡
```rust
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub permissions: std::fs::Permissions,
    pub is_symlink: bool,
    pub encoding: String,
    pub line_ending: LineEndingStyle,
}

impl FileMetadata {
    pub fn from_file(path: &Path) -> Result<Self, FileError> {
        let metadata = path.metadata()
            .map_err(|e| FileError::Io { source: e })?;

        let symlink_metadata = path.symlink_metadata()
            .map_err(|e| FileError::Io { source: e })?;

        Ok(FileMetadata {
            path: path.to_path_buf(),
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            permissions: metadata.permissions(),
            is_symlink: symlink_metadata.is_symlink(),
            encoding: "UTF-8".to_string(), // 強制UTF-8
            line_ending: LineEndingStyle::Lf, // 強制LF
        })
    }

    pub fn has_changed_externally(&self) -> Result<bool, FileError> {
        if !self.path.exists() {
            return Ok(true); // ファイルが削除された
        }

        let current_metadata = FileMetadata::from_file(&self.path)?;
        Ok(current_metadata.modified != self.modified ||
           current_metadata.size != self.size)
    }
}
```

## 統合例：完全なファイル操作フロー

### ファイルオープンからエディタ表示まで
```rust
pub struct FileOperationManager {
    path_processor: PathProcessor,
    file_reader: FileReader,
    completion_engine: PathCompletion,
    error_handler: FileOperationHandler,
}

impl FileOperationManager {
    pub fn open_file(&mut self, input_path: &str) -> Result<FileBuffer, FileError> {
        // 1. パス展開・正規化
        let normalized_path = self.path_processor.expand_path(input_path)
            .map_err(|e| {
                self.error_handler.handle_file_open_error(e.clone(), input_path);
                e
            })?;

        // 2. ファイル情報分析
        let file_info = FileInfo::analyze(&normalized_path)
            .map_err(|e| {
                self.error_handler.handle_file_open_error(e.clone(), input_path);
                e
            })?;

        // 3. 権限チェック（QA Q19）
        if file_info.exists && !file_info.is_readable {
            let error = FileError::PermissionDenied {
                path: normalized_path.display().to_string()
            };
            self.error_handler.handle_file_open_error(error.clone(), input_path);
            return Err(error);
        }

        // 4. ファイル読み込みまたは新規作成
        let buffer = FileBuffer::from_file(normalized_path)
            .map_err(|e| {
                self.error_handler.handle_file_open_error(e.clone(), input_path);
                e
            })?;

        Ok(buffer)
    }

    pub fn save_current_buffer(&mut self, buffer: &mut FileBuffer) -> Result<(), FileError> {
        // 1. 変更チェック
        if !buffer.is_modified() {
            // 変更なしメッセージ
            return Ok(());
        }

        // 2. 保存実行
        buffer.save().map_err(|e| {
            if let Some(path) = &buffer.path {
                self.error_handler.handle_file_save_error(e.clone(), &path.display().to_string());
            }
            e
        })?;

        Ok(())
    }

    pub fn complete_path(&mut self, input: &str) -> CompletionResult {
        self.completion_engine.complete_path(input)
            .unwrap_or_else(|e| {
                self.error_handler.handle_completion_error(e);
                CompletionResult {
                    candidates: Vec::new(),
                    common_prefix: String::new(),
                    is_directory_completion: false,
                }
            })
    }
}
```

## テスト戦略

### 単体テスト仕様
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_path_expansion() {
        let processor = PathProcessor::new();

        // ホームディレクトリ展開
        if let Some(home) = dirs::home_dir() {
            let expanded = processor.expand_path("~/test.txt").unwrap();
            assert_eq!(expanded, home.join("test.txt"));
        }

        // 相対パス
        let current = std::env::current_dir().unwrap();
        let expanded = processor.expand_path("test.txt").unwrap();
        assert_eq!(expanded, current.join("test.txt"));
    }

    #[test]
    fn test_line_ending_normalization() {
        assert_eq!(
            LineEndingProcessor::normalize_to_lf("hello\r\nworld\r\ntest"),
            "hello\nworld\ntest"
        );

        assert_eq!(
            LineEndingProcessor::normalize_to_lf("hello\rworld\rtest"),
            "hello\nworld\ntest"
        );

        assert_eq!(
            LineEndingProcessor::normalize_to_lf("hello\nworld\ntest"),
            "hello\nworld\ntest"
        );
    }

    #[test]
    fn test_file_operations_qa_compliance() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // QA Q16: バックアップなし
        let saver = FileSaver::new();
        saver.save_file(&test_file, "test content").unwrap();

        // バックアップファイルが作成されていないことを確認
        assert!(!temp_dir.path().join("test.txt~").exists());
        assert!(!temp_dir.path().join("test.txt.bak").exists());

        // QA Q18: シンボリックリンク基本対応
        let link_file = temp_dir.path().join("link.txt");
        std::os::unix::fs::symlink(&test_file, &link_file).unwrap();

        let file_info = FileInfo::analyze(&link_file).unwrap();
        assert!(file_info.is_symlink);
        assert_eq!(file_info.path, test_file); // リンク先パスが取得される
    }
}
```

## 将来の拡張計画

### フェーズ2: 高度なファイル操作
- バックアップファイル機能（alisp設定可能）
- 大きなファイル用プログレス表示
- 非同期ファイルI/O
- ファイル変更監視（inotify）

### フェーズ3: 高度な補完
- ファジー補完
- 最近使用ファイル履歴
- プロジェクトファイル認識
- 複数エンコーディング対応

### フェーズ4: 高度なファイル管理
- 同時編集検出・警告
- ファイルロック機能
- バックアップ戦略選択
- クラウドストレージ統合

この設計により、MVP段階での実用的なファイル操作と将来の高度な機能への拡張性を両立させる。