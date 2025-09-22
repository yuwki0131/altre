use altre::{error, App, Result};

fn main() -> Result<()> {
    error::setup_panic_handler();

    println!("altre - Modern Emacs-inspired text editor");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    let mut app = App::new()?;
    app.run()?;

    Ok(())
}
