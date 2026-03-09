# Demo: deadbranch stats command
# Shows repository branch statistics and age distribution

Source demo/tapes/settings.tape
Require deadbranch

Output demo/stats.gif

# Setup: create demo repo
Hide
Type "source demo/tapes/setup.sh && cd /tmp/deadbranch-demo && clear"
Enter
Sleep 2s
Show

# Show branch statistics
Type "deadbranch stats"
Enter
Sleep 5s
