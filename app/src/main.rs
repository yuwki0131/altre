use altre::{App, Result};

fn main() -> Result<()> {
    println!("altre - Modern Emacs-inspired text editor");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    let mut app = App::new()?;
    app.run()?;

    Ok(())
}
