//! テーマシステム
//!
//! カラー設定の管理、ダークモード対応、カスタマイズ可能なスタイル

use ratatui::{
    style::{Color, Modifier, Style},
    widgets::BorderType,
};
use std::collections::HashMap;

/// テーマの種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ThemeType {
    Light,
    Dark,
    HighContrast,
    Custom(String),
}

/// UIコンポーネントの種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ComponentType {
    /// テキストエリア
    TextArea,
    /// 行番号
    LineNumber,
    /// カーソル
    Cursor,
    /// 選択範囲
    Selection,
    /// ミニバッファ
    Minibuffer,
    /// ステータスライン
    StatusLine,
    /// ボーダー
    Border,
    /// エラーメッセージ
    Error,
    /// 警告メッセージ
    Warning,
    /// 情報メッセージ
    Info,
    /// 補完候補
    Completion,
    /// 選択された補完候補
    CompletionSelected,
    /// ウィンドウ間の区切り
    WindowDivider,
    /// シンタックスハイライト - キーワード
    SyntaxKeyword,
    /// シンタックスハイライト - 文字列
    SyntaxString,
    /// シンタックスハイライト - コメント
    SyntaxComment,
    /// シンタックスハイライト - 数値
    SyntaxNumber,
    /// シンタックスハイライト - 演算子
    SyntaxOperator,
}

/// カラー設定
#[derive(Debug, Clone)]
pub struct ColorScheme {
    /// 前景色
    pub foreground: Color,
    /// 背景色
    pub background: Color,
    /// 修飾子（太字、下線など）
    pub modifiers: Modifier,
}

impl ColorScheme {
    pub fn new(foreground: Color, background: Color) -> Self {
        Self {
            foreground,
            background,
            modifiers: Modifier::empty(),
        }
    }

    pub fn with_modifier(mut self, modifier: Modifier) -> Self {
        self.modifiers = modifier;
        self
    }

    pub fn to_style(&self) -> Style {
        Style::default()
            .fg(self.foreground)
            .bg(self.background)
            .add_modifier(self.modifiers)
    }
}

/// テーマ設定
#[derive(Debug, Clone)]
pub struct Theme {
    /// テーマ名
    pub name: String,
    /// テーマの種類
    pub theme_type: ThemeType,
    /// コンポーネント別のカラー設定
    pub colors: HashMap<ComponentType, ColorScheme>,
    /// ボーダータイプ
    pub border_type: BorderType,
    /// 日本語文字幅対応
    pub japanese_support: bool,
    /// 16色制限モード
    pub color_16_mode: bool,
}

impl Theme {
    /// 新しいテーマを作成
    pub fn new(name: String, theme_type: ThemeType) -> Self {
        let mut theme = Self {
            name,
            theme_type: theme_type.clone(),
            colors: HashMap::new(),
            border_type: BorderType::Rounded,
            japanese_support: true,
            color_16_mode: false,
        };

        // デフォルトカラーを設定
        theme.set_default_colors(&theme_type);
        theme
    }

    /// 特定のコンポーネントのスタイルを取得
    pub fn style(&self, component: &ComponentType) -> Style {
        self.colors
            .get(component)
            .map(|cs| cs.to_style())
            .unwrap_or_else(|| self.default_style())
    }

    /// カラー設定を追加
    pub fn set_color(&mut self, component: ComponentType, color_scheme: ColorScheme) {
        self.colors.insert(component, color_scheme);
    }

    /// デフォルトスタイルを取得
    pub fn default_style(&self) -> Style {
        match self.theme_type {
            ThemeType::Light => Style::default().fg(Color::Black).bg(Color::White),
            ThemeType::Dark => Style::default().fg(Color::White).bg(Color::Black),
            ThemeType::HighContrast => Style::default().fg(Color::White).bg(Color::Black),
            ThemeType::Custom(_) => Style::default().fg(Color::White).bg(Color::Black),
        }
    }

    /// 16色モードのカラーに変換
    pub fn to_16_color(&self, color: Color) -> Color {
        if !self.color_16_mode {
            return color;
        }

        match color {
            Color::Rgb(r, g, b) => {
                // RGB値を16色パレットにマッピング
                if r > 200 && g > 200 && b > 200 {
                    Color::White
                } else if r < 50 && g < 50 && b < 50 {
                    Color::Black
                } else if r > g && r > b {
                    Color::Red
                } else if g > r && g > b {
                    Color::Green
                } else if b > r && b > g {
                    Color::Blue
                } else if r > 100 && g > 100 {
                    Color::Yellow
                } else if r > 100 && b > 100 {
                    Color::Magenta
                } else if g > 100 && b > 100 {
                    Color::Cyan
                } else {
                    Color::Gray
                }
            }
            _ => color,
        }
    }

    /// 日本語文字幅を考慮したスタイル調整
    pub fn adjust_for_japanese(&self, style: Style) -> Style {
        if !self.japanese_support {
            return style;
        }

        // 日本語文字の表示では太字を避ける（表示が崩れることがある）
        if style.add_modifier.contains(Modifier::BOLD) {
            style.remove_modifier(Modifier::BOLD)
        } else {
            style
        }
    }

    /// デフォルトカラーを設定
    fn set_default_colors(&mut self, theme_type: &ThemeType) {
        match theme_type {
            ThemeType::Light => self.set_light_colors(),
            ThemeType::Dark => self.set_dark_colors(),
            ThemeType::HighContrast => self.set_high_contrast_colors(),
            ThemeType::Custom(_) => self.set_dark_colors(), // デフォルトはダークテーマ
        }
    }

    fn set_light_colors(&mut self) {
        // ライトテーマのカラー設定
        self.set_color(ComponentType::TextArea,
            ColorScheme::new(Color::Black, Color::White));
        self.set_color(ComponentType::LineNumber,
            ColorScheme::new(Color::DarkGray, Color::Gray));
        self.set_color(ComponentType::Cursor,
            ColorScheme::new(Color::White, Color::Blue));
        self.set_color(ComponentType::Selection,
            ColorScheme::new(Color::White, Color::Blue));
        self.set_color(ComponentType::Minibuffer,
            ColorScheme::new(Color::Black, Color::Gray));
        self.set_color(ComponentType::StatusLine,
            ColorScheme::new(Color::White, Color::Blue));
        self.set_color(ComponentType::Border,
            ColorScheme::new(Color::DarkGray, Color::White));
        self.set_color(ComponentType::Error,
            ColorScheme::new(Color::Red, Color::White).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Warning,
            ColorScheme::new(Color::Yellow, Color::White).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Info,
            ColorScheme::new(Color::Green, Color::White));
        self.set_color(ComponentType::Completion,
            ColorScheme::new(Color::Black, Color::Gray));
        self.set_color(ComponentType::CompletionSelected,
            ColorScheme::new(Color::White, Color::Blue).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::WindowDivider,
            ColorScheme::new(Color::Gray, Color::Gray));

        // シンタックスハイライト
        self.set_color(ComponentType::SyntaxKeyword,
            ColorScheme::new(Color::Blue, Color::White).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::SyntaxString,
            ColorScheme::new(Color::Green, Color::White));
        self.set_color(ComponentType::SyntaxComment,
            ColorScheme::new(Color::Gray, Color::White).with_modifier(Modifier::ITALIC));
        self.set_color(ComponentType::SyntaxNumber,
            ColorScheme::new(Color::Magenta, Color::White));
        self.set_color(ComponentType::SyntaxOperator,
            ColorScheme::new(Color::Red, Color::White));
    }

    fn set_dark_colors(&mut self) {
        // ダークテーマのカラー設定
        self.set_color(ComponentType::TextArea,
            ColorScheme::new(Color::White, Color::Black));
        self.set_color(ComponentType::LineNumber,
            ColorScheme::new(Color::Gray, Color::Black));
        self.set_color(ComponentType::Cursor,
            ColorScheme::new(Color::Black, Color::White));
        self.set_color(ComponentType::Selection,
            ColorScheme::new(Color::White, Color::Blue));
        self.set_color(ComponentType::Minibuffer,
            ColorScheme::new(Color::White, Color::DarkGray));
        self.set_color(ComponentType::StatusLine,
            ColorScheme::new(Color::Black, Color::Gray));
        self.set_color(ComponentType::Border,
            ColorScheme::new(Color::Gray, Color::Black));
        self.set_color(ComponentType::Error,
            ColorScheme::new(Color::LightRed, Color::Black).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Warning,
            ColorScheme::new(Color::LightYellow, Color::Black).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Info,
            ColorScheme::new(Color::LightGreen, Color::Black));
        self.set_color(ComponentType::Completion,
            ColorScheme::new(Color::White, Color::DarkGray));
        self.set_color(ComponentType::CompletionSelected,
            ColorScheme::new(Color::Black, Color::White).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::WindowDivider,
            ColorScheme::new(Color::Black, Color::DarkGray));

        // シンタックスハイライト
        self.set_color(ComponentType::SyntaxKeyword,
            ColorScheme::new(Color::LightBlue, Color::Black).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::SyntaxString,
            ColorScheme::new(Color::LightGreen, Color::Black));
        self.set_color(ComponentType::SyntaxComment,
            ColorScheme::new(Color::DarkGray, Color::Black).with_modifier(Modifier::ITALIC));
        self.set_color(ComponentType::SyntaxNumber,
            ColorScheme::new(Color::LightMagenta, Color::Black));
        self.set_color(ComponentType::SyntaxOperator,
            ColorScheme::new(Color::LightRed, Color::Black));
    }

    fn set_high_contrast_colors(&mut self) {
        // ハイコントラストテーマのカラー設定
        self.set_color(ComponentType::TextArea,
            ColorScheme::new(Color::White, Color::Black));
        self.set_color(ComponentType::LineNumber,
            ColorScheme::new(Color::White, Color::Black).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Cursor,
            ColorScheme::new(Color::Black, Color::White).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Selection,
            ColorScheme::new(Color::Black, Color::White).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Minibuffer,
            ColorScheme::new(Color::White, Color::Black).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::StatusLine,
            ColorScheme::new(Color::Black, Color::White).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Border,
            ColorScheme::new(Color::White, Color::Black).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Error,
            ColorScheme::new(Color::White, Color::Red).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Warning,
            ColorScheme::new(Color::Black, Color::Yellow).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Info,
            ColorScheme::new(Color::Black, Color::Green).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::Completion,
            ColorScheme::new(Color::White, Color::Black).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::CompletionSelected,
            ColorScheme::new(Color::Black, Color::White).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::WindowDivider,
            ColorScheme::new(Color::White, Color::Black).with_modifier(Modifier::BOLD));

        // シンタックスハイライト - ハイコントラストでは色分けを最小限に
        self.set_color(ComponentType::SyntaxKeyword,
            ColorScheme::new(Color::White, Color::Black).with_modifier(Modifier::BOLD));
        self.set_color(ComponentType::SyntaxString,
            ColorScheme::new(Color::White, Color::Black));
        self.set_color(ComponentType::SyntaxComment,
            ColorScheme::new(Color::White, Color::Black).with_modifier(Modifier::ITALIC));
        self.set_color(ComponentType::SyntaxNumber,
            ColorScheme::new(Color::White, Color::Black).with_modifier(Modifier::UNDERLINED));
        self.set_color(ComponentType::SyntaxOperator,
            ColorScheme::new(Color::White, Color::Black).with_modifier(Modifier::BOLD));
    }
}

/// テーママネージャー
pub struct ThemeManager {
    /// 利用可能なテーマ
    themes: HashMap<String, Theme>,
    /// 現在のテーマ
    current_theme: String,
    /// カスタムテーマの保存場所
    #[allow(dead_code)]
    custom_themes_path: Option<String>,
}

impl ThemeManager {
    /// 新しいテーママネージャーを作成
    pub fn new() -> Self {
        let mut manager = Self {
            themes: HashMap::new(),
            current_theme: "dark".to_string(),
            custom_themes_path: None,
        };

        // デフォルトテーマを登録
        manager.register_default_themes();
        manager
    }

    /// デフォルトテーマを登録
    fn register_default_themes(&mut self) {
        // ライトテーマ
        let light_theme = Theme::new("light".to_string(), ThemeType::Light);
        self.themes.insert("light".to_string(), light_theme);

        // ダークテーマ
        let dark_theme = Theme::new("dark".to_string(), ThemeType::Dark);
        self.themes.insert("dark".to_string(), dark_theme);

        // ハイコントラストテーマ
        let high_contrast_theme = Theme::new("high_contrast".to_string(), ThemeType::HighContrast);
        self.themes.insert("high_contrast".to_string(), high_contrast_theme);
    }

    /// 現在のテーマを取得
    pub fn current_theme(&self) -> &Theme {
        self.themes.get(&self.current_theme)
            .unwrap_or_else(|| self.themes.get("dark").unwrap())
    }

    /// テーマを切り替え
    pub fn set_theme(&mut self, theme_name: &str) -> bool {
        if self.themes.contains_key(theme_name) {
            self.current_theme = theme_name.to_string();
            true
        } else {
            false
        }
    }

    /// 利用可能なテーマ一覧を取得
    pub fn available_themes(&self) -> Vec<&String> {
        self.themes.keys().collect()
    }

    /// カスタムテーマを追加
    pub fn add_custom_theme(&mut self, theme: Theme) {
        let name = theme.name.clone();
        self.themes.insert(name, theme);
    }

    /// テーマを削除（デフォルトテーマは削除不可）
    pub fn remove_theme(&mut self, theme_name: &str) -> bool {
        if ["light", "dark", "high_contrast"].contains(&theme_name) {
            return false; // デフォルトテーマは削除不可
        }

        self.themes.remove(theme_name).is_some()
    }

    /// 16色モードに切り替え
    pub fn enable_16_color_mode(&mut self, enable: bool) {
        for theme in self.themes.values_mut() {
            theme.color_16_mode = enable;
        }
    }

    /// 日本語サポートを切り替え
    pub fn enable_japanese_support(&mut self, enable: bool) {
        for theme in self.themes.values_mut() {
            theme.japanese_support = enable;
        }
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let theme = Theme::new("test".to_string(), ThemeType::Dark);
        assert_eq!(theme.name, "test");
        assert_eq!(theme.theme_type, ThemeType::Dark);
        assert!(theme.japanese_support);
    }

    #[test]
    fn test_color_scheme() {
        let scheme = ColorScheme::new(Color::Red, Color::Blue)
            .with_modifier(Modifier::BOLD);

        assert_eq!(scheme.foreground, Color::Red);
        assert_eq!(scheme.background, Color::Blue);
        assert!(scheme.modifiers.contains(Modifier::BOLD));
    }

    #[test]
    fn test_theme_manager() {
        let mut manager = ThemeManager::new();

        // デフォルトテーマの存在確認
        assert!(manager.available_themes().contains(&&"dark".to_string()));
        assert!(manager.available_themes().contains(&&"light".to_string()));

        // テーマ切り替え
        assert!(manager.set_theme("light"));
        assert_eq!(manager.current_theme().name, "light");

        // 存在しないテーマへの切り替え
        assert!(!manager.set_theme("nonexistent"));
    }

    #[test]
    fn test_16_color_conversion() {
        let mut theme = Theme::new("test".to_string(), ThemeType::Dark);
        theme.color_16_mode = true;

        let white_rgb = Color::Rgb(255, 255, 255);
        assert_eq!(theme.to_16_color(white_rgb), Color::White);

        let black_rgb = Color::Rgb(0, 0, 0);
        assert_eq!(theme.to_16_color(black_rgb), Color::Black);
    }

    #[test]
    fn test_japanese_support() {
        let theme = Theme::new("test".to_string(), ThemeType::Dark);
        let bold_style = Style::default().add_modifier(Modifier::BOLD);

        let adjusted = theme.adjust_for_japanese(bold_style);
        assert!(!adjusted.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_custom_theme() {
        let mut manager = ThemeManager::new();
        let custom_theme = Theme::new("custom".to_string(), ThemeType::Custom("my_theme".to_string()));

        manager.add_custom_theme(custom_theme);
        assert!(manager.available_themes().contains(&&"custom".to_string()));

        // カスタムテーマは削除可能
        assert!(manager.remove_theme("custom"));
        assert!(!manager.available_themes().contains(&&"custom".to_string()));

        // デフォルトテーマは削除不可
        assert!(!manager.remove_theme("dark"));
    }

    #[test]
    fn test_component_styles() {
        let theme = Theme::new("test".to_string(), ThemeType::Dark);

        let text_style = theme.style(&ComponentType::TextArea);
        assert_eq!(text_style.fg, Some(Color::White));
        assert_eq!(text_style.bg, Some(Color::Black));

        let error_style = theme.style(&ComponentType::Error);
        assert_eq!(error_style.fg, Some(Color::LightRed));
        assert!(error_style.add_modifier.contains(Modifier::BOLD));
    }
}
