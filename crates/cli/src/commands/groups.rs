use anyhow::Result;
use losslessvault_core::Vault;

pub fn run(vault: &Vault) -> Result<()> {
    let groups = vault.groups()?;

    if groups.is_empty() {
        println!("No duplicate groups found. Run `lsvault scan` first.");
        return Ok(());
    }

    println!(
        "{:<6} {:<12} {:<8} {}",
        "ID", "Confidence", "Members", "Source of Truth"
    );
    println!("{}", "-".repeat(80));

    for group in &groups {
        let sot = group
            .members
            .iter()
            .find(|m| m.id == group.source_of_truth_id)
            .map(|m| m.path.display().to_string())
            .unwrap_or_else(|| "?".to_string());

        println!(
            "{:<6} {:<12} {:<8} {}",
            group.id,
            group.confidence,
            group.members.len(),
            sot,
        );
    }

    Ok(())
}
