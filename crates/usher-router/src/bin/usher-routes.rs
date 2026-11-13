//! Validates a catalog and writes or checks generated TypeScript without altering check inputs.
use std::{env, fs, path::Path};
use usher_router::{Catalog, Router, matcher::Matchit};
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 3 || !matches!(args[0].as_str(), "generate" | "check") {
        return Err("usage: usher-routes <generate|check> <catalog.json> <generated.ts>".into());
    }
    let catalog: Catalog = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    let generated = Router::new(catalog, &Matchit)?.typescript();
    let output = Path::new(&args[2]);
    if args[0] == "check" {
        if fs::read_to_string(output)? != generated {
            return Err("generated route contracts are stale".into());
        }
        println!("checked route contracts: {}", args[2]);
    } else {
        fs::write(output, generated)?;
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("route catalog: {error}");
        std::process::exit(1);
    }
}
