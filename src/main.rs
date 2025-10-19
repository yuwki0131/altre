use altre::TuiApplication;
use altre::{error, Result};
#[cfg(feature = "gui")]
use altre::{GuiApplication, GuiRunOptions};
#[cfg(feature = "gui")]
use std::path::PathBuf;

fn main() -> Result<()> {
    error::setup_panic_handler();
    #[cfg(feature = "gui")]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let (run_mode, debug_log) = parse_run_mode(&args);
        match run_mode {
            RunMode::Tui => run_tui()?,
            RunMode::Gui => run_gui(debug_log)?,
        }
    }

    #[cfg(not(feature = "gui"))]
    {
        run_tui()?;
    }

    Ok(())
}

fn run_tui() -> Result<()> {
    println!("altre - Modern Emacs-inspired text editor");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    let mut app = TuiApplication::new()?;
    app.run()
}

#[cfg(feature = "gui")]
fn run_gui(debug_log: Option<PathBuf>) -> Result<()> {
    let mut app = GuiApplication::with_options(GuiRunOptions { debug_log })?;
    app.run()
}

#[cfg(feature = "gui")]
enum RunMode {
    Gui,
    Tui,
}

#[cfg(feature = "gui")]
fn parse_run_mode(args: &[String]) -> (RunMode, Option<PathBuf>) {
    let mut run_mode = RunMode::Gui;
    let mut debug_log: Option<PathBuf> = None;

    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--tui" => run_mode = RunMode::Tui,
            "--gui" => run_mode = RunMode::Gui,
            "--gui-debug" => {
                let path = iter.peek().and_then(|next| {
                    if !next.starts_with('-') {
                        Some(PathBuf::from((*next).clone()))
                    } else {
                        None
                    }
                });
                if path.is_some() {
                    debug_log = path;
                    iter.next();
                } else {
                    debug_log = Some(PathBuf::from("debug.log"));
                }
                run_mode = RunMode::Gui;
            }
            "--debug-log" => {
                if let Some(next) = iter.peek() {
                    if !next.starts_with('-') {
                        debug_log = Some(PathBuf::from((*next).clone()));
                        iter.next();
                    }
                }
            }
            _ => {}
        }
    }

    (run_mode, debug_log)
}
