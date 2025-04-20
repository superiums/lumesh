# Lumesh

Welcome to **Lumesh**, a powerful shell forked from [Dune](https://github.com/adam-mcdaniel/dune)!

Lumesh aims to provide a fast, efficient, and user-friendly command-line experience, enhancing your productivity with a variety of new features.

<img src="https://raw.githubusercontent.com/superiums/lumesh/main/assets/lumesh.png" alt="lumesh" width="160" />

## Table of Contents
- [Wiki](#wiki)
- [Features](#features)
- [Installation](#installation)
- [Getting Started](#getting-started)
- [Contributing](#contributing)
- [License](#license)

## Wiki
For detailed documentation and guides, visit our [Wiki](https://github.com/superiums/lumesh/wiki).

## Features

### Improved Features from Dune
- **Fast and Efficient**: Optimized for speed and resource management.
- **Syntax Highlighting**: Enhanced readability with syntax highlighting for commands.
- **Operator Overloading**: Greater flexibility in command execution.
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

## Installation

You can install Lumesh in two ways:

1. **Download from the Latest Release**: Get the precompiled binaries from our [releases page](https://github.com/superiums/lumesh/releases).
2. **Compile from Source**:
   ```bash
   git clone 'https://github.com/superiums/lumesh.git'
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

## Contributing

We welcome contributions! If you would like to contribute to Lumesh, please check out our [Contributing Guidelines](CONTRIBUTING.md).

## License

Lumesh is licensed under the MIT License. See the [LICENSE](LICENSE) file for more details.

---

Thank you for choosing Lumesh! We hope you enjoy using it as much as we enjoyed building it. If you have any questions or feedback, feel free to reach out through our GitHub repository. Happy scripting!
