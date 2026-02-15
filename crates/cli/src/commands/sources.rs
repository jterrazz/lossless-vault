use anyhow::Result;
use losslessvault_core::Vault;

pub fn run(vault: &Vault) -> Result<()> {
    let sources = vault.sources()?;

    if sources.is_empty() {
        println!("No sources registered. Use `lsvault add <path>` to add one.");
        return Ok(());
    }

    println!("{:<4} {:<60} {}", "ID", "Path", "Last Scanned");
    println!("{}", "-".repeat(80));

    for source in &sources {
        let scanned = match source.last_scanned {
            Some(ts) => chrono::DateTime::from_timestamp(ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            None => "never".to_string(),
        };
        println!("{:<4} {:<60} {}", source.id, source.path.display(), scanned);
    }

    Ok(())
}
