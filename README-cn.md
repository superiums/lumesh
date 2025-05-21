### [Readme English](README.md)

# Lumesh - 一个光速Shell

- 开发现在在 Codeberg [https://codeberg.org/santo/lumesh] 继续进行， GitHub [https://github.com/superiums/lumesh] 仓库将成为镜像。

- 从现在开始，GitHub上的问题和拉取请求将被忽略。


欢迎使用 Lumesh，一个强大的照明Shell，完全重写自  [Dune](https://github.com/adam-mcdaniel/dune)!

[lumesh](https://codeberg.org/santo/lumesh/raw/branch/main/assets/lumesh.png)

Lumesh旨在提供快速、高效和用户友好的命令行体验，通过多种新功能提升您的生产力。

 * 像 python/js 一样编写
 * 像 bash 一样工作
 * 像 光 一样运行
 * 像 空气 一样静谧
 * 像 水 一样流动

## 为什么选择Lumesh


| 对比项目|    lume       |     bash      |     dash      |     fish      |
|---------|---------------|---------------|---------------|---------------|
| 速度(百万循环)    |     *****     |     ***       |     ****      |    *          |
| 交互    |     ****      |     **        |     *         |    *****      |
| 语法    |     *****     |     **        |     *         |    ****       |
| 体积    |     ****      |     ***       |     *****     |    **         |
| 错误提示|     *****     |     *         |     *         |    ***        |
| 错误处理|     *****     |     *         |     *         |    *          |
| 内置库  |     *****     |               |               |    *       |
| 结构化管道|     ☑      |               |               |              |
| AI交互  |     ☑        |               |               |               |

## 什么是 Lumesh？

**Lumesh**（或称“lume”）是一种外壳和脚本语言，作为对 `Dune` 的全面重写，具有显著的改进。该项目旨在实现：

- **快速高效**：针对速度和资源管理进行了优化
- **用户友好**：设计了改进的语法和错误处理
- **功能丰富**：包含用于常见操作的内置模块
- **兼容性强**：与传统的 shell 工作流兼容

## 主要特性

- **直观的语法**：比传统的 shell 脚本更结构化和可读
- **性能导向**：针对交互模式和脚本执行模式进行了优化
- **内置模块**：功能全面的库（文件系统、字符串、时间等）
- **强大的错误处理**：先进的错误捕获和恢复机制
- **结构化管道**：增强的流和数值的管道操作
- **AI 集成**：本地 AI 功能用于命令补全和辅助

自版本0.3.0以来，脚本解析器、执行器和前端已完全重写，旨在提高效率和语法扩展的灵活性。

在我们的 ChangeLog [CHANGELOG.md] 中发现更多功能。

## Wiki
有关详细文档和指南，请访问我们的 Wiki [https://codeberg.com/santo/lumesh/wiki/HOME-cn.md]。

wiki-English [https://codeberg.org/santo/lumesh/wiki/HOME]

[DeepWiki](https://deepwiki.com/superiums/lumesh)

## 基准测试

| ![highlight](assets/mem_chart.png) | ![highlight](assets/time_chart.png) |
|------------------------|------------------------|

由于fish无法完成一百万次的任务，我们记录了其一半任务的时间


## 安装

您可以通过两种方式安装Lumesh：
 1. 从最新版本下载：从我们的 发布页面 获取预编译的二进制文件。

- [发布页面1](https://codeberg.com/santo/lumesh/releases).
- [发布页面2](https://github.com/superiums/lumesh/releases)

 2. 从源代码编译：
   ```bash
   git clone 'https://codeberg.com/santo/lumesh.git'
   cd lumesh
   cargo build --release
   ```

## 开始

安装后，只需运行Lumesh可执行文件：
```bash
lume
```

## 贡献
我们欢迎贡献！如果您想为Lumesh做贡献，请查看我们的 贡献指南 [CONTRIBUTING.md]。

## 许可证
Lumesh根据MIT许可证发布。有关更多详细信息，请参阅 LICENSE [LICENSE] 文件。

----------------------------------------
感谢您选择Lumesh！我们希望您能像我们构建它时一样享受使用它。如果您有任何问题或反馈，请随时通过我们的GitHub仓库与我们联系。祝您编程愉快！
