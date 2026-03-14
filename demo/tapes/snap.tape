# Demo: deadbranch Thanos snap dissolution effect
# Short clip: select merged branches, confirm, watch them dissolve

Source demo/tapes/settings.tape
Require deadbranch

Output demo/snap.gif

# Setup: create demo repo
Hide
Type "source demo/tapes/setup.sh && cd /tmp/deadbranch-demo && clear"
Enter
Sleep 2s
Show

# Launch interactive mode
Type "deadbranch clean -i"
Enter
Sleep 3s

# By default all merged branches

# Delete selected
Type "d"
Sleep 1.5s

# Confirm deletion (includes remote, requires typing "yes")
Type@200ms "yes"
Sleep 500ms
Enter
Sleep 6s

# View summary, then exit
Sleep 1s
Type "q"
Sleep 1s
