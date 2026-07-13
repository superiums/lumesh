[English](README.md) | [简体中文](README-cn.md)

# Lumesh — Your Next Default Shell

[![GitHub License](https://img.shields.io/github/license/superiums/lumesh)]()
[![GitHub Repo stars](https://img.shields.io/github/stars/superiums/lumesh)]()
[![GitHub Release](https://img.shields.io/github/v/release/superiums/lumesh)]()

[Codeberg](https://codeberg.org/santo/lumesh)
| [GitHub](https://github.com/superiums/lumesh)
| [Documentation](https://lumesh.codeberg.page/)
| [DeepWiki](https://deepwiki.com/superiums/lumesh)
| [Release Page 1](https://github.com/superiums/lumesh/releases)
| [Release Page 2](https://codeberg.com/santo/lumesh/releases)
| [Syntax Highlighting Plugin](https://github.com/superiums/tree-sitter-lumesh)

```
     ⚡┓
      ┃ ┓┏┏┳┓┏┓
      ┗┛┗┻┛┗┗┗  Lightweight · Ultimate · Modern · Efficient
```

**Write scripts like JavaScript, call commands like Bash, run at light speed.**

![lume demo](assets/demo.gif)

Lumesh is a modern shell and scripting language implemented in Rust, designed specifically as a Bash replacement.
It maintains full compatibility with external command calling habits while providing modern programming language syntax and structured data processing capabilities.

---

## Why Migrate from Bash to Lumesh?

Have you encountered these problems in Bash?

```bash
# Bash: String comparison requires quotes, otherwise errors occur
if [ "$var" = "hello" ]; then ...

# Bash: Array operations are counterintuitive
arr=(1 2 3)
echo ${arr[@]}

# Bash: Error handling relies almost entirely on set -e, once an error occurs the entire script crashes
set -e
some_command || echo "failed"

# Bash: No structured data, processing JSON/tables requires awk/jq
ls -l | awk '{print $5, $9}'
```

**Lumesh makes all these problems disappear:**

```bash
# lumesh: Natural conditional judgment
if var == "hello" { ... }

# lumesh: Intuitive list operations
let arr = [1, 2, 3]
arr | list.map(x -> x * 2)

# lumesh: 7 error handling operators for fine-grained control
some_command ?.          # Ignore errors, continue execution
some_command ?: "default"  # Use default value on error
some_command ?!          # Terminate entire pipeline on error

# lumesh: Built-in structured data processing
fs.ls -lh | where(size > 5K) | select(name, size, modified)
```

---

## Bash vs Lumesh Syntax Quick Reference

| Scenario | Bash | Lumesh |
|----------|------|--------|
| Variable assignment | `name="Alice"` | `let name = "Alice"` |
| String interpolation | `echo "Hello $name"` | `` echo `Hello {name}` `` |
| Conditional | `if [ "$a" -gt 1 ]; then;do ... done` | `if a > 1 { ... }` |
| Loop | `for i in $(seq 1 10); do ... done` | `for i in 1..10 { ... }` |
| Function definition | `myfunc() { ... }` | `fn myfunc() { ... }` |
| Array | `arr=(1 2 3)` | `let arr = [1, 2, 3]` |
| Dictionary/Map | Requires `declare -A` | `let m = {a: 1, b: 2}` |
| Destructuring | Not supported | `let {name, age} = user` |
| Error ignore | `command 2>/dev/null \|\| true` | `command ?.` |
| Structured data in pipes | Not supported (requires jq/awk) | Native support |
| Method chaining | Not supported | `"hello".split(' ').join(',')` |
| Module import | Not supported | `use mylib as lib` |

---

## Migration Guide: Three Steps to Replace Bash

### Step 1: Install Lumesh

**Method 1: Using Installation Script (Recommended)**
```bash
# Download and run installation script
curl -LO https://github.com/superiums/lumesh/releases/latest/download/install.sh
bash ./install.sh
```

**Method 2: Download Precompiled Binary**
- [GitHub Release Page](https://github.com/superiums/lumesh/releases)
- [Codeberg Release Page](https://codeberg.org/santo/lumesh/releases)

> For command parameter auto-completion, extract `data.tgz` from the release package to the data directory.

**Method 3: Compile from Source**
**Method 4: Install via Cargo**

---

### Step 2: Experience Interactive Shell

After installation, run `lume` directly to enter interactive mode:

```bash
lume
```

You'll immediately get:
- **Syntax highlighting**: Real-time coloring of commands, variables, and strings
- **Smart completion**: Auto-completion for paths, commands, and parameters (including fish-style parameter hints)
- **Modern hotkeys**: `Ctrl+/` command menu, `Alt+g` quick directory jump, `CTRL_SHIFT_f` quick file selection...
- **AI assistance**: Built-in local AI command suggestions

You can try:
- `help` command to understand built-in commands
- `help doc` to view online documentation
- Execute regular third-party commands
- Use built-in libraries to write functions and scripts

---

### Step 3: Set Lumesh as Default Shell

Execute in lume:
```bash
use lman
lman::chsh()
```

After re-login, your terminal default will be Lumesh.

---

## Migrating Existing Bash Scripts

Lumesh uses `.lm` as the script extension. When migrating Bash scripts, the main changes focus on:

**1. Shebang line**
```bash
#!/usr/bin/env lumesh
```
> `lumesh` can be linked to either `lume` or `lume-se`
> `lume-se` is a non-interactive lightweight script executor, suitable for CI/CD and automation scenarios.

**2. Add `let` to variable declarations**
```bash
# Bash
NAME="world"

# Lumesh
let NAME = "world"
```

**3. Conditional and loop syntax**
```bash
# Bash
for f in *.txt; do
  echo "$f"
done

# Lumesh
for f in *.txt {
  print f
}
```

**4. Command calls require no changes**

Lumesh's CFM (Command First Mode) allows you to call external commands directly just like in Bash:
```bash
git status
docker ps -a
ping 1.1.1.1
chmod +x ./script.lm
```

---

## Syntax Features Overview

### ⚡ Structured Pipelines (Capabilities Bash Doesn't Have)
```bash
# List symlink files
fs.ls -l | where(type == 'symlink') | select(name, modified)

# Batch copy
ls -1 |> cp -r _ /tmp/backup/
```

### ⚡ Powerful Error Handling
```bash
command ?.          # Ignore errors
command ?: "default"   # Return default value on error
command ?+          # Print error message to standard output
command ??          # Print error message to standard error
command ?>          # Override output (data channel)
command ?!          # Terminate pipeline immediately on error
command ?~          # Convert error to boolean false
```

### ⚡ Modern Syntax
```bash
# Direct mathematical operations
10 - -6 / 3

# Destructuring assignment
let {name, age} = {name: "Lumesh", age: 3}
let [a, b, *rest] = [1, 2, 3, 4, 5]

# Arrow functions and higher-order functions
let evens = 1...20 | list.filter(x -> x % 2 == 0)
let doubled = evens | .map(x -> x * 2)

# Method chaining
"hello world".split(' ').map(s -> s.to_upper()).join('-')
```

### ⚡ Rich Built-in Modules (No Third-Party Tools Required)
| Module | Functionality |
|--------|--------------|
| `list` | map, filter, reduce, sort, unique… |
| `string` | split, join, trim, replace, pad… |
| `fs` | ls, read, write, copy, move… |
| `map` | Mapping operations |
| `table` | Table operations |
| `regex` | Regex matching, replacement, extraction |
| `time` | Time formatting, calculation, timezone |
| `math` | Complete mathematical function library |
| `into` / `from` | Data type conversion |
| `ui` | Interactive selection, confirmation dialogs |
| `log` | Structured logging |
| ... | Use `help libs` to see more |

**Constant Modules**
- `COLOR`
- `MATH`
- `STYLE`

### ⚡ Function Decorators
```bash
@log_time
@retry(3)
fn deploy() {
  # Deployment logic
}
```

### ⚡ Modular Programming
```bash
use ./utils as u

u::my_function()
```

---

## Performance Comparison

| Comparison Item | lume | bash | dash | fish |
|-----------------|------|------|------|------|
| Speed (million loops) | ★★★★★ | ★★★ | ★★★★ | ★ |
| Syntax friendliness | ★★★★★ | ★★ | ★ | ★★★★ |
| Error message quality | ★★★★★ | ★ | ★ | ★★★ |
| Error handling capability | ★★★★★ | ★ | ★ | ★ |
| Built-in function library | ★★★★★ | — | — | ★ |
| Interactive experience | ★★★★★ | ★★ | ★ | ★★★★★ |
| Binary size | ★★★★ | ★★★ | ★★★★★ | ★★ |
| Structured pipelines |  √  | — | — | — |
| AI assistance | ✅√  | — | — | — |

| ![Memory comparison](assets/mem_chart.png) | ![Speed comparison](assets/time_chart.png) |
|---|---|

> From v0.10.1, loop performance improved by about 2x; from v0.11.0, memory usage decreased by about 0.8 MB.

---

## Multiple Binaries, Choose as Needed

| Binary | Size | Use Case |
|--------|------|----------|
| `lume` | ~3.9 MB | Daily interactive shell, includes REPL, completion, highlighting + local HTTP protocol AI assistance |
| `lume-se` | ~2.7 MB | Script execution, CI/CD, embedded, fast startup |
| `lume-ai-https` | ~5.4 MB | Interactive shell + online HTTPS protocol AI assistance |

---

## Syntax Highlighting Support

- **In terminal**: Out-of-the-box, real-time highlighting
- **In editors**: Via [tree-sitter-lumesh](https://github.com/superiums/tree-sitter-lumesh) supporting Neovim, Helix, and other editors

---

## Most Flexible Hotkey Support
- Users can bind custom hotkeys to custom functions
- This function can read and modify the currently entered command line
- This means you can unleash your creativity to accomplish any functionality you want

**For example**
- Automatically correct input errors
- Save/call history directory/bookmark commands
- Call the `ui` module to create menus/dialogs for quick navigation
- Call `xdg-open` to quickly open files
- Create specific command menus for specific workspaces
- ...

---

## Version Highlights

- v0.8.0: CFM (Command First Mode), daily commands without quotes
- v0.10.0: Modular programming support
- v0.11.5: Middleware-style decorators, loop iterator optimization
- v0.11.6: Closure free variable capture, local variable support
- v0.12.7: HashMap/BTreeSet data types, constant support (COLOR/STYLE/MATH)
- v0.15.0: Rewritten editor for smoother experience
- v0.15.4: Richer auto-completion features
- v0.15.5: Rewritten lexer for higher parsing efficiency

---

![Star Trend](https://starchart.cc/superiums/lumesh.svg)

**Start your Lumesh journey now and say goodbye to Bash's historical baggage.**
