use altre::TuiApplication;
use altre::{error, Result};

fn main() -> Result<()> {
    error::setup_panic_handler();
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
