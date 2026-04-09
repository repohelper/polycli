use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context as _, Result};
use clap::CommandFactory;
use clap_complete::{generate, shells};

use crate::Cli;

pub fn generate_completions(shell: &str) -> Result<String> {
    let mut cmd = Cli::command();

    let mut buf = Vec::new();

    match shell {
        "bash" => {
            generate(shells::Bash, &mut cmd, "codexctl", &mut buf);
        }
        "zsh" => {
            generate(shells::Zsh, &mut cmd, "codexctl", &mut buf);
        }
        "fish" => {
            generate(shells::Fish, &mut cmd, "codexctl", &mut buf);
        }
        "powershell" => {
            generate(shells::PowerShell, &mut cmd, "codexctl", &mut buf);
        }
        "elvish" => {
            generate(shells::Elvish, &mut cmd, "codexctl", &mut buf);
        }
        _ => {
            anyhow::bail!(
                "Unsupported shell: {}. Supported: bash, zsh, fish, powershell, elvish",
                shell
            );
        }
    }

    let output = String::from_utf8(buf)?;
    Ok(output)
}

/// Returns `(install_dir, filename)` for the given shell, using platform-appropriate paths.
fn completion_install_path(shell: &str) -> Result<(PathBuf, &'static str)> {
    match shell {
        "bash" => {
            // Linux/macOS: XDG data dir or ~/.local/share
            let dir = dirs::data_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
                .join("bash-completion")
                .join("completions");
            Ok((dir, "codexctl"))
        }
        "zsh" => {
            let dir = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
                .join(".zsh")
                .join("completions");
            Ok((dir, "_codexctl"))
        }
        "fish" => {
            // Linux/macOS: ~/.config/fish/completions
            // Windows:     %APPDATA%\fish\completions (rare but follow same pattern)
            let dir = dirs::config_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
                .join("fish")
                .join("completions");
            Ok((dir, "codexctl.fish"))
        }
        "powershell" => {
            // PowerShell profile directory varies by OS:
            //   Windows:      ~/Documents/PowerShell/Completions
            //   Linux/macOS:  ~/.config/powershell/Completions
            let dir = powershell_completion_dir()?;
            Ok((dir, "codexctl.ps1"))
        }
        _ => {
            anyhow::bail!(
                "Auto-install not supported for {}. Use --print to output and install manually.",
                shell
            );
        }
    }
}

/// Returns the PowerShell completion directory for the current platform.
fn powershell_completion_dir() -> Result<PathBuf> {
    if cfg!(target_os = "windows") {
        // %USERPROFILE%\Documents\PowerShell\Completions
        dirs::document_dir()
            .map(|d| d.join("PowerShell").join("Completions"))
            .ok_or_else(|| anyhow::anyhow!("Could not determine Documents directory"))
    } else {
        // ~/.config/powershell/Completions  (PowerShell 7 on Linux/macOS)
        dirs::config_dir()
            .map(|d| d.join("powershell").join("Completions"))
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))
    }
}

pub fn install_completions(shell: &str) -> Result<()> {
    let completions = generate_completions(shell)?;

    let (install_dir, filename) = completion_install_path(shell)?;

    // Create directory if needed
    std::fs::create_dir_all(&install_dir).with_context(|| {
        format!(
            "Failed to create completion directory: {}",
            install_dir.display()
        )
    })?;

    // Write completion file
    let file_path = install_dir.join(filename);
    let mut file = std::fs::File::create(&file_path)
        .with_context(|| format!("Failed to create completion file: {}", file_path.display()))?;
    file.write_all(completions.as_bytes())?;

    println!("Completions installed to: {}", file_path.display());

    // Print post-install instructions
    match shell {
        "bash" => {
            println!("\nAdd this to your ~/.bashrc:");
            println!("  source {}", file_path.display());
        }
        "zsh" => {
            let zsh_dir = install_dir.display().to_string();
            println!("\nAdd this to your ~/.zshrc:");
            println!("  fpath+=({zsh_dir})");
            println!("  autoload -U compinit && compinit");
        }
        "fish" => {
            println!("\nFish will auto-load completions. Restart your shell or run:");
            println!("  source {}", file_path.display());
        }
        "powershell" => {
            println!("\nAdd this to your PowerShell profile ($PROFILE):");
            println!("  . \"{}\"", file_path.display());
            println!("\nTo open your profile for editing run:");
            println!("  notepad $PROFILE  # Windows");
            println!("  code $PROFILE     # VS Code");
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_completions_bash() {
        let result = generate_completions("bash");
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_generate_completions_zsh() {
        let result = generate_completions("zsh");
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_completions_fish() {
        let result = generate_completions("fish");
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_completions_powershell() {
        let result = generate_completions("powershell");
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_completions_invalid_shell() {
        let result = generate_completions("cmd");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported shell")
        );
    }

    #[test]
    fn test_completion_install_path_bash() {
        let result = completion_install_path("bash");
        assert!(result.is_ok());
        let (dir, filename) = result.unwrap();
        assert_eq!(filename, "codexctl");
        // Path should contain bash-completion
        assert!(dir.to_string_lossy().contains("bash-completion"));
    }

    #[test]
    fn test_completion_install_path_zsh() {
        let result = completion_install_path("zsh");
        assert!(result.is_ok());
        let (dir, filename) = result.unwrap();
        assert_eq!(filename, "_codexctl");
        assert!(dir.to_string_lossy().contains("zsh"));
    }

    #[test]
    fn test_completion_install_path_fish() {
        let result = completion_install_path("fish");
        assert!(result.is_ok());
        let (_, filename) = result.unwrap();
        assert_eq!(filename, "codexctl.fish");
    }

    #[test]
    fn test_completion_install_path_powershell() {
        let result = completion_install_path("powershell");
        assert!(result.is_ok());
        let (dir, filename) = result.unwrap();
        assert_eq!(filename, "codexctl.ps1");
        let dir_str = dir.to_string_lossy().to_lowercase();
        // On any platform the path should contain "powershell"
        assert!(
            dir_str.contains("powershell"),
            "expected 'powershell' in path, got: {dir_str}"
        );
    }

    #[test]
    fn test_completion_install_path_unsupported() {
        let result = completion_install_path("elvish");
        assert!(result.is_err());
    }

    #[test]
    fn test_powershell_completion_dir_is_absolute() {
        let dir = powershell_completion_dir();
        assert!(dir.is_ok());
        assert!(dir.unwrap().is_absolute());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_powershell_path_uses_documents() {
        let dir = powershell_completion_dir().unwrap();
        let s = dir.to_string_lossy();
        assert!(s.contains("PowerShell") && s.contains("Completions"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_unix_powershell_path_uses_config_dir() {
        let dir = powershell_completion_dir().unwrap();
        let s = dir.to_string_lossy().to_lowercase();
        assert!(s.contains("powershell") && s.contains("completions"));
    }
}
