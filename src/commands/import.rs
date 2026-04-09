use crate::utils::config::Config;
use anyhow::{Context as _, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use colored::Colorize as _;
use std::path::Path;

pub async fn execute(config: Config, name: String, data: String, quiet: bool) -> Result<()> {
    let profile_dir = config.profile_path(&name);

    if profile_dir.exists() {
        let confirm = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Profile '{}' already exists. Overwrite?",
                name.yellow()
            ))
            .default(false)
            .interact()?;

        if !confirm {
            if !quiet {
                println!("Cancelled");
            }
            return Ok(());
        }

        tokio::fs::remove_dir_all(&profile_dir).await?;
    }

    // Decode base64
    let decoded = STANDARD
        .decode(&data)
        .with_context(|| "Failed to decode base64 data")?;

    // Decompress (gzip)
    let decompressed = decompress(&decoded)?;

    // Parse as tarball and extract
    extract_tarball(&decompressed, &profile_dir).await?;

    if !quiet {
        println!(
            "{} Profile {} imported successfully",
            "✓".green().bold(),
            name.cyan()
        );
        println!("  {}: {}", "Location".dimmed(), profile_dir.display());
    }

    Ok(())
}

fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut decoder = flate2::read::GzDecoder::new(data);
    let mut result = Vec::new();
    decoder.read_to_end(&mut result)?;
    Ok(result)
}

async fn extract_tarball(data: &[u8], dest: &Path) -> Result<()> {
    use std::io::Cursor;
    use tar::Archive;

    let cursor = Cursor::new(data);
    let mut archive = Archive::new(cursor);

    tokio::fs::create_dir_all(dest).await?;
    archive.unpack(dest)?;

    Ok(())
}
