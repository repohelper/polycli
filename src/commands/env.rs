#![allow(deprecated)]

use crate::utils::config::Config;
use anyhow::Result;
use colored::Colorize as _;

/// Escape a value for safe inclusion in a bash/zsh/fish single-quoted string.
///
/// The value is wrapped in single quotes. Embedded single quotes are escaped
/// using the `'\''` idiom (close quote, escaped quote, reopen quote).
///
/// This neutralises all bash special characters (`$`, `` ` ``, `!`, `(`, `)`,
/// spaces, etc.) because none of them are interpreted inside single quotes.
fn shell_escape_bash(value: &str) -> String {
    let escaped = value.replace('\'', r"'\''");
    format!("'{}'", escaped)
}

/// Escape a value for safe inclusion in a PowerShell single-quoted string.
///
/// PowerShell single-quoted strings interpret nothing except `'`, which is
/// escaped by doubling it (`''`).  This neutralises `$`, backticks, `()`,
/// spaces, and everything else that PowerShell would otherwise expand.
fn shell_escape_powershell(value: &str) -> String {
    let escaped = value.replace('\'', "''");
    format!("'{}'", escaped)
}

/// Escape a value for safe use as the RHS of a CMD `set "VAR=value"` command.
///
/// The `set "VAR=…"` form tells cmd.exe to treat everything between the outer
/// double-quotes as a literal string (no caret-escaping needed for `&`, `|`,
/// `<`, `>`).  Two characters still require special handling:
///
/// * `%` – would trigger variable expansion even inside the quoted form;
///   doubled to `%%` to produce a literal `%`.
/// * `"` – would prematurely close the surrounding quotes; replaced with `""`
///   (an adjacent empty pair), which is the closest cmd.exe approximation.
///
/// The caller is responsible for wrapping the result in the `set "VAR=…"`
/// form, e.g. `set "CODEXCTL=<escaped>"`.
fn shell_escape_cmd(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 4);
    for ch in value.chars() {
        match ch {
            '%' => out.push_str("%%"),
            '"' => out.push_str("\"\""),
            _ => out.push(ch),
        }
    }
    out
}

/// Generate shell commands to set up environment for using a specific profile
/// This allows using different profiles in different terminals concurrently
pub async fn execute(
    config: Config,
    profile: String,
    shell: String,
    unset: bool,
    quiet: bool,
) -> Result<()> {
    let profile_dir = config.profile_path(&profile);

    if !profile_dir.exists() {
        anyhow::bail!(
            "Profile '{}' not found. Use 'poly list' to see available profiles.",
            profile
        );
    }

    if unset {
        match shell.as_str() {
            "fish" => {
                println!("set -e CODEXCTL;");
                println!("set -e CODEXCTL_DIR;");
            }
            "powershell" | "pwsh" => {
                println!("Remove-Item Env:CODEXCTL -ErrorAction SilentlyContinue;");
                println!("Remove-Item Env:CODEXCTL_DIR -ErrorAction SilentlyContinue;");
            }
            "cmd" | "batch" => {
                println!("set CODEXCTL=");
                println!("set CODEXCTL_DIR=");
            }
            _ => {
                // bash, zsh, etc.
                println!("unset CODEXCTL;");
                println!("unset CODEXCTL_DIR;");
            }
        }

        if !quiet {
            eprintln!(
                "{} Environment cleared. Using default Codex auth.",
                "✓".green()
            );
        }
        return Ok(());
    }

    let profile_dir_str = profile_dir.to_string_lossy();

    match shell.as_str() {
        "fish" => {
            println!("set -x CODEXCTL {};", shell_escape_bash(&profile));
            println!(
                "set -x CODEXCTL_DIR {};",
                shell_escape_bash(&profile_dir_str)
            );
            println!("# Use with: codex");
            println!(
                "# Or run: eval (poly env {} --unset) to clear",
                shell_escape_bash(&profile)
            );
        }
        "powershell" | "pwsh" => {
            println!("$env:CODEXCTL = {};", shell_escape_powershell(&profile));
            println!(
                "$env:CODEXCTL_DIR = {};",
                shell_escape_powershell(&profile_dir_str)
            );
            println!("# Use with: codex");
            println!(
                "# Or run: poly env {} --unset | Invoke-Expression to clear",
                shell_escape_powershell(&profile)
            );
        }
        "cmd" | "batch" => {
            println!("set \"CODEXCTL={}\"", shell_escape_cmd(&profile));
            println!(
                "set \"CODEXCTL_DIR={}\"",
                shell_escape_cmd(&profile_dir_str)
            );
            println!("REM Use with: codex");
        }
        _ => {
            // bash, zsh, etc.
            println!("export CODEXCTL={};", shell_escape_bash(&profile));
            println!(
                "export CODEXCTL_DIR={};",
                shell_escape_bash(&profile_dir_str)
            );
            println!("# Use with: codex");
            println!(
                "# Or run: eval $(poly env {} --unset) to clear",
                shell_escape_bash(&profile)
            );
        }
    }

    if !quiet {
        eprintln!();
        eprintln!(
            "{} Profile '{}' environment configured.",
            "✓".green(),
            profile.cyan()
        );
        eprintln!("{} Run the commands above to use this profile.", "ℹ".blue());
        eprintln!("{} This won't affect other terminals!", "ℹ".blue());
        eprintln!();
        eprintln!("Example usage:");
        eprintln!("  {}", format!("eval $(poly env {})", profile).yellow());
        eprintln!("  {}  # Uses '{}' profile", "codex".yellow(), profile);
        eprintln!();
        eprintln!("To switch back to default:");
        eprintln!(
            "  {}",
            format!("eval $(poly env {} --unset)", profile).yellow()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── bash / zsh / fish ──────────────────────────────────────────────────

    #[test]
    fn bash_plain_value() {
        assert_eq!(shell_escape_bash("myprofile"), "'myprofile'");
    }

    #[test]
    fn bash_spaces() {
        assert_eq!(shell_escape_bash("my profile"), "'my profile'");
    }

    #[test]
    fn bash_dollar_sign() {
        assert_eq!(shell_escape_bash("$HOME"), "'$HOME'");
    }

    #[test]
    fn bash_backtick() {
        assert_eq!(shell_escape_bash("`id`"), "'`id`'");
    }

    #[test]
    fn bash_subshell_parens() {
        assert_eq!(shell_escape_bash("$(id)"), "'$(id)'");
    }

    #[test]
    fn bash_embedded_single_quote() {
        // O'Brien → 'O'\''Brien'
        assert_eq!(shell_escape_bash("O'Brien"), r"'O'\''Brien'");
    }

    #[test]
    fn bash_double_quote() {
        assert_eq!(shell_escape_bash(r#"say "hi""#), r#"'say "hi"'"#);
    }

    #[test]
    fn bash_path_with_spaces() {
        assert_eq!(
            shell_escape_bash("/home/user/my docs"),
            "'/home/user/my docs'"
        );
    }

    #[test]
    fn bash_hostile_injection() {
        // Attempt to break out and run a command
        let hostile = "'; rm -rf /; echo '";
        let escaped = shell_escape_bash(hostile);
        // The result must keep the injected content inert
        assert_eq!(escaped, r"''\''; rm -rf /; echo '\'''");
    }

    #[test]
    fn bash_newline() {
        assert_eq!(shell_escape_bash("line1\nline2"), "'line1\nline2'");
    }

    #[test]
    fn bash_exclamation() {
        // ! triggers history expansion in interactive bash, but not inside single quotes
        assert_eq!(shell_escape_bash("hello!world"), "'hello!world'");
    }

    // ── PowerShell ─────────────────────────────────────────────────────────

    #[test]
    fn powershell_plain_value() {
        assert_eq!(shell_escape_powershell("myprofile"), "'myprofile'");
    }

    #[test]
    fn powershell_spaces() {
        assert_eq!(shell_escape_powershell("my profile"), "'my profile'");
    }

    #[test]
    fn powershell_dollar_sign() {
        // $ starts variable expansion in double-quoted PS strings; single quotes are safe
        assert_eq!(shell_escape_powershell("$env:PATH"), "'$env:PATH'");
    }

    #[test]
    fn powershell_backtick() {
        // backtick is the PS escape char; must be inert inside single quotes
        assert_eq!(shell_escape_powershell("`whoami`"), "'`whoami`'");
    }

    #[test]
    fn powershell_embedded_single_quote() {
        assert_eq!(shell_escape_powershell("it's"), "'it''s'");
    }

    #[test]
    fn powershell_windows_path() {
        assert_eq!(
            shell_escape_powershell(r"C:\Users\Alice\My Documents"),
            r"'C:\Users\Alice\My Documents'"
        );
    }

    #[test]
    fn powershell_hostile_injection() {
        let hostile = "'; Remove-Item -Recurse C:\\; $x='";
        let escaped = shell_escape_powershell(hostile);
        assert_eq!(escaped, "'''; Remove-Item -Recurse C:\\; $x='''");
    }

    #[test]
    fn powershell_parens() {
        assert_eq!(shell_escape_powershell("a(b)c"), "'a(b)c'");
    }

    // ── CMD ────────────────────────────────────────────────────────────────

    #[test]
    fn cmd_plain_value() {
        assert_eq!(shell_escape_cmd("myprofile"), "myprofile");
    }

    #[test]
    fn cmd_spaces() {
        // Spaces are safe inside the "set VAR=…" quoted form
        assert_eq!(shell_escape_cmd("my profile"), "my profile");
    }

    #[test]
    fn cmd_percent_sign() {
        assert_eq!(shell_escape_cmd("%PATH%"), "%%PATH%%");
    }

    #[test]
    fn cmd_double_quote() {
        // Each " is doubled: say "hi" → say ""hi""
        assert_eq!(shell_escape_cmd(r#"say "hi""#), r#"say ""hi"""#);
    }

    #[test]
    fn cmd_windows_path_with_spaces() {
        assert_eq!(
            shell_escape_cmd(r"C:\Program Files\codex"),
            r"C:\Program Files\codex"
        );
    }

    #[test]
    fn cmd_hostile_percent_injection() {
        // Prevent %COMSPEC% from expanding
        let hostile = "%COMSPEC% /c calc.exe";
        assert_eq!(shell_escape_cmd(hostile), "%%COMSPEC%% /c calc.exe");
    }

    #[test]
    fn cmd_hostile_quote_injection() {
        // Attempt to break out of the "set VAR=…" quoting
        let hostile = r#"foo" & calc.exe & set "X=bar"#;
        let escaped = shell_escape_cmd(hostile);
        // The " chars must be doubled, not left bare
        assert!(escaped.contains("\"\""));
        assert!(!escaped.starts_with('\"'));
    }

    #[test]
    fn cmd_empty_value() {
        assert_eq!(shell_escape_cmd(""), "");
    }
}
