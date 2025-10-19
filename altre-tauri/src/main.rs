use altre_tauri::{BackendController, BackendOptions};

fn main() {
    match BackendController::new(BackendOptions::default()) {
        Ok(mut controller) => {
            if let Ok(snapshot) = controller.snapshot() {
                println!("Tauri GUI プレースホルダ起動");
                println!("現在のバッファ行数: {}", snapshot.buffer.lines.len());
            } else {
                eprintln!("スナップショット取得に失敗しました（プレースホルダ）");
            }
        }
        Err(err) => {
            eprintln!("Tauri GUI は未実装です: {err}");
        }
    }
}
