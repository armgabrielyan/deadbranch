#!/usr/bin/env bash
# Sets up a temporary git repo with realistic stale branches for VHS demos.
# Usage: source demo/tapes/setup.sh
# Creates repo at /tmp/deadbranch-demo and cds into it.

set -euo pipefail

DEMO_REPO="/tmp/deadbranch-demo"

rm -rf "$DEMO_REPO"
mkdir -p "$DEMO_REPO"
cd "$DEMO_REPO"

git init -b main
git config user.name "Jane Doe"
git config user.email "jane@example.com"

# Initial commit on main
git commit --allow-empty -m "Initial commit"

# Set a clean short prompt for the demo
export PS1="~/my-project $ "

# Helper: create a branch with a backdated commit
create_branch() {
    local branch="$1"
    local days_ago="$2"
    local author="${3:-Jane Doe}"
    local email="${4:-jane@example.com}"
    local merged="${5:-true}"

    git checkout -b "$branch" main 2>/dev/null
    local date
    date=$(date -v-"${days_ago}"d +"%Y-%m-%dT12:00:00" 2>/dev/null || date -d "${days_ago} days ago" +"%Y-%m-%dT12:00:00")

    GIT_AUTHOR_DATE="$date" GIT_COMMITTER_DATE="$date" \
        git commit --allow-empty -m "Work on $branch" \
        --author="$author <$email>"

    if [ "$merged" = "true" ]; then
        git checkout main 2>/dev/null
        git merge --no-ff "$branch" -m "Merge $branch" 2>/dev/null
    fi

    git checkout main 2>/dev/null
}

# Merged stale branches (safe to delete)
create_branch "feature/user-auth"       120 "Jane Doe"    "jane@example.com"  true
create_branch "feature/old-api"         154 "John Smith"  "john@example.com"  true
create_branch "bugfix/header-layout"     89 "Jane Doe"    "jane@example.com"  true
create_branch "refactor/db-queries"      67 "John Smith"  "john@example.com"  true

# Unmerged stale branches
create_branch "feature/experiment"       45 "Jane Doe"    "jane@example.com"  false

# Fresh branches (not stale, < 30 days)
create_branch "feature/new-dashboard"     5 "Jane Doe"    "jane@example.com"  false

# Set up a bare repo as the remote
BARE_REPO="/tmp/deadbranch-demo-remote"
rm -rf "$BARE_REPO"
git clone --bare "$DEMO_REPO" "$BARE_REPO" 2>/dev/null

# Point origin to the bare repo
git remote add origin "$BARE_REPO" 2>/dev/null || git remote set-url origin "$BARE_REPO"

# Push all branches to the remote
git push origin --all 2>/dev/null

# Fetch so we see remote tracking branches
git fetch origin 2>/dev/null

echo "Demo repo ready at $DEMO_REPO"
