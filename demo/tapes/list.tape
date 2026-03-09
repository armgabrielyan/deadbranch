# Demo: deadbranch list command
# Shows listing stale branches with different filters

Source demo/tapes/settings.tape
Require deadbranch

Output demo/list.gif

# Setup: create demo repo
Hide
Type "source demo/tapes/setup.sh && cd /tmp/deadbranch-demo && clear"
Enter
Sleep 2s
Show

# List all stale branches (default: >30 days)
Type "deadbranch list"
Enter
Sleep 4s

# List with custom age threshold
Type "deadbranch list --days 60"
Enter
Sleep 4s

# List only merged branches
Type "deadbranch list --merged"
Enter
Sleep 4s
