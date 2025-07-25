### [中文说明-点这里](README-cn.md)

- Development is now continuing on [Codeberg](https://codeberg.org/santo/lumesh), with the [GitHub](https://github.com/superiums/lumesh) repository becoming a mirror. Issues & pull requests on GitHub will be ignored from now on.


# Lumesh - Light-speed Shell and Scripting Language

**Write like js, work like Bash, run like light**

Lumesh is a modern shell and scripting language, completely rewritten from Dune, designed for high performance and user-friendly experience.

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
- **Collection Operations**: `List.reduce, List.map`
- **File System**: `Fs.ls, Fs.read, Fs.write`
- **String Processing**: `String.split, String.join`, regex module, formatting module
- **Time Operations**: `Time.now, Time.format`
- **Data Conversion**: Into, Parse
- **Mathematical Calculations**: Complete math function library
- **Logging**: Log module
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

## ⚑ Use Cases

### ☘ Interactive Shell
Replace traditional shells, providing a modern command-line experience:
```bash
# Start interactive shell
lume
```

### ☘ Script Automation
```bash
#!/usr/bin/env lumesh

# File processing script
let files = Fs.ls("/data") | where(size > 1MB)
files | List.map(f -> Fs.cp(f, './backup'))
```

### ☘ System Management
```bash
# System monitoring and management
ps -u 1000  u | Into.table() | pprint
```

## ⚑ Quick Start

### Installation Methods

**Method 1: Download Precompiled Version**
- [release-page 1](https://codeberg.com/santo/lumesh/releases)
- [release-page 2](https://github.com/superiums/lumesh/releases)

**Method 2: Compile from Source**
```bash
git clone 'https://codeberg.com/santo/lumesh.git'
cd lumesh
cargo build --release
```

### Experience Immediately
- **`lume`**: Complete interactive shell, supports REPL, auto-completion, syntax highlighting
- **`lumesh`**: Lightweight script executor, quick startup, minimal dependencies

```bash
# Start interactive shell
lume

# Or execute script
lumesh script.lm
```

## Benchmark Testing

| ![highlight](assets/mem_chart.png) | ![highlight](assets/time_chart.png) |
|------------------------|------------------------|

_Due to fish being unable to complete one million tasks, we recorded its half-task time._

## ⚑ Learning Resources

- [中文 Wiki](https://lumesh.codeberg.page)
- [Wiki English](https://lumesh.codeberg.page/en/index)
- [DeepWiki](https://deepwiki.com/superiums/lumesh)


- **Syntax Manual** [https://lumesh.codeberg.page/en/syntax]
- **Built-in Function Library** [https://lumesh.codeberg.page/en/libs/index]
- **Bash Comparison** [https://lumesh.codeberg.page/rv/en.html]
- **Hotkeys** [https://lumesh.codeberg.page/en/keys]

## ⚑ Version History
Current version **0.6.3**, continuously updated:
- Decorator support
- IFS mode control
- Performance optimization
Completely rewritten since version 0.3.0, focusing on efficiency improvements and syntax extension flexibility.

---
