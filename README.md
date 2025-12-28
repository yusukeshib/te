# te (手)

> Your helping hand for editing terminal commands

`te` (Japanese: 手, "hand") is an interactive TUI tool that makes editing commands much easier by separating navigation and editing into two distinct modes. Perfect for tweaking long commands from your shell history or current command line.

## The Problem

Editing long commands in the terminal buffer is frustrating:

```bash
# You have this command in your terminal (maybe from history)
kubectl get pods -l app=asset -o custom-columns='POD:.metadata.name,RS:.metadata.ownerReferences[0].name' -w

# Want to change "asset" to "frontend"?
# - Arrow key through the entire line character by character
# - Easy to accidentally delete quotes, commas, or other syntax
# - Hard to see where you are in a long command
```

## The Solution

`te` gives you a **two-mode interface** inspired by vim:

- **Navigation Mode**: Jump between command components with arrow keys
- **Edit Mode**: Focus on editing a single component without accidentally breaking the rest

Simply prefix your command with `te`:

```bash
te kubectl get pods -l app=asset -o custom-columns='POD:.metadata.name,RS:.metadata.ownerReferences[0].name' -w
```

`te` will:
- 🧩 Parse your command into logical components (base, flags, values)
- 🎯 Let you **navigate between components** with arrow keys (Navigation Mode)
- 📚 Let you **cycle through historical values** you've used before
- ✏️ Let you **edit individual components** safely (Edit Mode)
- 👁️ Show a real-time preview as you make changes
- ⚡ Output the final command when ready

## Features

### 🎯 Two-Mode Interface
Clear separation between navigation and editing, inspired by modal editors:
- **Navigation Mode**: Use arrow keys to jump between command components instantly
- **Edit Mode**: Edit a single component in isolation without breaking the rest
- Visual distinction shows which mode you're in

### 🧩 Component-Based Parsing
Breaks commands into logical pieces:
- Base commands and subcommands
- Flags (`--flag` or `-f`)
- Values associated with flags
- Each component is independently editable

### 📚 History-Aware
Learns from your shell history to supercharge your workflow:
- Automatically retrieves previous values you've used with each flag
- Cycle through historical values with `←/→` keys in Navigation Mode
- Enabled with simple shell integration - see installation below

### 🔧 Universal Wrapper
Works with any CLI command. `te` simply parses your command string - no special support needed from the tool.

### 🚀 Edit, Don't Execute
`te` focuses on helping you build the right command:
- **Shows the final command** instead of executing it
- **Copy-paste friendly** output
- **Safe to experiment** - no accidental command execution

## Installation

```bash
# From source (recommended for now)
git clone https://github.com/yusukeshib/te
cd te
cargo install --path .

# Or build and use directly
cargo build --release
# Binary will be at ./target/release/te
```

### Shell Integration (Strongly Recommended)

Enable shell integration to unlock `te`'s full power, including history-aware value suggestions and direct command execution.

**Zsh** (`~/.zshrc`):
```zsh
eval "$(te init zsh)"
```

**Bash** (`~/.bashrc` or `~/.bash_profile`):
```bash
eval "$(te init bash)"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
te init fish | source
```

With shell integration you get:
- ✅ **History-based value suggestions** - Cycle through previous values you've used with each flag
- ✅ **`te-run` function** - Execute commands directly and add them to history
- ✅ **Zsh only**: `Ctrl+T` keybinding to invoke te on your current command line

**Usage with shell integration:**
```bash
# Use te-run to execute commands
te-run kubectl get pods -l app=myapp

# In Zsh: Type a command and press Ctrl+T to edit it interactively
kubectl get pods -l app=myapp  # Press Ctrl+T here
```

## Usage

### Basic Usage

Simply prefix your existing command with `te`:

```bash
# Edit a kubectl command
te kubectl get pods -l app=myapp -o json

# Edit a docker command
te docker run -d -p 8080:80 --name myapp -e ENV=prod nginx

# Edit an ffmpeg command
te ffmpeg -i input.mp4 -c:v libx264 -crf 23 output.mp4

# Edit the last command from history
te !!
```

### In the TUI

**Navigation Mode** (default):
- `↑/↓` or `j/k`: Jump between command components
- `←/→` or `h/l`: Cycle through historical values for the selected component
- `Enter`: Switch to Edit Mode for the selected component
- `Ctrl+X`: Confirm and output the final command
- `Esc`: Exit te

**Edit Mode** (when editing a component):
- Type to edit the component value
- `Ctrl+X`: Save changes and return to Navigation Mode
- `Esc`: Cancel changes and return to Navigation Mode

## How It Works

1. **Parse**: `te` breaks your command into components (base command, flags, values)
2. **Navigate**: Use `↑/↓` to jump between components instantly
3. **Edit**: Press `Enter` to edit a component, or `←/→` to cycle through historical values
4. **Confirm**: Press `Ctrl+X` to output the final command
5. **Execute**: With shell integration, the command runs automatically and is added to history

## Comparison

`te` takes a unique approach to command editing:

| Tool | Scope | Key Feature |
|------|-------|-------------|
| Terminal default | Any | Character-by-character editing |
| AWS CLI `--cli-auto-prompt` | AWS only | Interactive prompts with AWS-specific knowledge |
| `kube-prompt` | kubectl only | REPL with kubectl auto-completion |
| `trogon` | Click/Typer apps | Auto-generated forms from Python code |
| **`te`** | **Any CLI tool** | **Modal editing: Navigate by component, not by character** |

## Why "te" (手)?

In Japanese, 手 (te) means "hand" - representing:
- 🤝 A helping hand for complex commands
- ✋ Easy to type (just 2 characters)
- 🎌 Honoring the Unix philosophy with a Japanese touch

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details

## Credits

Built with:
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation
- [clap](https://github.com/clap-rs/clap) - Command line argument parsing

---

**Star ⭐ this repo if you find it useful!**
