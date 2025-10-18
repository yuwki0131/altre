fn main() {
    if std::env::var_os("CARGO_FEATURE_GUI").is_some() {
        slint_build::compile("src/frontend/gui/components/main.slint")
            .expect("Slint GUI コンポーネントの生成に失敗しました");
    }
}
