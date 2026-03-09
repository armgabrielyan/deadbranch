# deadbranch

[![crates.io](https://img.shields.io/crates/v/deadbranch.svg)](https://crates.io/crates/deadbranch)
[![crates.io downloads](https://img.shields.io/crates/d/deadbranch.svg)](https://crates.io/crates/deadbranch)
[![npm version](https://img.shields.io/npm/v/deadbranch)](https://www.npmjs.com/package/deadbranch)
[![npm downloads](https://img.shields.io/npm/dm/deadbranch)](https://www.npmjs.com/package/deadbranch)
[![CI](https://github.com/armgabrielyan/deadbranch/actions/workflows/ci.yml/badge.svg)](https://github.com/armgabrielyan/deadbranch/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**Clean up stale git branches safely.**

## Table of Contents

- [Demo](#demo)
- [Features](#-features)
- [Installation](#-installation)
  - [Quick Install (macOS/Linux)](#-quick-install-macoslinux)
  - [Homebrew (macOS/Linux)](#-homebrew-macoslinux)
  - [npm/npx](#-npmnpx)
  - [Cargo (from source)](#-cargo-from-source)
  - [Manual Download](#️-manual-download)
  - [Build from Source](#-build-from-source)
- [Shell Completions](#-shell-completions)
- [Quick Start](#-quick-start)
- [Usage](#️-usage)
  - [List Stale Branches](#-list-stale-branches)
  - [Delete Stale Branches](#️-delete-stale-branches)
  - [Interactive Mode](#️-interactive-mode)
  - [Dry Run Mode](#-dry-run-mode)
  - [Configuration](#️-configuration)
  - [Backup Management](#-backup-management)
  - [Branch Statistics](#-branch-statistics)
- [Safety Features](#️-safety-features)
- [Restoring Deleted Branches](#️-restoring-deleted-branches)
- [Pattern Matching](#-pattern-matching)
- [Requirements](#-requirements)
- [Roadmap](#️-roadmap)
- [License](#-license)
- [Contributing](#-contributing)

`deadbranch` helps you identify and remove old, unused git branches that clutter your repository. It's designed to be **safe by default** — protecting important branches and requiring explicit confirmation before any deletion.

## Demo

![deadbranch list](https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/demo/list.gif)

## ✨ Features

- 🔍 **List stale branches** — Find branches older than N days (default: 30)
- 🔒 **Safe deletion** — Only deletes merged branches by default
- 🛡️ **Protected branches** — Never touches `main`, `master`, `develop`, `staging`, or `production`
- 🚧 **WIP detection** — Automatically excludes `wip/*` and `draft/*` branches
- 💾 **Backup creation** — Saves deleted branch SHAs for easy restoration
- 👁️ **Dry-run mode** — Preview what would be deleted without making changes
- 🌐 **Local & remote** — Works with both local and remote branches
- 🖥️ **Interactive TUI mode** — Full-screen TUI with Vim navigation, fuzzy search, visual range selection, and 6-column sorting

## 📦 Installation

### ⚡ Quick Install (macOS/Linux)

```bash
curl -sSf https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/install.sh | sh
```

### 🍺 Homebrew (macOS/Linux)

```bash
brew install armgabrielyan/tap/deadbranch
```

### 📦 npm/npx

```bash
# Install globally
npm install -g deadbranch

# Or run directly
npx deadbranch list
```

### 🦀 Cargo (from source)

```bash
cargo install deadbranch
```

### ⬇️ Manual Download

Download pre-built binaries from the [GitHub Releases](https://github.com/armgabrielyan/deadbranch/releases) page.

| Platform | Architecture | Download |
|----------|--------------|----------|
| Linux | x86_64 (glibc) | `deadbranch-VERSION-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | x86_64 (musl/static) | `deadbranch-VERSION-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `deadbranch-VERSION-aarch64-unknown-linux-gnu.tar.gz` |
| macOS | Intel | `deadbranch-VERSION-x86_64-apple-darwin.tar.gz` |
| macOS | Apple Silicon | `deadbranch-VERSION-aarch64-apple-darwin.tar.gz` |
| Windows | x86_64 | `deadbranch-VERSION-x86_64-pc-windows-msvc.zip` |

### 🔨 Build from Source

```bash
git clone https://github.com/armgabrielyan/deadbranch
cd deadbranch
cargo build --release
# Binary will be at target/release/deadbranch
```

## 🐚 Shell Completions

`deadbranch` can generate tab-completion scripts for bash, zsh, and fish.

### Bash

```bash
mkdir -p ~/.local/share/bash-completion/completions
deadbranch completions bash > ~/.local/share/bash-completion/completions/deadbranch
```

Requires bash-completion 2.x to be active. On macOS without Homebrew's bash, source the file manually in `~/.bash_profile`:

```bash
source ~/.local/share/bash-completion/completions/deadbranch
```

### Zsh

```bash
mkdir -p ~/.zfunc
deadbranch completions zsh > ~/.zfunc/_deadbranch
```

Then add the following to your `~/.zshrc` **before** the `compinit` call (or add it if you don't have one):

```zsh
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

Reload your shell or run `exec zsh` to activate.

### Fish

```bash
mkdir -p ~/.config/fish/completions
deadbranch completions fish > ~/.config/fish/completions/deadbranch.fish
```

Fish auto-loads completions from this directory — no extra configuration needed.

## 🚀 Quick Start

```bash
# List all stale branches (older than 30 days)
deadbranch list

# List branches older than 60 days
deadbranch list --days 60

# Preview what would be deleted
deadbranch clean --dry-run

# Delete merged stale branches (with confirmation)
deadbranch clean

# Delete only local branches
deadbranch clean --local

# Show branch health overview
deadbranch stats
```

## 🛠️ Usage

### 📋 List Stale Branches

![deadbranch list](https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/demo/list.gif)

```bash
deadbranch list [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-d, --days <N>` | Only show branches older than N days (default: 30) |
| `--local` | Only show local branches |
| `--remote` | Only show remote branches |
| `--merged` | Only show merged branches |

**Example output:**

```
ℹ Using 'main' as the default branch for merge detection

Local Branches:
┌────┬──────────────────────┬─────────┬────────┬───────┬──────────────┬──────────────┐
│ #  │ Branch               │ Age     │ Status │ Type  │ Last Commit  │ Author       │
├────┼──────────────────────┼─────────┼────────┼───────┼──────────────┼──────────────┤
│ 1  │ feature/old-api      │ 154d    │ merged │ local │ 2024-09-01   │ Jane Doe     │
│ 2  │ bugfix/header-issue  │ 89d     │ merged │ local │ 2024-11-03   │ John Smith   │
└────┴──────────────────────┴─────────┴────────┴───────┴──────────────┴──────────────┘

Remote Branches:
┌────┬─────────────────────────────────┬─────────┬────────┬────────┬──────────────┬──────────────┐
│ #  │ Branch                          │ Age     │ Status │ Type   │ Last Commit  │ Author       │
├────┼─────────────────────────────────┼─────────┼────────┼────────┼──────────────┼──────────────┤
│ 1  │ origin/feature/deprecated       │ 203d    │ merged │ remote │ 2024-07-15   │ Jane Doe     │
└────┴─────────────────────────────────┴─────────┴────────┴────────┴──────────────┴──────────────┘
```

### 🗑️ Delete Stale Branches

![deadbranch clean](https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/demo/clean.gif)

```bash
deadbranch clean [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-d, --days <N>` | Only delete branches older than N days (default: 30) |
| `--merged` | Only delete merged branches (this is the default) |
| `--force` | Force delete unmerged branches (dangerous!) |
| `--dry-run` | Show what would be deleted without doing it |
| `--local` | Only delete local branches |
| `--remote` | Only delete remote branches |
| `-y, --yes` | Skip confirmation prompts (useful for scripts) |

**Safety features:**
- Only deletes **merged** branches by default
- Requires `--force` to delete unmerged branches
- Shows confirmation prompt before deletion
- Extra confirmation for remote branches
- Creates backup file with branch SHAs

**Example:**

```bash
$ deadbranch clean

Local Branches to Delete:
┌────┬──────────────────────┬─────────┬────────┬───────┬──────────────┬──────────────┐
│ #  │ Branch               │ Age     │ Status │ Type  │ Last Commit  │ Author       │
├────┼──────────────────────┼─────────┼────────┼───────┼──────────────┼──────────────┤
│ 1  │ feature/old-api      │ 154d    │ merged │ local │ 2024-09-01   │ Jane Doe     │
│ 2  │ bugfix/header-issue  │ 89d     │ merged │ local │ 2024-11-03   │ John Smith   │
└────┴──────────────────────┴─────────┴────────┴───────┴──────────────┴──────────────┘

Delete 2 local branches? [y/N] y

Deleting local branches...
  ✓ feature/old-api
  ✓ bugfix/header-issue

✓ Deleted 2 local branches
  ↪ Backup: ~/.deadbranch/backups/my-repo/backup-20250201-143022.txt
```

### 🖥️ Interactive Mode

![deadbranch interactive](https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/demo/interactive.gif)

Open a full-screen TUI for browsing, filtering, and selecting branches to delete:

```bash
deadbranch clean -i [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-i, --interactive` | Open interactive TUI for branch selection |
| `-d, --days <N>` | Pre-filter to branches older than N days |
| `--merged` | Start with merged-only filter active |
| `--force` | Unlock unmerged branches for selection |
| `--local` | Start with local-only filter active |
| `--remote` | Start with remote-only filter active |

**Key bindings:**

| Key | Action |
|-----|--------|
| `j`/`↓`, `k`/`↑` | Navigate down/up |
| `gg`, `G` | Jump to top/bottom |
| `Ctrl+d`/`Ctrl+u` | Half-page down/up |
| `Ctrl+f`/`Ctrl+b` | Full-page down/up |
| Mouse scroll | Scroll up/down |
| `Space` | Toggle selection |
| `V` | Visual range select (Vim-style) |
| `a` | Select all merged |
| `A` | Select all (requires `--force`) |
| `n` | Deselect all |
| `i` | Invert selection |
| `d` | Delete selected |
| `/` | Fuzzy search by branch name |
| `s` | Cycle sort (age → branch → status → type → date → author) |
| `S` | Reverse sort direction |
| `m` | Toggle merged-only filter |
| `l` | Toggle local-only filter |
| `R` | Toggle remote-only filter |
| `?` | Help |
| `q`/`Esc` | Quit |

**TUI features:**

- **Vim-style navigation** — `gg`/`G` jump, `Ctrl+d`/`Ctrl+u` page scrolling, relative line numbers
- **Visual range selection** — Press `V` to anchor, move cursor to extend range, `Space` to toggle
- **Fuzzy search** — `/` opens search with matched characters highlighted
- **Age coloring** — Green (<30d), yellow (31–90d), red (>90d)
- **Sections** — Merged and unmerged branches are visually grouped with section headers
- **Progressive deletion** — Live progress bar during batch deletion
- **Smart confirmation** — Simple confirm for safe deletions, typed `yes` for risky ones (unmerged/remote)

**Note:** `-i` cannot be combined with `-y` (skip confirmation) or `--dry-run`.

### 🔍 Dry Run Mode

Preview deletions without making any changes:

```bash
$ deadbranch clean --dry-run

Local Branches to Delete:
┌────┬──────────────────────┬─────────┬────────┬───────┬──────────────┬──────────────┐
│ #  │ Branch               │ Age     │ Status │ Type  │ Last Commit  │ Author       │
├────┼──────────────────────┼─────────┼────────┼───────┼──────────────┼──────────────┤
│ 1  │ feature/old-api      │ 154d    │ merged │ local │ 2024-09-01   │ Jane Doe     │
└────┴──────────────────────┴─────────┴────────┴───────┴──────────────┴──────────────┘

[DRY RUN] Commands that would be executed:
  git branch -d feature/old-api

No branches were actually deleted.
```

### ⚙️ Configuration

![deadbranch config](https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/demo/config.gif)

`deadbranch` stores its configuration in `~/.deadbranch/config.toml`.

```bash
# Show current configuration
deadbranch config show

# Set default age threshold
deadbranch config set days 45

# Set default branch for merge detection
deadbranch config set default-branch main

# Set protected branches
deadbranch config set protected-branches main master develop

# Set exclude patterns
deadbranch config set exclude-patterns "wip/*" "draft/*" "temp/*"

# Open config in your editor
deadbranch config edit

# Reset to defaults
deadbranch config reset
```

**Default configuration:**

```toml
[general]
default_days = 30

[branches]
protected = ["main", "master", "develop", "staging", "production"]
exclude_patterns = ["wip/*", "draft/*", "*/wip", "*/draft"]
```

#### Config keys

| Key | Aliases | Description |
|-----|---------|-------------|
| `days` | `default-days`, `general.default-days` | Default age threshold in days |
| `default-branch` | `branches.default-branch` | Branch used for merge detection (auto-detected if unset) |
| `protected-branches` | `branches.protected` | Branches that are never deleted |
| `exclude-patterns` | `branches.exclude-patterns` | Glob patterns for branches to skip |

### 💾 Backup Management

![deadbranch backup](https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/demo/backup.gif)

Every `deadbranch clean` run automatically creates a backup. Use `deadbranch backup` to manage those backups.

#### List backups

```bash
# Show a summary of all repositories with backups
deadbranch backup list

# Show backups for the current repository
deadbranch backup list --current

# Show backups for a specific repository
deadbranch backup list --repo my-repo
```

#### Restore a deleted branch

```bash
# Restore from the most recent backup
deadbranch backup restore feature/old-api

# Restore from a specific backup file
deadbranch backup restore feature/old-api --from backup-20250201-143022.txt

# Restore with a different name
deadbranch backup restore feature/old-api --as feature/recovered

# Overwrite an existing branch
deadbranch backup restore feature/old-api --force
```

#### Backup statistics

```bash
# Show storage usage per repository and overall
deadbranch backup stats
```

#### Clean up old backups

```bash
# Keep the 10 most recent backups for the current repo (default)
deadbranch backup clean --current

# Keep only the 3 most recent backups
deadbranch backup clean --current --keep 3

# Preview what would be removed
deadbranch backup clean --current --dry-run

# Skip confirmation prompt
deadbranch backup clean --current --yes

# Clean backups for a specific repository by name
deadbranch backup clean --repo my-repo
```

### 📊 Branch Statistics

![deadbranch stats](https://raw.githubusercontent.com/armgabrielyan/deadbranch/main/demo/stats.gif)

Get a health overview of all branches in your repository:

```bash
deadbranch stats [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-d, --days <N>` | Age threshold for "stale" classification (default: from config or 30) |

**Example output:**

```
ℹ Using 'main' as the default branch for merge detection

Repository Statistics:
┌────────────────┬───────┬───────┬────────┐
│ Category       │ Total │ Local │ Remote │
├────────────────┼───────┼───────┼────────┤
│ All branches   │ 12    │ 7     │ 5      │
│ Merged         │ 8     │ 5     │ 3      │
│ Unmerged       │ 4     │ 2     │ 2      │
│ Stale (>30d)   │ 6     │ 4     │ 2      │
│ Safe to delete │ 5     │ 3     │ 2      │
└────────────────┴───────┴───────┴────────┘

Age Distribution:
┌─────────────┬───────┬────────┐
│ Age Range   │ Count │ Status │
├─────────────┼───────┼────────┤
│ < 7 days    │ 2     │ fresh  │
│ 7–30 days   │ 4     │ fresh  │
│ 30–90 days  │ 3     │ stale  │
│ > 90 days   │ 3     │ stale  │
└─────────────┴───────┴────────┘

💡 Run 'deadbranch clean' to remove 5 safe-to-delete branches
```

Stats cover all visible branches (respecting protected and exclude patterns) regardless of age, so `--days` only shifts the stale/safe-to-delete threshold — it doesn't hide branches.

## 🛡️ Safety Features

`deadbranch` is designed to prevent accidental data loss:

| Feature | Description |
|---------|-------------|
| **Merged-only default** | Only deletes branches already merged to main/master |
| **Protected branches** | Never deletes main, master, develop, staging, production |
| **Current branch** | Never deletes the branch you're currently on |
| **WIP detection** | Excludes branches matching `wip/*`, `draft/*`, etc. |
| **Confirmation prompts** | Always asks before deleting |
| **Remote warning** | Extra confirmation for remote deletions |
| **Backup files** | Saves SHA of every deleted branch for restoration |
| **Dry-run mode** | Preview changes without risk |

## ♻️ Restoring Deleted Branches

Every deletion creates a backup file at `~/.deadbranch/backups/<repo>/backup-<timestamp>.txt`.

The backup contains git commands to restore each branch:

```bash
# From the backup file:
git branch feature/old-api abc1234def5678
git branch bugfix/header-issue 987654fedcba
```

You can restore branches manually by running those commands, or use the `deadbranch backup restore` command.

## 🔤 Pattern Matching

Exclude patterns support glob-style wildcards:

| Pattern | Matches |
|---------|---------|
| `wip/*` | `wip/experiment`, `wip/test` |
| `*/draft` | `feature/draft`, `bugfix/draft` |
| `feature/*/temp` | `feature/foo/temp`, `feature/bar/temp` |
| `*test*` | `test`, `testing`, `my-test-branch` |

## 📋 Requirements

- Git (installed and accessible in PATH)
- A git repository (run from within a repo)

## 🗺️ Roadmap

- [x] 🖥️ Interactive TUI mode
- [ ] 👤 `--only-mine` flag for personal branches
- [ ] 🔗 GitHub/GitLab PR detection
- [ ] 📊 Multiple output formats (JSON, CSV)
- [ ] 📁 Per-repo configuration

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=armgabrielyan/deadbranch&type=Date)](https://star-history.com/#armgabrielyan/deadbranch&Date)

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

## 🤝 Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, commit message conventions, testing requirements, and how to submit a pull request.
