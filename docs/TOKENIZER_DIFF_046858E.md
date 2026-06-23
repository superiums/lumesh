# 历史提交 046858e 与当前版本的设计差异

## 核心架构差异

### 1. 上下文判定方式

**046858e（扁平优先级链）**：

- 没有 `Ctx` 枚举。`parse_token()` 是一个固定的 `alt()` 优先级链，每次从行首重新匹配。
- 需要上下文感知的标签函数（`prefix_tag`、`infix_tag`、`postfix_tag` 等）通过 `input.previous_char()` 自行检查前一个字符。
- 例如 `prefix_tag` 会检查前一个字符是否为空白或特定括号：
  ```rust
  if input.previous_char().is_some_and(|c| {
      !c.is_ascii_whitespace() && !matches!(&c, '(' | '[' | '{' | ... )
  }) { return Err(NOT_FOUND); }
  ```

**当前版本（显式 Ctx 驱动）**：

- 引入 `Ctx::Start/Space/Word/Open` 枚举，在循环中维护全局上下文状态。
- `Ctx::after_token()` 根据上一个 token 的**最后一个字符**确定下一个上下文。
- 分发函数（`dot_dispatch`、`minus_dispatch`、`bang_dispatch` 等）根据 `Ctx` 选择不同匹配优先级，而非依赖 `previous_char()` 回溯。

### 2. 符号长度计算

**046858e**：

```rust
fn symbol(input: Input<'_>) -> TokenizationResult<'_> {
    let len = input.chars().take_while(|&c| is_symbol_char(c))
                   .map(char::len_utf8).sum();
    Ok(input.split_at(len))
}
```

使用 `map(char::len_utf8).sum()` 计算字节长度。

**当前版本**：

```rust
fn symbol(input: Input<'_>) -> TokenizationResult<'_> {
    let len = input.chars().take_while(|&c| is_symbol_char(c)).count();
    Ok(input.split_at(len))
}
```

改用 `count()` 计算字符数（split_at 接受字符计数而非字节数，取决于 Input 实现）。

### 3. linebreak 函数签名

**046858e**：`fn linebreak(mut input: Input<'_>)` — 参数为 `mut`，内部修改 `input`。

**当前版本**：`fn linebreak(input: Input<'_>)` — 使用局部绑定而非修改参数。

### 4. comment 匹配规则

**046858e**：`#` 后必须紧跟空格或 `!`（`# ` 或 `#!`）才匹配注释，否则失败。

**当前版本**：`#` 后任意内容直到行尾都是注释（`take_while(|&c| !matches!(c, '\n' | '\r'))`），不再要求 `#` 后必须有空格。

### 5. infix_operator / word_infix_tag

**046858e** 独有：

- `infix_operator()` — 独立解析 `...`、`...=`、`..`、`..=`、`@`、`::` 为中缀运算符
- `word_infix_tag()` — `::` 需要前后都是字母（模块调用）
- `postfix_unit_tag()` — 数字后的单位后缀（`K`、`M`、`G`、`T`、`P`、`B`、`%`）
- `postfix_tag()` — 独立的后缀标签（`.`、`[` 等）
- `custom_operator()` / `custom_tag("__")` — 自定义操作符

**当前版本**：全部移除。`..`/`...` 等统一由 `dot_dispatch` 在 `Ctx::Word` 下作为 `infix_tag` 处理，`@` 和 `::` 不再有特殊中缀处理，单位后缀已移除。

### 6. 数字解析的负号检查

**046858e**：`number_literal` 内部检查 `input.previous_char()` 来决定 `-` 是否为负号的一部分。

**当前版本**：负号 `-` 完全由 `minus_dispatch` 根据 `Ctx` 分发处理，`number_literal` 不再检查前一个字符。

### 7. parse_string_inner 复杂度

**046858e**：

- 使用逐字符循环匹配引号
- 支持 ANSI 转义序列解析（`parse_ansi_sequence`）
- 支持 `\u{...}` Unicode 转义
- 区分 `"`（完整转义）、`'`/`` ` ``（仅 `\'`/`` \` `` 转义）
- 返回多种 `Diagnostic`：`InvalidUnicode`、`InvalidStringEscapes`、`InvalidColorCode`

**当前版本**：

- 使用字节级扫描（`bytes[pos]`），跳过转义更快
- 移除了 ANSI 颜色码、Unicode 转义解析
- 统一用 `pos += 1/2` 跳过普通字符和转义对
- 仅返回 `Valid` / `UnterminatedString`

### 8. 关键字重复

**046858e**：`any_keyword` 中 `export` 出现两次（bug）。

**当前版本**：已修复，`export` 只出现一次。

### 9. 长运算符优先级

**046858e**：`long_operator` 内部使用嵌套 `alt((keyword_tag(...), alt((punctuation_tag(...), ...))))`。

**当前版本**：扁平化为一层 `alt()` 链，先 `keyword_tag` 后 `punctuation_tag`。

### 10. catch_operator

**046858e**：`catch_operator()` 单独作为一个函数，包含 `?+`、`?.`、`??` 等 `?` 系列多字符操作符。

**当前版本**：`question_operator()` 替代，函数名更清晰，功能一致。

### 11. CFM 模式

**046858e**：CFM 模式下 `parse_command_token` 使用 `alt()` 链，无上下文感知。

**当前版本**：CFM 模式下的 `parse_command_token` 增加 `ctx` 参数，区分 `Ctx::Word` 和非 Word 上下文的解析顺序（如 `argument_symbol` 和 `cfm_postfix_operator` 的优先级不同）。

### 12. 导入差异

**046858e**：`use nom::{..., multi::fold_many_m_n, sequence::tuple}; use std::convert::TryFrom; use core::option::Option::None;`

**当前版本**：仅 `use nom::{IResult, branch::alt, error::ParseError};`，移除了未使用的导入。

### 13. 符号字符常量定义

**两者相同**：`is_symbol_char` 的 ASCII 符号字符集未变（`a-z A-Z 0-9 _ ~ ? & # $ - / \`）。

## 总结

| 方面       | 046858e                        | 当前版本               |
| ---------- | ------------------------------ | ---------------------- |
| 上下文管理 | 隐式（`previous_char()` 回溯） | 显式（`Ctx` 枚举驱动） |
| 架构风格   | 扁平优先级链                   | 分派式上下文感知       |
| 注释规则   | `#` 后需空格                   | `#` 后任意内容         |
| 字符串解析 | 完整转义/ANSI/Unicode          | 简化字节扫描           |
| 中缀运算符 | 独立 `infix_operator()`        | 整合到 `dot_dispatch`  |
| 单位后缀   | `K/M/G/T/P/B/%`                | 已移除                 |
| 性能       | 逐字符循环解析字符串           | 字节级扫描             |
| 代码量     | ~1100 行                       | ~1105 行               |
