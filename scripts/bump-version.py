#!/usr/bin/env python3
import sys
import os
import re
import subprocess

def run_command(cmd, shell=False):
    """Helper to run command and exit if it fails."""
    print(f"Running: {' '.join(cmd) if isinstance(cmd, list) else cmd}")
    res = subprocess.run(cmd, shell=shell)
    if res.returncode != 0:
        print(f"Error: Command failed with exit code {res.returncode}", file=sys.stderr)
        sys.exit(res.returncode)

def main():
    # 1. Run checks first before doing anything
    print("=== Running pre-bump checks ===")
    run_command(["cargo", "fmt", "--all", "--", "--check"])
    run_command(["cargo", "clippy", "--all-targets", "--all-features", "--", "-D", "warnings"])
    run_command(["cargo", "test", "--all-targets", "--all-features"])
    print("=== Pre-bump checks passed successfully! ===\n")

    # 2. Read Cargo.toml
    cargo_toml_path = "Cargo.toml"
    if not os.path.exists(cargo_toml_path):
        print(f"Error: {cargo_toml_path} not found in current directory.", file=sys.stderr)
        sys.exit(1)

    with open(cargo_toml_path, "r") as f:
        content = f.read()

    # Find current version under [package]
    # We look for the first version field which is typically the package version
    match = re.search(r'^version\s*=\s*"([^"]+)"', content, re.MULTILINE)
    if not match:
        print("Error: Could not parse version from Cargo.toml", file=sys.stderr)
        sys.exit(1)

    current_version = match.group(1)
    print(f"Current package version: {current_version}")

    # Determine target version
    target_version = None
    if len(sys.argv) > 1 and sys.argv[1].strip():
        target_version = sys.argv[1].strip()
        print(f"Bumping to user-specified version: {target_version}")
    else:
        # Auto bump patch version (e.g. 0.1.0 -> 0.1.1)
        version_parts = current_version.split(".")
        if len(version_parts) != 3:
            print(f"Error: Version '{current_version}' is not in standard X.Y.Z semver format.", file=sys.stderr)
            sys.exit(1)
        try:
            patch = int(version_parts[2])
            version_parts[2] = str(patch + 1)
            target_version = ".".join(version_parts)
            print(f"Auto-bumping patch version: {current_version} -> {target_version}")
        except ValueError:
            print(f"Error: Patch version part '{version_parts[2]}' in '{current_version}' is not an integer.", file=sys.stderr)
            sys.exit(1)

    # 3. Update Cargo.toml
    # We only replace the first match of version="..." to avoid modifying dependency versions
    new_content, count = re.subn(
        r'^version\s*=\s*"[^"]+"',
        f'version = "{target_version}"',
        content,
        count=1,
        flags=re.MULTILINE
    )
    if count == 0:
        print("Error: Failed to replace version in Cargo.toml", file=sys.stderr)
        sys.exit(1)

    with open(cargo_toml_path, "w") as f:
        f.write(new_content)
    print("Updated Cargo.toml successfully.")

    # 4. Update Cargo.lock by running cargo check
    print("Running 'cargo check' to update Cargo.lock...")
    run_command(["cargo", "check"])

    # 5. Git commit and tag
    print("Staging version changes...")
    run_command(["git", "add", "Cargo.toml", "Cargo.lock"])

    commit_msg = f"chore: bump version to {target_version}"
    print(f"Committing changes (excluding git hooks with --no-verify)...")
    run_command(["git", "commit", "-m", commit_msg, "--no-verify"])

    tag_name = f"v{target_version}"
    print(f"Creating git tag: {tag_name}...")
    run_command(["git", "tag", "-a", tag_name, "-m", f"Release {tag_name}"])

    print("\n=======================================================")
    print("SUCCESS: Version bumped, committed, and tagged locally!")
    print(f"New Version: {target_version}")
    print(f"Git Tag:     {tag_name}")
    print("=======================================================")
    print("To push the changes and the tag to GitHub, run:")
    print(f"  git push origin && git push origin {tag_name} --no-verify")
    print("=======================================================")

if __name__ == "__main__":
    main()
