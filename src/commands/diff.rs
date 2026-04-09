#![allow(deprecated)]

use crate::utils::config::Config;
use anyhow::Result;
use colored::Colorize as _;
use prettytable::{Cell, Row, Table, format};
use std::collections::HashMap;

pub async fn execute(
    config: Config,
    profile1: String,
    profile2: String,
    changes_only: bool,
    quiet: bool,
) -> Result<()> {
    let dir1 = config.profile_path(&profile1);
    let dir2 = config.profile_path(&profile2);

    if !dir1.exists() {
        anyhow::bail!("Profile '{}' not found", profile1);
    }
    if !dir2.exists() {
        anyhow::bail!("Profile '{}' not found", profile2);
    }

    if !quiet {
        println!(
            "\n{} Comparing profiles: {} vs {}",
            "🔍".cyan(),
            profile1.green(),
            profile2.yellow()
        );
    }

    // Get file listings for both profiles
    let files1 = get_profile_files(&dir1).await?;
    let files2 = get_profile_files(&dir2).await?;

    // Compare files
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    // Header
    table.add_row(Row::new(vec![
        Cell::new("File").style_spec("Fb"),
        Cell::new(&profile1.to_string()).style_spec("Fg"),
        Cell::new(&profile2.to_string()).style_spec("Fy"),
        Cell::new("Status").style_spec("Fb"),
    ]));

    // Get all unique files
    let mut all_files: Vec<String> = files1.keys().cloned().collect();
    for file in files2.keys() {
        if !all_files.contains(file) {
            all_files.push(file.clone());
        }
    }
    all_files.sort();

    let mut differences = 0;

    for file in &all_files {
        let (content1, content2) = (
            files1.get(file).map(|v| v.as_slice()),
            files2.get(file).map(|v| v.as_slice()),
        );

        let status = match (content1, content2) {
            (Some(c1), Some(c2)) => {
                if c1 == c2 {
                    if changes_only {
                        continue;
                    }
                    "✓ Same".to_string()
                } else {
                    differences += 1;
                    format!("{} Different", "≠".yellow())
                }
            }
            (Some(_), None) => {
                differences += 1;
                format!("{} Only in {}", "→".green(), profile1)
            }
            (None, Some(_)) => {
                differences += 1;
                format!("{} Only in {}", "→".yellow(), profile2)
            }
            (None, None) => unreachable!(),
        };

        let size1 = content1
            .map(|c| format!("{} bytes", c.len()))
            .unwrap_or_else(|| "-".to_string());
        let size2 = content2
            .map(|c| format!("{} bytes", c.len()))
            .unwrap_or_else(|| "-".to_string());

        table.add_row(Row::new(vec![
            Cell::new(file),
            Cell::new(&size1),
            Cell::new(&size2),
            Cell::new(&status),
        ]));
    }

    if !all_files.is_empty() {
        table.printstd();
    }

    if !quiet {
        println!();
        if differences == 0 {
            println!("{} Profiles are identical!", "✓".green());
        } else {
            println!("{} {} difference(s) found", "!".yellow(), differences);
        }
    }

    Ok(())
}

async fn get_profile_files(dir: &std::path::Path) -> Result<HashMap<String, Vec<u8>>> {
    let mut files = HashMap::new();

    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let content = tokio::fs::read(&path).await?;
            files.insert(name, content);
        }
    }

    Ok(files)
}
