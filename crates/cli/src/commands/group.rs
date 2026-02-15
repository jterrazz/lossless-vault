use anyhow::Result;
use losslessvault_core::Vault;

pub fn run(vault: &Vault, id: i64) -> Result<()> {
    let group = vault.group(id)?;

    println!("Group #{} ({})", group.id, group.confidence);
    println!("{}", "-".repeat(60));

    for member in &group.members {
        let marker = if member.id == group.source_of_truth_id {
            " [SOURCE]"
        } else {
            ""
        };
        println!(
            "  {} ({}, {:.1} KB){}",
            member.path.display(),
            member.format,
            member.size as f64 / 1024.0,
            marker,
        );
    }

    Ok(())
}
