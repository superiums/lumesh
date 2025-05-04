# Lumesh - a lighting Shell

### Development is now continuing on [Codeberg](https://codeberg.org/santo/lumesh), with the [GitHub](https://github.com/superiums/lumesh) repository becoming a mirror. Issues & pull requests on GitHub will be ignored from now on.

Welcome to **Lumesh**, a powerful shell forked from [Dune](https://github.com/adam-mcdaniel/dune)!

<img src="https://codeberg.org/santo/lumesh/raw/branch/main/assets/lumesh.png" alt="lumesh"/>

Lumesh aims to provide a fast, efficient, and user-friendly command-line experience, enhancing your productivity with a variety of new features.

- write like `python`/`js`
- works like `bash`
- runs like **light**
- stays like **air**
- flows like **water**

## Table of Contents
- [Wiki](#wiki)
- [Features](#features)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Benchmark](#benchmark)
- [Contributing](#contributing)
- [License](#license)

## Wiki
For detailed documentation and guides, visit our [Wiki](https://codeberg.com/santo/lumesh/wiki).

- Syntax handbook

[syntax-handbook](wiki/syntax.md)

[语法手册](wiki/syntax-cn.md)

## Features

### Improved Features from Dune
- **Fast and Efficient**: Optimized for speed and resource management.
- **Syntax Highlighting**: Enhanced readability with syntax highlighting for commands.
- **Metaprogramming**: Advanced capabilities for dynamic code generation.

### New Features
- **User-Friendly**: Designed with simplicity in mind for a better user experience.
- **Extended Syntax Support**: More comprehensive syntax options for commands.
- **Built-in Modules**: A variety of built-in modules to extend functionality.
- **Environment Variable Management**: Easy handling of environment variables.
- **Login Shell Support**: Seamless integration as a login shell.
- **Command Suggestions**: Intelligent suggestions for commands as you type.
- **Local AI Support**: Leverage AI capabilities for enhanced command execution.

The script parser, executor, and front end have been completely rewritten since version 0.3.0, targeting improved efficiency and flexibility for syntax extension.

Discover more features in our [ChangeLog](CHANGELOG.md).

## Benchmark
```bash
initial_memory = `grep VmRSS /proc/self/status | awk '{print $2}'`
let start=time.stamp-ms();
let sum=0; for i in 0..1000001 {
    sum += i
};
let end=time.stamp-ms();

echo "takes time: " end - start "ms";
# takes time:  612 ms
# bash takes 2224 ms.
```
## Installation

You can install Lumesh in two ways:

1. **Download from the Latest Release**: Get the precompiled binaries from our [releases page](https://codeberg.com/santo/lumesh/releases).
2. **Compile from Source**:
   ```bash
   git clone 'https://codeberg.com/santo/lumesh.git'
   cd lumesh
   cargo build --release
   ```

**Note**: As Lumesh is still in its early stages of development, we recommend using the latest code from the repository rather than the releases, as there are frequent bug fixes and new features added.

## Getting Started

- interactive mode:
After installation, simply run the Lumesh executable:

```bash
# Start Lumesh
lumesh
```

- shell parser mode:

```bash
# Start Lumesh
lumesh /your/script/path/file.lsh
```

or use shebang at the head of your script and run it directly.

- login shell mode:

```bash
# Start Lumesh
chsh -s /lume/install/path
```

## Test script

syntax test: [syntax test script](tests/test.lsh).


## Contributing

We welcome contributions! If you would like to contribute to Lumesh, please check out our [Contributing Guidelines](CONTRIBUTING.md).

## License

Lumesh is licensed under the MIT License. See the [LICENSE](LICENSE) file for more details.

---

Thank you for choosing Lumesh! We hope you enjoy using it as much as we enjoyed building it. If you have any questions or feedback, feel free to reach out through our GitHub repository. Happy scripting!
