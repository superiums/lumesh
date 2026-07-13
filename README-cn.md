[English](README.md) | 简体中文

# Lumesh —— 你的下一个默认 Shell

[![GitHub License](https://img.shields.io/github/license/superiums/lumesh)]()
[![GitHub Repo stars](https://img.shields.io/github/stars/superiums/lumesh)]()
[![GitHub Release](https://img.shields.io/github/v/release/superiums/lumesh)]()

[Codeberg](https://codeberg.org/santo/lumesh)
| [GitHub](https://github.com/superiums/lumesh)
| [文档](https://lumesh.codeberg.page/)
| [DeepWiki](https://deepwiki.com/superiums/lumesh)
| [发布页 1](https://github.com/superiums/lumesh/releases)
| [发布页 2](https://codeberg.org/santo/lumesh/releases)
| [语法高亮插件](https://github.com/superiums/tree-sitter-lumesh)

```
     ⚡┓
      ┃ ┓┏┏┳┓┏┓
      ┗┛┗┻┛┗┗┗  Lightweight · Ultimate · Modern · Efficient
```

**像写 JS 一样编写脚本，像 Bash 一样调用命令，像光一样运行。**

Lumesh 是用 Rust 实现的现代 Shell 与脚本语言，专为替代 Bash 而生。
它完全兼容外部命令调用习惯，同时提供现代编程语言的语法体验和结构化数据处理能力。

---

## 为什么要从 Bash 迁移到 Lumesh？

你是否在 Bash 中遇到过这些问题？

```bash
# Bash：字符串比较要加引号，否则报错
if [ "$var" = "hello" ]; then ...

# Bash：数组操作反直觉
arr=(1 2 3)
echo ${arr[@]}

# Bash：错误处理几乎靠 set -e，一旦出错整个脚本崩溃
set -e
some_command || echo "failed"

# Bash：没有结构化数据，处理 JSON/表格要靠 awk/jq
ls -l | awk '{print $5, $9}'
```

**Lumesh 让这些问题通通消失：**

```bash
# lumesh：自然的条件判断
if var == "hello" { ... }

# lumesh：列表操作直观
let arr = [1, 2, 3]
arr | list.map(x -> x * 2)

# lumesh：7种错误处理操作符，精细控制
some_command ?.          # 忽略错误，继续执行
some_command ?: "默认值"  # 出错时使用默认值
some_command ?!          # 出错时终止整个管道

# lumesh：内置结构化数据处理
fs.ls -lh | where(size > 5K) | select(name, size, modified)
```

---

## Bash vs Lumesh 语法速查

| 场景 | Bash | Lumesh |
|------|------|--------|
| 变量赋值 | `name="Alice"` | `let name = "Alice"` |
| 字符串插值 | `echo "Hello $name"` | `` echo `Hello {name}` `` |
| 条件判断 | `if [ "$a" -gt 1 ]; then;do ... done` | `if a > 1 { ... }` |
| 循环 | `for i in $(seq 1 10); do ... done` | `for i in 1..10 { ... }` |
| 函数定义 | `myfunc() { ... }` | `fn myfunc() { ... }` |
| 数组 | `arr=(1 2 3)` | `let arr = [1, 2, 3]` |
| 字典/Map | 需借助 `declare -A` | `let m = {a: 1, b: 2}` |
| 解构赋值 | 不支持 | `let {name, age} = user` |
| 错误忽略 | `command 2>/dev/null \|\| true` | `command ?.` |
| 管道结构化数据 | 不支持（需 jq/awk） | 原生支持 |
| 链式调用 | 不支持 | `"hello".split(' ').join(',')` |
| 模块导入 | 不支持 | `use mylib as lib` |

---

## 迁移指南：三步替换 Bash

### 第一步：安装 Lumesh

**方式一：使用安装脚本（推荐）**
```bash
# 下载并运行安装脚本
curl -LO https://github.com/superiums/lumesh/releases/latest/download/install.sh
bash ./install.sh
```

**方式二：下载预编译二进制**
- [GitHub 发布页](https://github.com/superiums/lumesh/releases)
- [Codeberg 发布页](https://codeberg.org/santo/lumesh/releases)

> 如需命令参数自动补全，请将发布包中的 `data.tgz` 解压到数据目录。

**方式三：从源码编译**
**方式四：通过 Cargo 安装**

---

### 第二步：体验交互式 Shell

安装完成后，直接运行 `lume` 进入交互模式：

```bash
lume
```

你会立刻获得：
- **语法高亮**：命令、变量、字符串实时着色
- **智能补全**：路径、命令、参数（含 fish 风格参数提示）自动补全
- **现代快捷键**：`Ctrl+/` 命令菜单、`Alt+g` 快速目录跳转、`CTRL_SHIFT_f` 快速选中文件...
- **AI 辅助**：内置本地 AI 命令建议

你可以尝试：
- `help` 命令，了解内置命令
- `help doc`，查看在线文档
- 执行常规三方命令
- 使用内置库编写函数、脚本

---

### 第三步：将 Lumesh 设为默认 Shell

在lume中执行
```bash
use lman
lman::chsh()
```

重新登录后，你的终端默认就是 Lumesh。

---

## 迁移现有 Bash 脚本

Lumesh 使用 `.lm` 作为脚本扩展名。迁移 Bash 脚本时，主要改动集中在：

**1. Shebang 行**
```bash
#!/usr/bin/env lumesh
```
> `lumesh` 可选择链接到`lume`或`lume-se`
> `lume-se` 是无交互的轻量脚本执行器，适合 CI/CD 和自动化场景。

**2. 变量声明加 `let`**
```bash
# Bash
NAME="world"

# Lumesh
let NAME = "world"
```

**3. 条件与循环语法**
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

**4. 命令调用无需改动**

Lumesh 的 CFM（命令优先模式）让你像在 Bash 中一样直接调用外部命令：
```bash
git status
docker ps -a
ping 1.1.1.1
chmod +x ./script.lm
```

---

## 语法特性一览

### ⚡ 结构化管道（Bash 没有的能力）
```bash
# 列出链接文件
fs.ls -l | where(type == 'symlink') | select(name, modified)

# 批量复制
ls -1 |> cp -r _ /tmp/backup/
```

### ⚡ 强大的错误处理
```bash
command ?.          # 忽略错误
command ?: "默认"   # 出错返回默认值
command ?+          # 错误信息打印到标准输出
command ??          # 错误信息打印到标准错误
command ?>          # 覆盖输出（数据通道）
command ?!          # 出错立即终止管道
command ?~          # 将错误转为布尔值 false
```

### ⚡ 现代语法
```bash
# 直接数学运算
10 - -6 / 3

# 解构赋值
let {name, age} = {name: "Lumesh", age: 3}
let [a, b, *rest] = [1, 2, 3, 4, 5]

# 箭头函数与高阶函数
let evens = 1...20 | list.filter(x -> x % 2 == 0)
let doubled = evens | .map(x -> x * 2)

# 链式调用
"hello world".split(' ').map(s -> s.to_upper()).join('-')
```

### ⚡ 丰富的内置模块（无需安装第三方工具）
| 模块 | 功能 |
|------|------|
| `list` | map、filter、reduce、sort、unique… |
| `string` | split、join、trim、replace、pad… |
| `fs` | ls、read、write、copy、move… |
| `map` | 映射操作 |
| `table` | 表格操作 |
| `regex` | 正则匹配、替换、提取 |
| `time` | 时间格式化、计算、时区 |
| `math` | 完整数学函数库 |
| `into` / `from` | 数据类型转换 |
| `ui` | 交互式选择、确认对话框 |
| `log` | 结构化日志输出 |
| ... | 使用`help libs`查看更多 |

**常数模块**
- `COLOR`
- `MATH`
- `STYLE`

### ⚡ 函数装饰器
```bash
@log_time
@retry(3)
fn deploy() {
  # 部署逻辑
}
```

### ⚡ 模块化编程
```bash
use ./utils as u

u::my_function()
```

---

## 特能对比

| 对比项目 | lume | bash | dash | fish |
|----------|------|------|------|------|
| 速度（百万次循环） | ★★★★★ | ★★★ | ★★★★ | ★ |
| 语法友好度 | ★★★★★ | ★★ | ★ | ★★★★ |
| 错误提示质量 | ★★★★★ | ★ | ★ | ★★★ |
| 错误处理能力 | ★★★★★ | ★ | ★ | ★ |
| 内置函数库 | ★★★★★ | — | — | ★ |
| 交互体验 | ★★★★★ | ★★ | ★ | ★★★★★ |
| 二进制体积 | ★★★★ | ★★★ | ★★★★★ | ★★ |
| 结构化管道 |  √  | — | — | — |
| AI 辅助 | ✅√  | — | — | — |

| ![内存对比](assets/mem_chart.png) | ![速度对比](assets/time_chart.png) |
|---|---|

> 从 v0.10.1 起，循环性能提升约 2 倍；从 v0.11.0 起，内存占用下降约 0.8 MB。

---

## 多个二进制，按需选择

| 二进制 | 大小 | 适用场景 |
|--------|------|----------|
| `lume` | ~3.9 MB | 日常交互式 Shell，含 REPL、补全、高亮 + 本地http协议的AI辅助 |
| `lume-se` | ~2.7 MB | 脚本执行、CI/CD、嵌入式，快速启动 |
| `lume-ai-https` | ~5.4 MB | 交互Shell + 在线https协议的AI辅助 |

---

## 语法高亮支持

- **终端内**：开箱即用，实时高亮
- **编辑器**：通过 [tree-sitter-lumesh](https://github.com/superiums/tree-sitter-lumesh) 支持 Neovim、Helix 等编辑器

---

## 最灵活的快捷键支持
- 用户可将自定义快捷键绑定到自定义函数
- 该函数可以读取并修改当前输入的命令行
- 这意味着你可以尽情发挥，完成任何你想要的功能
- 
**比如**
- 自动修正输入错误
- 保存/调用 历史目录/书签命令
- 调用`ui`模块，制作菜单/对话框，进行快速跳转
- 调用`xdg-open`快速打开文件
- 针对特定工作区制作特定命令菜单
- ...

---

## 版本亮点

- v0.8.0：CFM 命令优先模式，日常命令无需引号
- v0.10.0：模块化编程支持
- v0.11.5：中间件式装饰器、循环迭代器优化
- v0.11.6：闭包自由变量捕获、局部变量支持
- v0.12.7：HashMap/BTreeSet 数据类型、常量支持（COLOR/STYLE/MATH）
- v0.15.0: 重写编辑器带来更流畅的体验
- v0.15.4：更丰富的自动完成特性
- v0.15.5：重写词法分析器带来更高的解析效率

---

![Star 趋势](https://starchart.cc/superiums/lumesh.svg)

**现在就开始你的 Lumesh 之旅，告别 Bash 的历史包袱。**
