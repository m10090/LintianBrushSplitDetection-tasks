use anyhow::{Context, Result};
use debian_control::lossless::Control;

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "debian/control".to_owned());

    let control = Control::from_file(&path).with_context(|| format!("read {path}"))?;

    for binary in control.binaries() {
        if let Some(name) = binary.name() {
            println!("{name}");
        }
    }

    Ok(())
}
