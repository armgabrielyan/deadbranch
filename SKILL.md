---
name: deadbranch
description: Safely identify and remove stale git branches with dry-run previews, automatic backups, protected branch lists, and an interactive TUI.
---

# deadbranch — Agent Skill

> **deadbranch** is a CLI tool that safely identifies and removes stale git branches. It protects important branches by default, supports dry-run previews, auto-creates backups before deletion, and includes an interactive TUI with fuzzy search and visual selection.

---
## Capabilities
| Capability | Command |
|---|---|
| List stale branches | `deadbranch list` |
| Delete stale branches | `deadbranch clean` |
| Interactive branch management | `deadbranch clean -i` |
| Repository branch health stats | `deadbranch stats` |
| Manage configuration | `deadbranch config` |
| Manage backups | `deadbranch backup` |
| Generate shell completions | `deadbranch completions <shell>` |

---

## Installation

### Quickest (curl)
```bash
curl -sSf https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/install.sh | sh
```

### Homebrew (macOS / Linux)
```bash
brew install armgabrielyan/tap/deadbranch
```

### npm / npx
```bash
npm install -g deadbranch
# or, without installing:
npx deadbranch list
```

### Cargo (Rust)
```bash
cargo install deadbranch
```

### From Source
```bash
git clone https://github.com/armgabrielyan/deadbranch
cd deadbranch
cargo build --release
# binary at: ./target/release/deadbranch
```

### Verify Installation
```bash
deadbranch --version
```

---

## Quick Start

```bash
# Preview stale branches older than 30 days (safe, no changes)
deadbranch list

# Preview what would be deleted (dry-run)
deadbranch clean --dry-run

# Delete merged stale branches with confirmation
deadbranch clean

# Delete all stale branches (merged + unmerged) — use with care
deadbranch clean --force

# Skip confirmation prompt
deadbranch clean --yes

# Open interactive TUI to select and delete branches manually
deadbranch clean -i
```

---

## Commands

Run `deadbranch <command> --help` to see all available options for any command.

---

## Safety Model

deadbranch is designed to avoid accidental data loss:

1. **Merged-only default** — Only merged branches are deleted unless `--force` is passed.
2. **Protected branches** — `main`, `master`, `trunk`, `develop`, `development`, `staging`, `production` are never touched.
3. **WIP detection** — Branches matching `wip/*`, `draft/*`, `*/wip`, `*/draft` are excluded.
4. **Current branch protection** — The checked-out branch is never deleted.
5. **Confirmation prompts** — Interactive confirmation before deletion; remote deletions require typing `yes`.
6. **Auto-backup** — Branch SHAs are saved to `~/.deadbranch/backups/` before every deletion, allowing restoration.
7. **Dry-run mode** — `--dry-run` previews all changes without modifying anything.

---

## How It Works (High-Level)

```
1. Validate git repository
2. Load ~/.deadbranch/config.toml (create defaults if missing)
3. Auto-detect default branch (remote HEAD → main/master fallback)
4. List local + remote branches with metadata (age, author, SHA)
5. Two-pass merge detection:
   - Pass 1: git branch --merged (fast ancestry check)
   - Pass 2: git merge-tree --write-tree in parallel (squash/rebase merge detection)
6. Apply filters: age, protected, WIP patterns, local/remote, merged
7. Show table of matching branches
8. Confirm with user (or skip with --yes)
9. Create backup file with branch SHAs
10. Delete:
    - Local:  git branch -d / -D
    - Remote: git push origin --delete <branches> (batched)
11. (Interactive mode) Play snap animation during deletion
```

---

## Agent Usage Guide

When using deadbranch as an AI agent in a CI/CD workflow or automation script:

**Safe automation pattern:**
```bash
# Always start with a dry-run to audit
deadbranch clean --dry-run --days 60 --merged

# Then apply with --yes to skip interactive prompt
deadbranch clean --days 60 --merged --yes
```

**Scripting tips:**
- Use `--dry-run` first; check exit code (0 = success, non-zero = error).
- Use `--yes` to suppress confirmation prompts in non-interactive environments.
- Avoid `--force` in automated pipelines unless you have a recovery plan.
- Use `deadbranch backup list --current` to audit the backup before proceeding.
- Shell completions (`deadbranch completions bash`) improve discoverability in interactive sessions.

**Exit codes:** `0` on success, non-zero on error (git errors, permission issues, etc.).

---

## Common Workflows

```bash
# Weekly cleanup of merged branches older than 2 weeks
deadbranch clean --days 14 --merged --yes

# Audit before a release freeze
deadbranch stats
deadbranch list --days 7 --remote

# Clean up after squash-merge heavy workflow (detects squash/rebase merges)
deadbranch clean --days 30 --merged --yes

# Protect additional branches beyond defaults
deadbranch config set protected-branches main master staging release hotfix

# Restore a branch deleted by mistake
deadbranch backup list --current
deadbranch backup restore feature/accidentally-deleted
```

