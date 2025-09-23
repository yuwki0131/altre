use altre::ui::minibuffer::MinibufferSession;
use std::io::{self, Write};

fn main() {
    println!("altre - Modern Emacs-inspired text editor");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    println!("Type alisp expressions. Use :quit to exit.");
    let mut session = MinibufferSession::new();
    let stdin = io::stdin();
    loop {
        print!("alisp> ");
        if io::stdout().flush().is_err() {
            break;
        }
        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if matches!(trimmed, ":quit" | ":exit") {
                    break;
                }
                let outcome = session.evaluate(trimmed);
                println!("{}", outcome.output);
                for msg in outcome.messages {
                    println!("message: {}", msg);
                }
            }
            Err(_) => break,
        }
    }
}
