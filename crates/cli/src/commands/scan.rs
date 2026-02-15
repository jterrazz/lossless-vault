use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use losslessvault_core::{ScanProgress, Vault};

pub fn run(vault: &Vault) -> Result<()> {
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    vault.scan(Some(&mut |progress| match progress {
        ScanProgress::SourceStart {
            source,
            file_count,
        } => {
            pb.set_length(file_count as u64);
            pb.set_position(0);
            pb.set_message(format!("Scanning {source}"));
        }
        ScanProgress::FileProcessed { .. } => {
            pb.inc(1);
        }
        ScanProgress::PhaseComplete { phase } => {
            pb.finish_with_message(format!("{phase} complete"));
        }
    }))?;

    println!("Scan complete.");
    Ok(())
}
