//! ウィンドウ管理モジュール
//! 
//! 分割ウィンドウとビューポート状態を管理する。

use std::collections::HashMap;

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::buffer::BufferId;
use crate::ui::ViewportState;

/// ウィンドウID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub usize);

/// 分割方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitOrientation {
    /// 上下方向への分割（Emacs の `C-x 2`）
    Horizontal,
    /// 左右方向への分割（Emacs の `C-x 3`）
    Vertical,
}

/// ウィンドウ管理エラー
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WindowError {
    #[error("これ以上ウィンドウを削除できません")] 
    LastWindow,
    #[error("対象ウィンドウが見つかりません")] 
    NotFound,
}

#[derive(Debug, Clone)]
struct WindowState {
    buffer_id: Option<BufferId>,
    viewport: ViewportState,
}

#[derive(Debug, Clone)]
enum LayoutNode {
    Leaf(WindowId),
    Split {
        orientation: SplitOrientation,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

impl LayoutNode {
    fn collect_leaves(&self, leaves: &mut Vec<WindowId>) {
        match self {
            LayoutNode::Leaf(id) => leaves.push(*id),
            LayoutNode::Split { first, second, .. } => {
                first.collect_leaves(leaves);
                second.collect_leaves(leaves);
            }
        }
    }

    fn replace_leaf(&mut self, target: WindowId, replacement: LayoutNode) -> bool {
        match self {
            LayoutNode::Leaf(id) if *id == target => {
                *self = replacement;
                true
            }
            LayoutNode::Leaf(_) => false,
            LayoutNode::Split { first, second, .. } => {
                if first.replace_leaf(target, replacement.clone()) {
                    true
                } else {
                    second.replace_leaf(target, replacement)
                }
            }
        }
    }

    fn remove_leaf(&mut self, target: WindowId) -> bool {
        match self {
            LayoutNode::Leaf(id) => *id == target,
            LayoutNode::Split { first, second, .. } => {
                if first.remove_leaf(target) {
                    // target は first 内にあったので、second を引き上げる
                    *self = (*second.clone()).clone();
                    true
                } else if second.remove_leaf(target) {
                    *self = (*first.clone()).clone();
                    true
                } else {
                    false
                }
            }
        }
    }

    fn layout(&self, area: Rect, result: &mut Vec<(WindowId, Rect)>) {
        match self {
            LayoutNode::Leaf(id) => result.push((*id, area)),
            LayoutNode::Split {
                orientation,
                first,
                second,
            } => {
                let (first_area, _, second_area) = LayoutNode::split_area(area, *orientation);
                first.layout(first_area, result);
                second.layout(second_area, result);
            }
        }
    }

    fn layout_with_dividers(
        &self,
        area: Rect,
        result: &mut Vec<(WindowId, Rect)>,
        dividers: &mut Vec<Rect>,
    ) {
        match self {
            LayoutNode::Leaf(id) => result.push((*id, area)),
            LayoutNode::Split {
                orientation,
                first,
                second,
            } => {
                let (first_area, divider_area, second_area) =
                    LayoutNode::split_area(area, *orientation);

                if let Some(divider) = divider_area {
                    dividers.push(divider);
                }

                first.layout_with_dividers(first_area, result, dividers);
                second.layout_with_dividers(second_area, result, dividers);
            }
        }
    }

    fn split_area(area: Rect, orientation: SplitOrientation) -> (Rect, Option<Rect>, Rect) {
        match orientation {
            SplitOrientation::Horizontal => {
                LayoutNode::split_area_with_direction(area, Direction::Vertical, area.height)
            }
            SplitOrientation::Vertical => {
                LayoutNode::split_area_with_direction(area, Direction::Horizontal, area.width)
            }
        }
    }

    fn split_area_with_direction(
        area: Rect,
        direction: Direction,
        dimension: u16,
    ) -> (Rect, Option<Rect>, Rect) {
        if dimension < 3 {
            let chunks = Layout::default()
                .direction(direction)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(area);
            return (chunks[0], None, chunks[1]);
        }

        let divider_thickness: u16 = 1;
        let remainder = dimension - divider_thickness;

        let mut first = remainder / 2;
        if first == 0 {
            first = 1;
        }
        let mut second = remainder - first;
        if second == 0 {
            second = 1;
            if remainder > 1 {
                first = remainder - second;
            }
        }

        let chunks = Layout::default()
            .direction(direction)
            .constraints([
                Constraint::Length(first),
                Constraint::Length(divider_thickness),
                Constraint::Length(second),
            ])
            .split(area);

        (chunks[0], Some(chunks[1]), chunks[2])
    }
}

/// ウィンドウ管理器
#[derive(Debug, Clone)]
pub struct WindowManager {
    layout: LayoutNode,
    states: HashMap<WindowId, WindowState>,
    focused: WindowId,
    next_id: usize,
}

impl WindowManager {
    /// 新しいウィンドウマネージャーを作成
    pub fn new() -> Self {
        let initial_id = WindowId(0);
        let mut states = HashMap::new();
        states.insert(
            initial_id,
            WindowState {
                buffer_id: None,
                viewport: ViewportState::new(),
            },
        );

        Self {
            layout: LayoutNode::Leaf(initial_id),
            states,
            focused: initial_id,
            next_id: 1,
        }
    }

    /// フォーカス中のウィンドウIDを取得
    pub fn focused_window(&self) -> WindowId {
        self.focused
    }

    /// フォーカスが指定IDかどうか
    pub fn is_focused(&self, id: WindowId) -> bool {
        self.focused == id
    }

    /// ウィンドウ数
    pub fn window_count(&self) -> usize {
        self.states.len()
    }

    /// フォーカス中ウィンドウのビューポートへの可変参照
    pub fn focused_viewport_mut(&mut self) -> Option<&mut ViewportState> {
        self.states.get_mut(&self.focused).map(|state| &mut state.viewport)
    }

    /// フォーカス中ウィンドウのビューポート
    pub fn focused_viewport(&self) -> Option<&ViewportState> {
        self.states.get(&self.focused).map(|state| &state.viewport)
    }

    /// 指定ウィンドウのビューポートへの可変参照
    pub fn viewport_mut(&mut self, id: WindowId) -> Option<&mut ViewportState> {
        self.states.get_mut(&id).map(|state| &mut state.viewport)
    }

    /// 指定ウィンドウのビューポートを取得
    pub fn viewport(&self, id: WindowId) -> Option<&ViewportState> {
        self.states.get(&id).map(|state| &state.viewport)
    }

    /// フォーカス中ウィンドウのバッファID
    pub fn focused_buffer(&self) -> Option<BufferId> {
        self.states.get(&self.focused).and_then(|state| state.buffer_id)
    }

    /// 指定ウィンドウにバッファIDを割り当て
    pub fn set_buffer(&mut self, id: WindowId, buffer_id: Option<BufferId>) {
        if let Some(state) = self.states.get_mut(&id) {
            state.buffer_id = buffer_id;
        }
    }

    /// フォーカス中ウィンドウを分割
    pub fn split_focused(&mut self, orientation: SplitOrientation) -> WindowId {
        let new_id = WindowId(self.next_id);
        self.next_id += 1;

        let cloned_state = self
            .states
            .get(&self.focused)
            .cloned()
            .unwrap_or(WindowState {
                buffer_id: None,
                viewport: ViewportState::new(),
            });
        self.states.insert(
            new_id,
            WindowState {
                buffer_id: cloned_state.buffer_id,
                viewport: cloned_state.viewport,
            },
        );

        let replacement = LayoutNode::Split {
            orientation,
            first: Box::new(LayoutNode::Leaf(self.focused)),
            second: Box::new(LayoutNode::Leaf(new_id)),
        };

        self.layout.replace_leaf(self.focused, replacement);
        new_id
    }

    /// フォーカス中のウィンドウを削除
    pub fn delete_focused(&mut self) -> Result<(), WindowError> {
        if self.states.len() <= 1 {
            return Err(WindowError::LastWindow);
        }

        let target = self.focused;
        self.states.remove(&target);

        if !self.layout.remove_leaf(target) {
            return Err(WindowError::NotFound);
        }

        // 新しいフォーカス先を決定
        let leaves = self.leaf_order();
        if leaves.is_empty() {
            return Err(WindowError::LastWindow);
        }
        self.focused = leaves[0];
        Ok(())
    }

    /// フォーカス以外のウィンドウをすべて削除
    pub fn delete_others(&mut self) -> Result<(), WindowError> {
        let focused = self.focused;
        self.layout = LayoutNode::Leaf(focused);
        self.states.retain(|&id, _| id == focused);
        if self.states.is_empty() {
            return Err(WindowError::LastWindow);
        }
        Ok(())
    }

    /// 次のウィンドウへフォーカスを移動
    pub fn focus_next(&mut self) {
        let leaves = self.leaf_order();
        if leaves.len() <= 1 {
            return;
        }

        if let Some(pos) = leaves.iter().position(|&id| id == self.focused) {
            let next = (pos + 1) % leaves.len();
            self.focused = leaves[next];
        }
    }

    /// レイアウト順のウィンドウID一覧
    pub fn leaf_order(&self) -> Vec<WindowId> {
        let mut leaves = Vec::new();
        self.layout.collect_leaves(&mut leaves);
        leaves
    }

    /// 指定領域内でのウィンドウ矩形を取得
    pub fn layout_rects(&self, area: Rect) -> Vec<(WindowId, Rect)> {
        let mut rects = Vec::new();
        self.layout.layout(area, &mut rects);
        rects
    }

    /// ウィンドウ矩形と区切り線の領域を取得
    pub fn layout_rects_with_dividers(
        &self,
        area: Rect,
    ) -> (Vec<(WindowId, Rect)>, Vec<Rect>) {
        let mut rects = Vec::new();
        let mut dividers = Vec::new();
        self.layout
            .layout_with_dividers(area, &mut rects, &mut dividers);
        (rects, dividers)
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}
