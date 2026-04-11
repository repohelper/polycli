use crate::utils::config::Config;
use crate::utils::validation::ProfileName;
use anyhow::{Context as _, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use colored::Colorize as _;

pub async fn execute(config: Config, name: String, quiet: bool) -> Result<()> {
    let profile_name = ProfileName::try_from(name.as_str())
        .with_context(|| format!("Invalid profile name '{name}'"))?;
    let profile_dir = config.profile_path_validated(&profile_name)?;

    if !profile_dir.exists() {
        anyhow::bail!("Profile '{name}' not found");
    }

    // Create tarball
    let tarball = create_tarball(&profile_dir)?;

    // Compress (gzip)
    let compressed = compress(&tarball)?;

    // Encode base64
    let encoded = STANDARD.encode(&compressed);

    if !quiet {
        println!("{} Profile {} exported\n", "✓".green().bold(), name.cyan());
    }

    println!("{encoded}");

    // Also save to file
    let export_path = profile_dir.join(format!("{name}.export.txt"));
    tokio::fs::write(&export_path, &encoded).await?;

    if !quiet {
        println!("\n  {}: {}", "Saved to".dimmed(), export_path.display());
        println!(
            "  {}: Copy the base64 string above to import on another machine",
            "Tip".yellow()
        );
    }

    Ok(())
}

fn compress(data: &[u8]) -> Result<Vec<u8>> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

fn create_tarball(dir: &std::path::Path) -> Result<Vec<u8>> {
    use std::io::Cursor;
    use tar::Builder;

    let mut buf = Vec::new();
    {
        let cursor = Cursor::new(&mut buf);
        let mut builder = Builder::new(cursor);
        builder.append_dir_all(".", dir)?;
        builder.finish()?;
    }

    Ok(buf)
}
