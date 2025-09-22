//! バッファ管理モジュール
//!
//! テキストデータの管理、編集操作、カーソル位置管理を提供

pub mod gap_buffer;
pub mod cursor;
pub mod operations;
pub mod editor;

// 公開API
pub use gap_buffer::GapBuffer;
pub use cursor::CursorPosition;
pub use operations::EditOperation;
pub use editor::{TextEditor, EditOperations, ChangeEvent};
pub use crate::error::EditError;

use crate::error::Result;
use std::collections::HashMap;
use std::path::PathBuf;

/// バッファの一意識別子
pub type BufferId = usize;

/// 単一のテキストバッファ
pub struct Buffer {
    /// バッファ内容（ギャップバッファ）
    pub content: GapBuffer,
    /// カーソル位置
    pub cursor: CursorPosition,
    /// 関連ファイルパス（任意）
    pub file_path: Option<PathBuf>,
    /// 変更フラグ
    pub modified: bool,
}

impl Buffer {
    /// 新しい空のバッファを作成
    pub fn new() -> Self {
        Self {
            content: GapBuffer::new(),
            cursor: CursorPosition::new(),
            file_path: None,
            modified: false,
        }
    }

    /// ファイルパス付きで新しいバッファを作成
    pub fn with_file(path: PathBuf) -> Self {
        Self {
            content: GapBuffer::new(),
            cursor: CursorPosition::new(),
            file_path: Some(path),
            modified: false,
        }
    }

    /// バッファが変更されているかを確認
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// 変更フラグをセット
    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

/// 複数のバッファを管理する構造体
pub struct BufferManager {
    /// バッファの格納
    buffers: HashMap<BufferId, Buffer>,
    /// 現在アクティブなバッファID
    current_buffer: Option<BufferId>,
    /// 次に割り当てるバッファID
    next_id: BufferId,
}

impl BufferManager {
    /// 新しいバッファマネージャーを作成
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            current_buffer: None,
            next_id: 0,
        }
    }

    /// 新しいバッファを作成し、IDを返す
    pub fn create_buffer(&mut self) -> BufferId {
        let id = self.next_id;
        self.next_id += 1;

        let buffer = Buffer::new();
        self.buffers.insert(id, buffer);

        // 最初のバッファは自動的に現在のバッファにする
        if self.current_buffer.is_none() {
            self.current_buffer = Some(id);
        }

        id
    }

    /// 現在のバッファIDを取得
    pub fn current_buffer_id(&self) -> Option<BufferId> {
        self.current_buffer
    }

    /// 現在のバッファへの参照を取得
    pub fn current_buffer(&self) -> Option<&Buffer> {
        self.current_buffer
            .and_then(|id| self.buffers.get(&id))
    }

    /// 現在のバッファへの可変参照を取得
    pub fn current_buffer_mut(&mut self) -> Option<&mut Buffer> {
        self.current_buffer
            .and_then(|id| self.buffers.get_mut(&id))
    }

    /// 指定されたIDのバッファへの参照を取得
    pub fn get_buffer(&self, id: BufferId) -> Option<&Buffer> {
        self.buffers.get(&id)
    }

    /// 指定されたIDのバッファへの可変参照を取得
    pub fn get_buffer_mut(&mut self, id: BufferId) -> Option<&mut Buffer> {
        self.buffers.get_mut(&id)
    }

    /// 現在のバッファを切り替え
    pub fn switch_to_buffer(&mut self, id: BufferId) -> Result<()> {
        if self.buffers.contains_key(&id) {
            self.current_buffer = Some(id);
            Ok(())
        } else {
            Err(crate::error::AltreError::Buffer(
                crate::error::BufferError::Empty
            ))
        }
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}