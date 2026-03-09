# Demo: deadbranch interactive TUI mode
# Shows navigation, search, selection, sorting, and deletion

Source demo/tapes/settings.tape
Require deadbranch

Output demo/interactive.gif

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

# Show help panel
Type "?"
Sleep 3s
Type "?"
Sleep 1s

# Navigate down with j
Type "j"
Sleep 600ms
Type "j"
Sleep 600ms
Type "j"
Sleep 600ms

# Navigate up with k
Type "k"
Sleep 600ms
Type "k"
Sleep 1s

# Toggle selection with Space
Space
Sleep 800ms
Type "j"
Sleep 600ms
Space
Sleep 1s

# Deselect all
Type "n"
Sleep 1s

# Visual range selection
Type "V"
Sleep 800ms
Type "j"
Sleep 600ms
Type "j"
Sleep 600ms
Type "j"
Sleep 800ms
Space
Sleep 1.5s

# Deselect all again before search
Type "n"
Sleep 1s

# Fuzzy search
Type "/"
Sleep 600ms
Type@150ms "old"
Sleep 1.5s
Enter
Sleep 1s

# Clear search (open search again and clear)
Type "/"
Sleep 600ms
Backspace 3
Sleep 600ms
Escape
Sleep 1s

# Cycle sort column
Type "s"
Sleep 1s
Type "s"
Sleep 1s

# Reverse sort direction
Type "S"
Sleep 1s

# Toggle merged-only filter
Type "m"
Sleep 1s
Type "m"
Sleep 1s

# Select all merged branches
Type "a"
Sleep 1.5s

# Delete selected
Type "d"
Sleep 1.5s

# Confirm deletion (includes remote branches, requires typing "yes")
Type@200ms "yes"
Sleep 500ms
Enter
Sleep 4s

# View summary, then exit
Sleep 3s
Type "q"
Sleep 1s
