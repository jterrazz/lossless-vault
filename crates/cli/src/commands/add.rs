use std::path::PathBuf;

use anyhow::Result;
use losslessvault_core::Vault;

pub fn run(vault: &Vault, path: PathBuf) -> Result<()> {
    let source = vault.add_source(&path)?;
    println!("Added source: {}", source.path.display());
    Ok(())
}
