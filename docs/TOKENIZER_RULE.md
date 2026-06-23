# LumeSh Tokenizer Design Rules

基于 `src/tokenizer.rs` 当前实现（commit `b6a8187`）。

## 1. Token 类型

| TokenKind | 含义 | 示例 |
|---|---|---|
| `Whitespace` | 空格/制表符 | ` `, `\t` |
| `LineBreak` | 换行/语句分隔符 | `\n`, `;` |
| `Comment` | 注释 | `# ...` |
| `Symbol` | 普通标识符/路径 | `foo`, `a+b`, `a/b` |
| `StringRaw` | 原始字符串/参数 | `'...'`, `--flag`, `./path` |
| `StringLiteral` | 双引号字符串（转义） | `"..."` |
| `StringTemplate` | 反引号模板字符串 | `` `...` `` |
| `Regex` | 正则表达式 | `r'...'` |
| `Time` | 时间字面量 | `t'...'` |
| `IntegerLiteral` | 整数 | `42` |
| `FloatLiteral` | 浮点数 | `3.14` |
| `Keyword` | 关键字 | `let`, `set`, `if`, `fn`, `match` |
| `ValueSymbol` | 预定义值 | `true`, `false`, `none`, `_` |
| `Punctuation` | 标点/括号分隔符 | `(`, `)`, `[`, `]`, `{`, `}`, `,`, `H{`, `M{`, `S{`, `%{` |
| `Operator` | 通用运算符 | `==`, `!=`, `&&`, `\|\|`, `+=`, `->` |
| `OperatorPrefix` | 前缀运算符 | `$var`, `@deco`, `!neg`, `.method` |
| `OperatorPostfix` | 后缀运算符 | `func!`, `(call)`, `obj.` |
| `OperatorInfix` | 中缀运算符 | `..`, `...`, `..=`, `...=`, `::` |

## 2. 上下文（Ctx）设计

tokenizer 并非纯基于优先级的 alt 串联，而是引入了**上下文**概念 —— 根据前一个 token 的最后一个字符决定当前 token 的分类：

```rust
enum Ctx {
    Start,  // 行首 / 初始状态
    Space,  // 前一个 token 以空白结尾（Whitespace / LineBreak / Comment 后）
    Word,   // 前一个 token 以字母数字或 `_` 或闭合括号/引号结尾
    Open,   // 其他（非空白、非单词的符号后）
}
```

**规则核心**：
- **`Ctx::Word`**：前接字母/数字/`_` / `)` / `]` / `}` / `'` / `"` / `` ` ``。此时 `.` 解析为 `OperatorPostfix`（方法调用），`-` 解析为普通 Operator，`(` / `[` 解析为 `OperatorPostfix`（函数调用/索引）。
- **`Ctx::Space` / `Ctx::Start` / `Ctx::Open`**：此时 `.` 解析为路径前缀/参数（如 `./foo`），`-` 解析为 `OperatorPrefix` 或 `argument_symbol`（如 `-flag`）。
- **`Ctx::Start` / `Ctx::Space`** 中，`.` 的首选匹配还包括 `number_literal`（如 `.5`）和 `argument_symbol`（如 `..`, `./path`）。

## 3. 字符分类与符号定义

### 3.1 符号字符（`is_symbol_char`）

ASCII 符号字符包括：`a-z`, `A-Z`, `0-9`, `_`, `~`, `?`, `&`, `#`, `$`, `-`, `/`, `\`

**被排除的 ASCII 符号**（即不会作为符号的一部分，而是触发运算符/标点解析）：
- `+`, `=`, `<`, `>`, `*`, `%`, `^`, `|`, `:`, `@`, `!`, `.`, `,`, `;`, `(`, `)`, `[`, `]`, `{`, `}`, `'`, `"`, ` `` `, 空白字符

### 3.2 `:` 和 `@` 的特殊处理

- **`:`** — 在 `Ctx::Word` 下，`::` 解析为 `OperatorInfix`（模块调用，如 `mod::func`）；单个 `:` 解析为 `Operator`。
- **`@`** — 在非 `Ctx::Word` 下，`@` 解析为 `OperatorPrefix`（装饰器，如 `@deco`）；在 `Ctx::Word` 中作为 `Symbol` 的一部分。

### 3.3 非 ASCII 字符

非 ASCII（Unicode）字符一律解析为 `TokenKind::StringRaw` 的单一 token。

## 4. 空格使用规则

| 场景 | 规则 |
|---|---|
| 空白 token | 连续的空白字符被归为一个 `Whitespace` token |
| 运算符相邻 | `a+1` 中的 `+` 解析为 Operator（不要求两侧空格） |
| 中缀运算符 | `..`, `...`, `..=`, `...=` 必须在 `Ctx::Word` 下且后接字母/数字/`(`/`_`/`-` |
| 关键字约束 | `keyword_tag`: 关键字后不能紧跟符号字符；`keyword_alone_tag`: 关键字后必须是空白；`keyword_alone_or_end`: 关键字后可以是空白或输入结束 |
| 运算符约束 | `operator_tag`: 不允许后跟 ASCII 标点（防止 `+` 合并成 `+=`）|
| 前缀约束 | `prefix_tag`: 必须后跟字母/数字/`(`/`[`/`{`/`$`；`prefix_minus_tag`: 必须后跟字母/数字/`(`/`[`/`{`/`"`/`'`/`` ` ``/`.`/`-`；`@` 在非 Word 上下文下解析为 `OperatorPrefix` |
| 后缀约束 | `postfix_break_tag`: 必须后跟空白或行尾；`postfix_tag`: 前一个字符必须是字母/数字/`)`/`]`/`}`/引号 |
| 中缀前文约束 | `infix_tag`: 前一个字符必须是字母/数字/`)`/`]`/`_` |
| `among_punc_tag` | 匹配必须被空白或标点包围（如 `_` 不能是符号 `foo_bar` 的一部分） |

## 5. 各字符/前缀的分发逻辑

### `.`（dot_dispatch）

| 上下文 | 匹配优先级 |
|---|---|
| **Ctx::Word** | `...=` > `...` > `..=` > `..` > `.`（OperatorPostfix，方法调用） |
| **Ctx::Start / Space / Open** | `..`（punct_seq）> `.`（prefix）> argument_symbol > number_literal > `.` / `..`（ValueSymbol） |

### `-`（minus_dispatch）

| 上下文 | 匹配优先级 |
|---|---|
| **Ctx::Word** | `-=` > `->` > `-`（Operator）> symbol |
| **Ctx::Start / Space** | `-=` > `->` > argument_symbol（如 `--flag`）> `prefix_minus_tag`（`-` 后接字母/数字/括号/引号 → `OperatorPrefix`）> `-`（Operator）> symbol |
| **Ctx::Open** | `prefix_minus_tag`（`-` 后接字母/数字/括号/引号 → `OperatorPrefix`）> number_literal > `-`（Operator）> symbol |

**`prefix_minus_tag`**：匹配 `-` 作为前缀运算符，要求 `-` 后紧跟字母、数字、`(`、`[`、`{`、`"`、`'`、`` ` ``、`.` 或 `-`。这使解析器可以区分：`-42` → 取负，`-arg` → 标志，`-(expr)` → 分组取负。

### `!`（bang_dispatch）

| 上下文 | 匹配优先级 |
|---|---|
| **Ctx::Word** | `!==` > `!=` > `!~:` > `!`（OperatorPostfix）> punctuation |
| **Ctx::Start / Space / Open** | `!==` > `!=` > `!~:` > `!`（OperatorPrefix）> punctuation |

### `?`（question_dispatch）

统一匹配：`?+` / `?.` / `??` / `?>` / `?!` / `?:` / `?~`（Operator）> `?`（Operator）> symbol

### `_`（underscore_dispatch）

`_`（ValueSymbol，仅当被标点或空白包围）> symbol（如 `_foo`）

### `H` / `M` / `S`（try_map_or_symbol）

如果后跟 `{`，则匹配 `H{` / `M{` / `S{`（Punctuation，表示 HashMap/BMap/BSet 字面量）
否则走 alpha_dispatch（keyword / value_symbol / string / symbol）

### `%`（特殊处理）

如果后跟 `{`，则匹配 `%{`（Punctuation，显式 block）
否则走 operator_or_symbol

## 6. CFM（Command First Mode）

单行且不以 `:` 开头时使用 CFM 模式（或输入以 `>` 开头时强制 CFM）：

- **符号解析**：`cfm_parse_symbol` — 读取直到遇到空白、`=`、`>`、括号、`^`、`$`、`!`、`|`、`;`、`.`、`,` 或控制字符
- **运算符**：简化版运算符集，无复杂中缀，主要为管道/比较/箭头
- **前缀**：`.`, `!`, `$`, `@`（装饰器）
- **后缀**：`.`, `!`, `^`, `(`
- **中缀**：`::`（模块调用）
- **参数符号优先**：在非 Word 上下文中优先解析 `argument_symbol`

## 7. 字符串字面量

| 前缀 | TokenKind | 转义处理 |
|---|---|---|
| `"` | StringLiteral | 完整转义（`\n`, `\t`, `\\`, `\"`, `\u{...}` 等） |
| `'` | StringRaw | 仅 `\'` 转义 |
| `` ` `` | StringTemplate | 仅 `` \` `` 转义 |
| `r'` | Regex | 同原始字符串 |
| `t'` | Time | 同原始字符串 |

## 8. 数字字面量

- `-` 前缀仅在 `Ctx::Space` / `Ctx::Start` / `Ctx::Open` 下可作为前缀运算符（通过 `prefix_minus_tag`，后接数字时为负数，后接字母时为标志）
- `0..` 触发 range 运算符时不解析为浮点数
- 整数：`42` → `IntegerLiteral`
- 浮点数：`3.14` → `FloatLiteral`；`3.` → `FloatLiteral`（带 `InvalidNumber` 诊断）

## 9. 续行与注释

- **续行符**：`\` 后直接跟换行符（`\n` 或 `\r\n`）视为续行（Whitespace）
- **注释**：`#` 到行尾的全部内容视为 Comment

## 10. 诊断类型

| Diagnostic | 含义 |
|---|---|
| `Valid` | 正常 |
| `InvalidNumber` | 不完整的数字（如 `3.`） |
| `IllegalChar` | 非法字符 |
| `NotTokenized` | 剩余无法解析的内容 |
| `UnterminatedString` | 字符串未闭合 |