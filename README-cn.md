### [Readme English](README.md)

- 开发现在在 Codeberg [https://codeberg.org/santo/lumesh] 继续进行， GitHub [https://github.com/superiums/lumesh] 仓库将成为镜像。

# Lumesh - 光速 Shell 和脚本语言

**像 js 一样编写，像 Bash 一样工作，像光一样运行**

Lumesh 是一个现代化的 shell 和脚本语言，完全重写自 Dune，专为高性能和用户友好体验而设计。

## ⚑ 为什么选择 Lumesh？

### 性能对比

| 对比项目|    lume       |     bash      |     dash      |     fish      |
|---------|---------------|---------------|---------------|---------------|
| 速度(百万循环)    |     *****     |     ***       |     ****      |    *          |
| 语法友好    |     *****     |     **        |     *         |    ****       |
| 错误提示|     *****     |     *         |     *         |    ***        |
| 错误处理|     *****     |     *         |     *         |    *          |
| 内置库  |     *****     |               |               |    *       |
| 交互    |     ****      |     **        |     *         |    *****      |
| 体积    |     ****      |     ***       |     *****     |    **         |
| 按键绑定|     ☑      |               |               |     ☑         |
| 结构化管道|     ☑      |               |               |              |
| AI交互  |     ☑        |               |               |               |

## ⚑ 核心特性

### ⚡ 直观的语法设计

```bash
# 像现代编程语言一样的语法
let user = {name: "Alice", age: 25}
let numbers = 1..10 | list.filter(x -> x > 5)
let [a, b] = [1, 2]
```


### ⚡ 链式调用
支持类似面向对象语言的链式方法调用：

```bash
"hello world".split(' ').join(',')
data | .filter(x -> x > 0)
```


### ⚡ 强大的错误处理
比传统 shell 更智能的错误提示、错误捕获和处理机制。

```bash
command ?.        # 忽略错误
command ?: e      # 错误捕获 或 默认值
command ?+        # 打印到标准输出
command ??        # 打印到错误输出
command ?>        # 覆盖打印 （数据通道）
command ?!        # 遇错终止  (终止管道)
```


### ⚡ 多样化管道操作
```bash
data | process           # 标准管道,支持结构化数据
data |_ positional       # 位置管道
data |> loop_deel        # 循环管道
data |^ interactive      # PTY 管道
```

结构化管道：
```bash
ls -l | .to_table() | where(size > 5K)
Fs.ls -l | where(size > 5K) | select(name,size,modified)
ls -1 |> cp -r _ /tmp/
```


### ⚡ 丰富的内置模块
- **集合操作**: `List.reduce, List.map`
- **文件系统**: `Fs.ls, Fs.read, Fs.write`
- **字符串处理**: `String.split, String.join`、正则模块、格式化模块
- **时间操作**: `Time.now, Time.format`
- **数据转换**: Into, Parse
- **数学计算**: 完整的数学函数库
- **日志记录**: Log模块
- **UI操作**: `ui.pick, ui.confirm`


### ⚡ 函数装饰器
支持函数装饰器语法：

```bash
@decorator_name
@decorator_with_args(param1, param2)
fn my_function() { ... }
```


### ⚡ 模块导入
支持模块导入语法:

```bash
use moduleA as ma
```


### AI 集成支持
内置本地 AI 助手，支持命令补全和智能建议

## ⚑ 使用场景

### ☘ 交互式 Shell
替代传统 shell，提供现代化的命令行体验：
```bash
# 启动交互式 shell
lume
```

### ☘ 脚本自动化
```bash
#!/usr/bin/env lumesh

# 文件处理脚本
let files = Fs.ls("/data") | where(size > 1MB)
files | List.map(f -> Fs.cp(f, './backup'))
```

### ☘ 系统管理
```bash
# 系统监控和管理
ps -u 1000  u | Into.table() | pprint
```

## ⚑ 快速开始

### 安装方式

**方式一：下载预编译版本**
- [release-page 1](https://codeberg.com/santo/lumesh/releases)
- [release-page 2](https://github.com/superiums/lumesh/releases)

**方式二：从源码编译**
```bash
git clone 'https://codeberg.com/santo/lumesh.git'
cd lumesh
cargo build --release
```

### 立即体验
- **`lume`**: 完整交互式 shell，支持 REPL、自动补全、语法高亮
- **`lumesh`**: 轻量级脚本执行器，快速启动，最小依赖

```bash
# 启动交互式 shell
lume

# 或执行脚本
lumesh script.lm
```


## 基准测试

| ![highlight](assets/mem_chart.png) | ![highlight](assets/time_chart.png) |
|------------------------|------------------------|

_由于fish无法完成一百万次的任务，我们记录了其一半任务的时间_

## ⚑ 学习资源

- [中文Wiki](https://lumesh.codeberg.page)
- [Wiki English](https://lumesh.codeberg.page/en/index)

- **语法手册** [https://lumesh.codeberg.page/zh-cn/syntax]
- **内置函数库** libs [https://lumesh.codeberg.page/zh-cn/libs/index]
- **Bash对比** [https://lumesh.codeberg.page/zh-cn/bash_user_guid]
- **快捷键** [https://lumesh.codeberg.page/zh-cn/keys]

## ⚑ 版本历程
当前版本 **0.6.3**，持续更新中：
- 装饰器支持
- IFS 模式控制
- 性能优化
自 0.3.0 版本起完全重写，专注于效率提升和语法扩展的灵活性。

---

**立即开始您的 Lumesh 之旅吧！**
