# Demo: deadbranch config command
# Shows config show, set, and reset

Source demo/tapes/settings.tape
Require deadbranch

Output demo/config.gif

# Setup: create demo repo
Hide
Type "source demo/tapes/setup.sh && cd /tmp/deadbranch-demo && clear"
Enter
Sleep 2s
Show

# Show current configuration
Type "deadbranch config show"
Enter
Sleep 4s

# Set a custom age threshold
Type "deadbranch config set days 45"
Enter
Sleep 2s

# Add a custom exclude pattern
Type@80ms `deadbranch config set exclude-patterns "wip/*" "draft/*" "temp/*"`
Enter
Sleep 2s

# Verify the changes
Type "deadbranch config show"
Enter
Sleep 4s

# Reset to defaults
Type "deadbranch config reset"
Enter
Sleep 1s

# Confirm reset
Type "y"
Enter
Sleep 2s
