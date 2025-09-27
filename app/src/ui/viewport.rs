//! ビューポート管理
//!
//! 画面に表示するテキスト領域のスクロール位置を管理する。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewportManager {
    /// 表示の開始行
    top_line: usize,
    /// 表示可能な行数
    height: usize,
    /// 表示可能な列数
    width: usize,
}

impl ViewportManager {
    pub fn new(height: usize, width: usize) -> Self {
        Self {
            top_line: 0,
            height: height.max(1),
            width,
        }
    }

    /// ビューポートの寸法を更新
    pub fn set_dimensions(&mut self, height: usize, width: usize) {
        self.height = height.max(1);
        self.width = width;
    }

    /// カーソル行が画面内に収まるようスクロールする
    ///
    /// 戻り値はスクロールが発生したかどうか
    pub fn ensure_visible(&mut self, cursor_line: usize) -> bool {
        if cursor_line < self.top_line {
            self.top_line = cursor_line;
            true
        } else {
            let bottom_line = self.top_line + self.height.saturating_sub(1);
            if cursor_line > bottom_line {
                // 中央表示ポリシー（QA Q23）: カーソルを中央に配置
                let half = self.height / 2;
                self.top_line = cursor_line.saturating_sub(half);
                true
            } else {
                false
            }
        }
    }

    /// 現在の表示開始行を取得
    pub fn top_line(&self) -> usize {
        self.top_line
    }

    /// 表示領域の高さを取得
    pub fn height(&self) -> usize {
        self.height
    }

    /// 表示領域の幅を取得
    pub fn width(&self) -> usize {
        self.width
    }
}

impl Default for ViewportManager {
    fn default() -> Self {
        Self::new(1, 0)
    }
}

/// 描画に利用するビューポート状態
#[derive(Debug, Clone)]
pub struct ViewportState {
    /// 表示開始行
    pub top_line: usize,
    /// 水平スクロール位置（文字数単位）
    pub scroll_x: usize,
    /// 表示高さ（行数）
    pub height: usize,
    /// 表示幅（列数）
    pub width: usize,
}

impl ViewportState {
    pub fn new() -> Self {
        Self {
            top_line: 0,
            scroll_x: 0,
            height: 1,
            width: 1,
        }
    }

    /// 利用可能な高さ・幅を更新
    pub fn update_dimensions(&mut self, height: usize, width: usize) {
        self.height = height.max(1);
        self.width = width.max(1);
    }

    /// 全体の行数に合わせて垂直スクロールを補正
    pub fn clamp_vertical(&mut self, total_lines: usize) {
        let total = total_lines.max(1);
        let max_top = total.saturating_sub(self.height);
        if self.top_line > max_top {
            self.top_line = max_top;
        }
    }

    /// コンテンツの最大列数に合わせて水平スクロールを補正
    pub fn clamp_horizontal(&mut self, max_columns: usize) {
        let available = self.width.max(1);
        let max_scroll = max_columns.saturating_sub(available.saturating_sub(1));
        if self.scroll_x > max_scroll {
            self.scroll_x = max_scroll;
        }
    }
}

impl Default for ViewportState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_visible_scrolls_up() {
        let mut viewport = ViewportManager::new(10, 80);
        viewport.ensure_visible(5);
        assert_eq!(viewport.top_line(), 0);

        viewport.ensure_visible(0);
        assert_eq!(viewport.top_line(), 0);
    }

    #[test]
    fn test_ensure_visible_scrolls_down() {
        let mut viewport = ViewportManager::new(4, 80);
        viewport.ensure_visible(10);
        assert!(viewport.top_line() <= 10);
        assert!(viewport.top_line() + viewport.height() - 1 >= 10);
    }

    #[test]
    fn viewport_state_vertical_clamp() {
        let mut state = ViewportState::new();
        state.update_dimensions(5, 80);
        state.top_line = 100;
        state.clamp_vertical(10);
        assert_eq!(state.top_line, 5);
    }

    #[test]
    fn viewport_state_horizontal_clamp() {
        let mut state = ViewportState::new();
        state.update_dimensions(10, 5);
        state.scroll_x = 20;
        state.clamp_horizontal(12);
        assert_eq!(state.scroll_x, 8);
    }
}
