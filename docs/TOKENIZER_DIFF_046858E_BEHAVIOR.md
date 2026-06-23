# 046858e 与当前版本的实际行为差异

从用户角度，相同输入在两个版本中产生的 token 结果不同之处如下：

## 确认的差异

| # | 输入 | 046858e | 当前版本 | 影响 |
|---|------|---------|----------|------|
| 1 | `#foo` | `Symbol("#")` + `Symbol("foo")` | `Comment("#foo")` | `#` 后不需要空格了 |
| 2 | `-42` | `Operator("-")` + `IntegerLiteral("42")` | `StringRaw("-42")` | 行首负数变成了参数字符串 |
| 3 | `-x` | `OperatorPrefix("-")` + `Symbol("x")` | `StringRaw("-x")` | 前缀减号变成了参数 |
| 4 | `a@0` | `Symbol(a)` + `OperatorInfix(@)` + `IntegerLiteral(0)` | `Symbol(a)` + `NotTokenized("@0")` | `@` 索引访问失效 |
| 5 | `@decorator` | `OperatorPrefix(@)` + `Symbol(decorator)` | `NotTokenized("@decorator")` | `@` 装饰器前缀失效 |
| 6 | `mod::func` | `Symbol(mod)` + `OperatorInfix(::)` + `Symbol(func)` | `Symbol(mod)` + `Operator(:)` + `Operator(:)` + `Symbol(func)` | `::` 被拆成两个 `:` |
| 7 | `..`（行首） | 报错（无匹配） | `Operator("..")` | 裸 `..` 现在能识别 |
| 8 | `...`（行首） | 报错 | `Operator("...")` | 裸 `...` 现在能识别 |
| 9 | `..10`（行首） | 报错 | `Operator("..")` + `IntegerLiteral(10)` | 范围前缀数字现在能识别 |

## 无差异（行为一致）

以下常见用例两个版本输出完全相同：

| 输入 | 输出 |
|------|------|
| `a.b` | `Symbol(a) OperatorPostfix(.) Symbol(b)` |
| `a..b` | `Symbol(a) OperatorInfix(..) Symbol(b)` |
| `a...b` | `Symbol(a) OperatorInfix(...) Symbol(b)` |
| `a..=b` | `Symbol(a) OperatorInfix(..=) Symbol(b)` |
| `.5` | `OperatorPrefix(.) IntegerLiteral(5)` |
| `./path` | `StringRaw("./path")` |
| `--flag` | `StringRaw("--flag")` |
| `foo!` | `Symbol(foo) OperatorPostfix(!)` |
| `!x` | `OperatorPrefix(!) Symbol(x)` |
| `a!=b` | `Symbol(a) Operator(!=) Symbol(b)` |
| `arr[0]` | `Symbol(arr) OperatorPostfix([) IntegerLiteral(0) Punctuation(])` |
| `func(arg)` | `Symbol(func) OperatorPostfix(() Symbol(arg) Punctuation())` |
| `x->y` | `Symbol(x) Operator(->) Symbol(y)` |
| `x&&y` | `Symbol("x&&y")`（整个是一个符号） |
| `x||y` | `Symbol(x)` + 错误 |
| `x|>y` | `Symbol(x) Operator(|) Operator(>) Symbol(y)` |
| `x/y` | `Symbol("x/y")` |
| `x%y` | `Symbol(x) Operator(%) Symbol(y)` |
| `x^y` | `Symbol(x) Operator(^) Symbol(y)` |
| `1.5` | `FloatLiteral("1.5")` |
| `1.` | `FloatLiteral("1.")` + `InvalidNumber` |
| `10..20` | `IntegerLiteral(10) OperatorInfix(..) IntegerLiteral(20)` |
| `foo_bar` | `Symbol("foo_bar")` |
| `_`（单独） | `ValueSymbol("_")` |
| `%%{` | `Punctuation("%{")` |
| `??` | `Operator("??")` |
| `a - b` | `Symbol(a) Operator(-) Symbol(b)` |
| `a-b` | `Symbol("a-b")` |
| `let x = 1` | `Keyword(let) Symbol(x) Operator(=) IntegerLiteral(1)` |
| `fn foo() { }` | `Keyword(fn) Symbol(foo) OperatorPostfix(() Punctuation()) Punctuation({) Punctuation(})` |
| `CFM: ls -la --color=auto` | `Symbol(ls) StringRaw(-la) StringRaw(--color=auto)` |

## 差异原因分析

### 1. `#foo` 变成注释
- **046858e**: `comment()` 要求 `# ` 或 `#!` 前缀
- **当前**: `comment()` 匹配 `#` 到行尾任意内容

### 2. 负号 `-42` / `-x` 变成参数
- **046858e**: 扁平 `alt()` 链中 `prefix_operator("-")` 在 `number_literal` 之前，`-x` 匹配为前缀运算符；`number_literal` 内部检查 `previous_char()` 决定是否吞并 `-`
- **当前**: `minus_dispatch(Start/Space)` 中 `argument_symbol` 优先级高于 `prefix_tag("-")` 和 `number_literal`，所以 `-42` 和 `-x` 被 `path_tag("-")` 整体匹配为 `StringRaw`

### 3. `@` 索引/装饰器失效
- **046858e**: 有独立的 `infix_operator()`（含 `infix_tag("@")`）和 `prefix_operator()`（含 `prefix_tag("@")`）
- **当前**: `@` 路由到 `operator_or_symbol()`，其中没有 `@` 的特殊处理，`@` 不是 `is_symbol_char`，最终导致 `NotTokenized`

### 4. `::` 模块调用被拆分
- **046858e**: `word_infix_tag("::")` 要求前后都是字母
- **当前**: `long_operator` 中没有 `::`（只有 `:=`），`short_operator` 中只有单个 `:`，所以 `::` 被拆成两个 `:` 运算符

### 5. 裸 `..` / `...` / `..10` 现在能识别
- **046858e**: `infix_tag("..")` 要求前一个字符是字母/数字/`)`/`]`/`_`，行首不匹配，又没有单独的 `prefix_tag("..")`
- **当前**: `dot_dispatch(Start/Space/Open)` 中 `punct_seq_tag("..")` 不检查前文，直接匹配
