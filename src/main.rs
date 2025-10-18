use altre::{error, Result};
#[cfg(feature = "gui")]
use altre::GuiApplication;
#[cfg(not(feature = "gui"))]
use altre::TuiApplication;

fn main() -> Result<()> {
    error::setup_panic_handler();

    #[cfg(not(feature = "gui"))]
    {
        println!("altre - Modern Emacs-inspired text editor");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!();

        let mut app = TuiApplication::new()?;
        app.run()?;
    }

    #[cfg(feature = "gui")]
    {
        let mut app = GuiApplication::new()?;
        app.run()?;
    }

    Ok(())
}
