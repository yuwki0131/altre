use altre::TuiApplication;
use altre::{error, Result};
use std::env;
use std::path::Path;
use std::process::{self, Command, Stdio};

fn main() -> Result<()> {
    error::setup_panic_handler();

    let args: Vec<String> = env::args().collect();
    let mut args_iter = args.iter().skip(1);
    let mut force_tui = false;

    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--tui" => force_tui = true,
            "--gui" => force_tui = false,
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "-V" | "--version" => {
                print_version();
                return Ok(());
            }
            _ => {
                eprintln!("不明なオプションです: {arg}");
                print_help();
                process::exit(2);
            }
        }
    }

    if force_tui {
        run_tui()?;
        return Ok(());
    }

    if launch_gui()? {
        return Ok(());
    }

    eprintln!("GUI の起動に失敗したため TUI モードへフォールバックします");
    run_tui()?;
    Ok(())
}

fn run_tui() -> Result<()> {
    println!("altre - Modern Emacs-inspired text editor");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    let mut app = TuiApplication::new()?;
    app.run()
}

fn launch_gui() -> Result<bool> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let manifest_path = workspace_root.join("src-tauri/Cargo.toml");
    let release_binary = workspace_root.join("target/release/altre-tauri-app");

    // 環境変数 `ALTRE_GUI_BINARY` が指定されていればそれを優先
    if let Ok(binary) = env::var("ALTRE_GUI_BINARY") {
        let status = Command::new(binary)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        match status {
            Ok(exit) if exit.success() => return Ok(true),
            Ok(_) => return Ok(false),
            Err(err) => {
                eprintln!("ALTRE_GUI_BINARY の実行に失敗しました: {err}");
                return Ok(false);
            }
        }
    }

    if ensure_gui_binary(workspace_root, &release_binary)? {
        return run_gui_binary(&release_binary);
    }

    // 最後の手段として `cargo tauri dev`
    let status = Command::new("cargo")
        .args([
            "tauri",
            "dev",
            "--manifest-path",
            manifest_path.to_str().unwrap(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(workspace_root)
        .status();

    match status {
        Ok(exit) if exit.success() => Ok(true),
        Ok(_) => Ok(false),
        Err(err) => {
            eprintln!("cargo tauri dev の起動に失敗しました: {err}");
            Ok(false)
        }
    }
}

fn print_help() {
    println!("altre {version}", version = env!("CARGO_PKG_VERSION"));
    println!("使用方法: altre [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --gui        GUI モードで起動 (デフォルト)");
    println!("    --tui        TUI モードを強制");
    println!("    -h, --help   このメッセージを表示");
    println!("    -V, --version バージョン情報を表示");
}

fn print_version() {
    println!("altre {version}", version = env!("CARGO_PKG_VERSION"));
}

fn ensure_gui_binary(workspace_root: &Path, binary_path: &Path) -> Result<bool> {
    if binary_path.exists() {
        return Ok(true);
    }

    let status = Command::new("cargo")
        .args(["build", "-p", "altre-tauri-app", "--release"])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(workspace_root)
        .status();

    match status {
        Ok(exit) if exit.success() => Ok(binary_path.exists()),
        Ok(_) => Ok(false),
        Err(err) => {
            eprintln!("GUI バイナリのビルドに失敗しました: {err}");
            Ok(false)
        }
    }
}

fn run_gui_binary(path: &Path) -> Result<bool> {
    let status = Command::new(path)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    match status {
        Ok(exit) if exit.success() => Ok(true),
        Ok(_) => Ok(false),
        Err(err) => {
            eprintln!("GUI バイナリの実行に失敗しました: {err}");
            Ok(false)
        }
    }
}
