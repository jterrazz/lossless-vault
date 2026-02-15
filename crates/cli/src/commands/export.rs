use std::path::PathBuf;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use photopack_core::{export::ExportProgress, Vault};

pub fn run(vault: &mut Vault, path: Option<PathBuf>, quality: u8) -> Result<()> {
    if let Some(path) = path {
        vault.set_export_path(&path)?;
        let resolved = vault.get_export_path()?.unwrap();
        println!("Export path set to: {}", resolved.display());
    }

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
