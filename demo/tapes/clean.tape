# Demo: deadbranch clean command
# Shows dry-run preview, then actual deletion with confirmation (local + remote)

Source demo/tapes/settings.tape
Require deadbranch

Output demo/clean.gif

# Setup: create demo repo
Hide
Type "source demo/tapes/setup.sh && cd /tmp/deadbranch-demo && clear"
Enter
Sleep 2s
Show

# First: dry-run to preview what would be deleted
Type "deadbranch clean --dry-run"
Enter
Sleep 5s

# Now: actual deletion with confirmation
Type "deadbranch clean"
Enter
Sleep 3s

# Confirm local branch deletion
Type "y"
Enter
Sleep 3s

# Confirm remote branch deletion (requires typing exact phrase)
Type "delete 4 remote branches"
Enter
Sleep 3s
