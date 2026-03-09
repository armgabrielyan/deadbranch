# Demo: deadbranch backup commands
# Shows backup list, restore, stats, and clean

Source demo/tapes/settings.tape
Require deadbranch

Output demo/backup.gif

# Setup: create demo repo and run a deletion to generate backups
Hide
Type "export GIT_PAGER=cat && source demo/tapes/setup.sh && cd /tmp/deadbranch-demo"
Enter
Sleep 2s
Type "deadbranch clean --yes && clear"
Enter
Sleep 3s
Show

# List backups for the current repo
Type "deadbranch backup list --current"
Enter
Sleep 4s

# Show backup storage stats
Type "deadbranch backup stats"
Enter
Sleep 4s

# Restore a deleted branch
Type "deadbranch backup restore feature/user-auth"
Enter
Sleep 3s

# Clean up old backups (keep only 1)
Type "deadbranch backup clean --current --keep 1"
Enter
Sleep 2s

# Confirm cleanup
Type "y"
Enter
Sleep 2s
