### [中文说明-点这里](README-cn.md)
# Lumesh - a lighting Shell

### Development is now continuing on [Codeberg](https://codeberg.org/santo/lumesh), with the [GitHub](https://github.com/superiums/lumesh) repository becoming a mirror. Issues & pull requests on GitHub will be ignored from now on.


Welcome to **Lumesh**, a powerful lighting shell, full rewrite of [Dune](https://github.com/adam-mcdaniel/dune)!

[lumesh](https://codeberg.org/santo/lumesh/raw/branch/main/assets/lumesh.png)

Lumesh aims to provide a fast, efficient, and user-friendly command-line experience, enhancing your productivity with a variety of new features.

- write like `python`/`js`
- works like `bash`
- runs like **light**
- stays like **air**
- flows like **water**

## Why lumesh

| compare |    lume       |     bash      |     dash      |     fish      |
|---------|---------------|---------------|---------------|---------------|
| speed(million circle)    |     *****     |     ***       |     ****      |    *          |
| interactive    |     ****      |     **        |     *         |    *****      |
| sytax    |     *****     |     **        |     *         |    ****       |
| size    |     ****      |     ***       |     *****     |    **         |
| error tips|     *****     |     *         |     *         |    ***        |
| error catch|     *****     |     *         |     *         |    *        |
| builtin Lib  |     *****     |               |               |    *       |
| structured pipe|     ☑     |               |               |              |
| key-bindings|     ☑     |               |               |              |
| AI helper  |     ☑        |               |               |               |

## What is Lumesh?

**Lumesh** (or "lume") is a shell and scripting language designed as a complete rewrite of
`Dune` with substantial improvements. The project aims to be:

- **Fast and efficient**: Optimized for speed and resource management
- **User-friendly**: Designed with improved syntax and error handling
- **Feature-rich**: Includes built-in modules for common operations
- **Compatible**: Works with traditional shell workflows


## Key Features
- **Intuitive Syntax**: More structured and readable than traditional shell scripts
- **Performance-Focused**: Optimized for both interactive and script execution modes
- **Built-in Modules**: Comprehensive library of functionality (fs, string, time, etc.)
- **Powerful Error Handling**: Advanced error catching and recovery mechanisms
- **Structured Pipelines**: Enhanced pipe operations for streams and values
- **AI Integration**: Local AI capabilities for command completion and assistance

The script parser, executor, and front end have been completely rewritten since version 0.3.0, targeting improved efficiency and flexibility for syntax extension.

Discover more features in our [ChangeLog](CHANGELOG.md).


## Wiki
For detailed documentation and guides, visit our [Wiki](https://codeberg.com/santo/lumesh/wiki).

[中文wiki](https://codeberg.org/santo/lumesh/wiki/HOME-cn.md)

[DeepWiki](https://deepwiki.com/superiums/lumesh)

## Benchmark

| ![highlight](assets/mem_chart.png) | ![highlight](assets/time_chart.png) |
|------------------------|------------------------|

_as fish was unable to fishish one million times task, we take the time of its harf task_


## Installation

You can install Lumesh in two ways:

1. **Download from the Latest Release**: Get the precompiled binaries from our
- [release-page 1](https://codeberg.com/santo/lumesh/releases)
- [release-page 2](https://github.com/superiums/lumesh/releases)
2. **Compile from Source**:
   ```bash
   git clone 'https://codeberg.com/santo/lumesh.git'
   cd lumesh
   cargo build --release
   ```

## Getting Started

After installation, simply run the Lumesh executable:

```bash
# Start Lumesh
lume
```

## Contributing

We welcome contributions! If you would like to contribute to Lumesh, please check out our [Contributing Guidelines](CONTRIBUTING.md).

## License

Lumesh is licensed under the MIT License. See the [LICENSE](LICENSE) file for more details.

---

Thank you for choosing Lumesh! We hope you enjoy using it as much as we enjoyed building it. If you have any questions or feedback, feel free to reach out through our GitHub repository. Happy scripting!
