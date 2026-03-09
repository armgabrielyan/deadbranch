# Demo GIFs & VHS Tapes

This directory contains the demo GIFs used in the README and the [VHS](https://github.com/charmbracelet/vhs) tape files used to generate them.

## Structure

```
demo/
├── tapes/
│   ├── settings.tape       # Shared VHS settings (theme, font, dimensions)
│   ├── setup.sh            # Creates a temp git repo with realistic stale branches
│   ├── list.tape            # deadbranch list
│   ├── clean.tape           # deadbranch clean (dry-run + deletion)
│   ├── interactive.tape     # deadbranch clean -i (TUI mode)
│   ├── config.tape          # deadbranch config
│   ├── backup.tape          # deadbranch backup (list, stats, restore, clean)
│   └── stats.tape           # deadbranch stats
├── list.gif
├── clean.gif
├── interactive.gif
├── config.gif
├── backup.gif
└── stats.gif
```

## Prerequisites

Install VHS and its dependencies:

```bash
# macOS
brew install charmbracelet/tap/vhs ffmpeg ttyd

# Linux (Debian/Ubuntu)
sudo apt install ffmpeg
go install github.com/nicholasgasior/gont@latest  # for ttyd, or install from source
go install github.com/charmbracelet/vhs@latest

# Or see https://github.com/charmbracelet/vhs#installation
```

You also need `deadbranch` installed and available on your `PATH`.

## Generating GIFs

Run individual tapes from the project root:

```bash
vhs demo/tapes/list.tape
vhs demo/tapes/clean.tape
vhs demo/tapes/interactive.tape
vhs demo/tapes/config.tape
vhs demo/tapes/backup.tape
vhs demo/tapes/stats.tape
```

Or regenerate all GIFs at once:

```bash
for tape in demo/tapes/*.tape; do
    [[ "$(basename "$tape")" == "settings.tape" ]] && continue
    vhs "$tape"
done
```

Each tape outputs its GIF to `demo/<name>.gif`.

## How tapes work

### Shared settings (`settings.tape`)

All tapes source `demo/tapes/settings.tape` for consistent appearance:
- Theme: Catppuccin Mocha
- Font size: 16
- Dimensions: 1400x900
- Typing speed: 50ms

### Test repo setup (`setup.sh`)

Each tape sources `demo/tapes/setup.sh` in a hidden block to create a temporary git repo at `/tmp/deadbranch-demo` with:
- 4 merged stale branches (67-154 days old, two different authors)
- 1 unmerged stale branch (45 days old)
- 1 fresh branch (5 days old, not stale)
- A bare remote repo at `/tmp/deadbranch-demo-remote`

### Writing a new tape

```tape
# Demo: deadbranch <command>
Source demo/tapes/settings.tape
Require deadbranch

Output demo/<command>.gif

# Setup (hidden from recording)
Hide
Type "source demo/tapes/setup.sh && cd /tmp/deadbranch-demo && clear"
Enter
Sleep 2s
Show

# Your demo steps here
Type "deadbranch <command>"
Enter
Sleep 2s
```

See the [VHS documentation](https://github.com/charmbracelet/vhs) for the full list of available commands.
