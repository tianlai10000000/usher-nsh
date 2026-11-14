//! Generates or verifies the startup theme artifact using the production Rust registry.
use std::{env, fs, io, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let path = PathBuf::from(
        args.next()
            .ok_or("usage: theme-snapshots <output.json> [--check]")?,
    );
    let check = match args.next().as_deref() {
        None => false,
        Some("--check") => true,
        _ => return Err("invalid option".into()),
    };
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let output = format!(
        "{}\n",
        serde_json::to_string_pretty(&usher_nsh::state::appearance::bundled_snapshots())?
    );
    if check {
        if fs::read_to_string(&path)? != output {
            return Err(io::Error::other("startup theme artifact is stale").into());
        }
    } else {
        fs::write(path, output)?;
    }
    Ok(())
}
