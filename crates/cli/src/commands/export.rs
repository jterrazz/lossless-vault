use std::path::PathBuf;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use losslessvault_core::{export::ExportProgress, Vault};

pub fn set(vault: &Vault, path: PathBuf) -> Result<()> {
    vault.set_export_path(&path)?;
    let resolved = vault.get_export_path()?.unwrap();
    println!("Export path set to: {}", resolved.display());
    Ok(())
}

pub fn show(vault: &Vault) -> Result<()> {
    match vault.get_export_path()? {
        Some(path) => println!("Export path: {}", path.display()),
        None => println!("No export path configured. Use `lsvault export set <path>` to set one."),
    }
    Ok(())
}

pub fn run(vault: &Vault, quality: u8) -> Result<()> {
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    vault.export(
        quality,
        Some(&mut |progress| match progress {
            ExportProgress::Start { total } => {
                pb.set_length(total as u64);
                pb.set_position(0);
                pb.set_message("Converting photos to HEIC...");
            }
            ExportProgress::Converted { target, .. } => {
                pb.inc(1);
                pb.set_message(format!("-> {}", target.display()));
            }
            ExportProgress::Skipped { .. } => {
                pb.inc(1);
            }
            ExportProgress::Complete {
                converted,
                skipped,
            } => {
                pb.finish_with_message(format!("{converted} converted, {skipped} skipped"));
            }
        }),
    )?;

    println!("Export complete.");
    Ok(())
}
