use std::env;
use std::fs;
use std::process::{Command, exit};

fn run_command(cmd: &str, args: &[&str]) {
    println!("Running: {} {}", cmd, args.join(" "));
    let status = Command::new(cmd)
        .args(args)
        .status()
        .unwrap_or_else(|err| {
            eprintln!("Failed to execute command '{}': {}", cmd, err);
            exit(1);
        });
    if !status.success() {
        eprintln!("Error: Command '{}' failed with exit code {:?}", cmd, status.code());
        exit(status.code().unwrap_or(1));
    }
}

fn main() {
    // 1. Run checks first before doing anything
    println!("=== Running pre-bump checks ===");
    run_command("cargo", &["fmt", "--all", "--", "--check"]);
    run_command("cargo", &["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"]);
    run_command("cargo", &["test", "--all-targets", "--all-features"]);
    println!("=== Pre-bump checks passed successfully! ===\n");

    // 2. Read Cargo.toml
    let cargo_toml_path = "Cargo.toml";
    let content = fs::read_to_string(cargo_toml_path).unwrap_or_else(|err| {
        eprintln!("Error reading {}: {}", cargo_toml_path, err);
        exit(1);
    });

    // Parse the current version. We find the version line within [package].
    let mut in_package = false;
    let mut current_version = None;
    let mut version_line_index = None;
    let lines: Vec<&str> = content.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if in_package && trimmed.starts_with('[') {
            // Entered another section without finding version
            in_package = false;
        }
        if in_package && trimmed.starts_with("version") {
            // Parse version = "X.Y.Z"
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    let version = &line[start + 1..start + 1 + end];
                    current_version = Some(version.to_string());
                    version_line_index = Some(i);
                    break;
                }
            }
        }
    }

    let current_version = current_version.unwrap_or_else(|| {
        eprintln!("Error: Could not find version in [package] section of Cargo.toml");
        exit(1);
    });
    let version_line_idx = version_line_index.unwrap();

    println!("Current package version: {}", current_version);

    // Determine target version
    let args: Vec<String> = env::args().collect();
    let target_version = if args.len() > 1 && !args[1].trim().is_empty() {
        let val = args[1].trim().to_string();
        println!("Bumping to user-specified version: {}", val);
        val
    } else {
        // Auto bump patch version
        let parts: Vec<&str> = current_version.split('.').collect();
        if parts.len() != 3 {
            eprintln!("Error: Current version '{}' is not in standard X.Y.Z semver format.", current_version);
            exit(1);
        }
        let patch: u32 = parts[2].parse().unwrap_or_else(|_| {
            eprintln!("Error: Patch version part '{}' in '{}' is not an integer.", parts[2], current_version);
            exit(1);
        });
        let new_patch = patch + 1;
        let val = format!("{}.{}.{}", parts[0], parts[1], new_patch);
        println!("Auto-bumping patch version: {} -> {}", current_version, val);
        val
    };

    // 3. Update Cargo.toml content
    let mut new_lines = lines.clone();
    let old_line = lines[version_line_idx];
    let new_line = if let Some(start) = old_line.find('"') {
        if let Some(end) = old_line[start + 1..].find('"') {
            let prefix = &old_line[..start + 1];
            let suffix = &old_line[start + 1 + end..];
            format!("{}{}{}", prefix, target_version, suffix)
        } else {
            format!("version = \"{}\"", target_version)
        }
    } else {
        format!("version = \"{}\"", target_version)
    };

    new_lines[version_line_idx] = &new_line;
    let new_content = new_lines.join("\n") + "\n";

    fs::write(cargo_toml_path, new_content).unwrap_or_else(|err| {
        eprintln!("Error writing {}: {}", cargo_toml_path, err);
        exit(1);
    });
    println!("Updated Cargo.toml successfully.");

    // 4. Update Cargo.lock by running cargo check
    println!("Running 'cargo check' to update Cargo.lock...");
    run_command("cargo", &["check"]);

    // 5. Git commit and tag
    println!("Staging version changes...");
    run_command("git", &["add", "Cargo.toml", "Cargo.lock"]);

    let commit_msg = format!("chore: bump version to {}", target_version);
    println!("Committing changes (excluding git hooks with --no-verify)...");
    run_command("git", &["commit", "-m", &commit_msg, "--no-verify"]);

    let tag_name = format!("v{}", target_version);
    println!("Creating git tag: {}...", tag_name);
    run_command("git", &["tag", "-a", &tag_name, "-m", &format!("Release {}", tag_name)]);

    println!("\n=======================================================");
    println!("SUCCESS: Version bumped, committed, and tagged locally!");
    println!("New Version: {}", target_version);
    println!("Git Tag:     {}", tag_name);
    println!("=======================================================");
    println!("To push the changes and the tag to GitHub, run:");
    println!("  git push origin && git push origin {} --no-verify", tag_name);
    println!("=======================================================");
}
