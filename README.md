English | [简体中文](README-cn.md)

# Lumesh

[![GitHub License](https://img.shields.io/github/license/superiums/lumesh)]()
[![GitHub Repo stars](https://img.shields.io/github/stars/superiums/lumesh)]()
[![GitHub Release](https://img.shields.io/github/v/release/superiums/lumesh)]()

[Codeberg](https://codeberg.org/santo/lumesh)
| [GitHub](https://github.com/superiums/lumesh)
| [Document](https://lumesh.codeberg.page/)
| [DeepWiki](https://deepwiki.com/superiums/lumesh)
| [release-page 1](https://codeberg.com/santo/lumesh/releases)
| [release-page 2](https://github.com/superiums/lumesh/releases)
| [tree-sitter](https://github.com/superiums/tree-sitter-lumesh)


```
     ⚡┓
      ┃ ┓┏┏┳┓┏┓
      ┗┛┗┻┛┗┗┗  lightweight ultimate modern efficient 
```      
**Write as js, works as Bash, run as light**

Lumesh is a modern shell and scripting language, as a bash replacer, it was completely rewritten from Dune, designed for high performance and user-friendly experience.


## The Origin of Lume's Name
**Lume** [lʌmi] means 'light' and symbolizes lightness and speed.

- **L**ightweight

  Lume Shell is a lightweight shell with a clean design and minimal resource usage, ideal for scenarios requiring rapid startup and efficient operation.

- **U**ltimate

  Lume Shell is a powerful tool that delivers a comprehensive command-line experience for advanced users.

- **M**odern

  Lume Shell incorporates contemporary design philosophies and technologies, supporting the latest scripting language features and interactive methods.

- **E**fficient

  Lume Shell excels in command execution and script processing, delivering both high efficiency and rapid response.

## ⚑ Why Choose Lumesh?

### Performance Comparison

| Comparison Item |    lume       |     bash      |     dash      |     fish      |
|------------------|---------------|---------------|---------------|---------------|
| Speed (million loops) |     *****     |     ***       |     ****      |    *          |
| Syntax Friendliness |     *****     |     **        |     *         |    ****       |
| Error Messages |     *****     |     *         |     *         |    ***        |
| Error Handling |     *****     |     *         |     *         |    *          |
| Built-in Libraries |     *****     |               |               |    *       |
| Interactivity |     ****      |     **        |     *         |    *****      |
| Size |     ****      |     ***       |     *****     |    **         |
| Key Bindings |     ☑      |               |               |     ☑         |
| Structured Pipelines |     ☑      |               |               |              |
| AI Interaction |     ☑        |               |               |               |

## ⚑ Core Features

### ⚡ Intuitive Syntax Design
```bash
# Syntax like modern programming languages
let user = {name: "Alice", age: 25}
let {name, age} = user
let numbers = 1..10 | list.filter(x -> x > 5)
let [a, b] = [1, 2]
```

### ⚡ Chained Calls
Supports method chaining similar to object-oriented languages:

```bash
"hello world".split(' ').join(',')
data | .filter(x -> x > 0)
```


### ⚡ Powerful Error Handling
More intelligent error tips, error capture and recovery deeling than traditional shells.

```bash
command ?.        # Ignore errors
command ?: e      # Error capture or default value
command ?+        # Print to standard output
command ??        # Print to error output
command ?>        # Override print (data channel)
command ?!        # Terminate on error (terminate pipeline)
```

### ⚡ Diverse Pipeline Operations
```bash
data | process           # Standard pipeline, supports structured data
data |_ positional       # Positional pipeline
data |> loop_deel        # Loop pipeline
data |^ interactive      # PTY pipeline
```

Structured pipelines:
```bash
ls -l | .to_table() | where(size > 5K)
Fs.ls -l | where(size > 5K) | select(name,size,modified)
ls -1 |> cp -r _ /tmp/
```

### ⚡ Rich Built-in Modules
- **Collection Operations**: `list.reduce, list.map`
- **File System**: `fs.ls, fs.read, fs.write`
- **String Processing**: `string.split, string.join`, `regex` module
- **Time Operations**: `time.now, time.format`
- **Data Conversion**: `into`, `from` module
- **Mathematical Calculations**: Complete `math` function library
- **Logging**: `log` module
- **UI Operations**: `ui.pick, ui.confirm`


### ⚡ Function Decorators
Supports function decorator syntax:

```bash
@decorator_name
@decorator_with_args(param1, param2)
fn my_function() { ... }
```

### ⚡ Module import
Supports module import syntax:

```bash
use moduleA as ma
```

### AI Integration Support
Built-in local AI assistant, supports command completion and smart suggestions.


## ⚑ Quick Start

### Installation Methods

**Method 1: Download Precompiled Version**
- [release-page 1](https://codeberg.com/santo/lumesh/releases)
- [release-page 2](https://github.com/superiums/lumesh/releases)

**Method 2: Build from Cargo**
```bash
cargo install lumesh
```

**Method 3: Compile from Source**
```bash
git clone 'https://codeberg.com/santo/lumesh.git'
cd lumesh
cargo build --release
```

### Experience Immediately
- **`lume`**: Complete shell, supports REPL, auto-completion, syntax highlighting
- **`lume-se`**: Lightweight script executor, quick startup, minimal dependencies.

```bash
# Start interactive shell
lume

# Or execute script
lumesh script.lm
```

### Grammer highlight
- Interactive highlight：already supported within box
- editor highlight：
 [tree-sitter](https://github.com/superiums/tree-sitter-lumesh)
 

## Benchmark Testing

| ![highlight](assets/mem_chart.png) | ![highlight](assets/time_chart.png) |
|------------------------|------------------------|

_Due to fish being unable to complete one million tasks, we recorded its half-task time._

**from v0.10.1 on, lume becomes about 2x faster than before!**

**from v0.11.0 on, lume takes less memery**

## ⚑ Version History
Recent development has emphasized:

- Decorator support for function enhancement
- IFS (Internal Field Separator) mode control for compatibility
- Enhanced module system with cross-linking between modules
- Improved error reporting and debugging capabilities
- CFM (Command First Mode) for daily commands (v0.8.0)
- More friendly help info (v0.8.5)
- Params completion from fish (v0.8.8)
- Automatic completion detection logic optimization (v0.8.8)
- Modular programming support (v0.10.0)
- Built-in library dynamic lazy loading support (v0.10.1)
- More flexible placeholder support (v0.10.2)
- Improved CFM (v0.11.1)
- Middleware-style decorator support (v0.11.5)
- Loop iterator optimization (v0.11.5)
- Local variable support (v0.11.6)
---

![Stargazers over time](https://starchart.cc/superiums/lumesh.svg)

**start your travel with lumesh now**
