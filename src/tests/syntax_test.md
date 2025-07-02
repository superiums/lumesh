基于 Lumesh 解析器的实现，我为您编写了复杂的测试表达式，特别关注错误提示的准确性：

### 复杂嵌套结构错误测试 (20个)

```bash
# 正确的嵌套结构
{users: [{name: "alice", age: 30}, {name: "bob", age: 25}]}
func(a + b, {key: [1, 2, 3]}, (x * y))

# 错误的嵌套结构
{users: [{name: "alice", age: 30}, {name: "bob", age: 25}]  # 缺少右大括号
func(a + b, {key: [1, 2, 3}, (x * y))                      # 映射未闭合
[1, [2, [3, [4, 5]]                                        # 多层嵌套数组未闭合
{a: {b: {c: {d: 1}}}                                       # 深度嵌套映射未闭合
func(nested(call(deep(value)))                            # 深度函数调用未闭合
((((1 + 2) * 3) - 4) / 5                                   # 多层括号未闭合
[{key: func(1, 2)}, {other: [a, b, c]]                    # 复杂结构未闭合
{users: [{name: "alice", projects: []}                      # 多层结构部分未闭合
```

### 运算符优先级和结合性错误 (15个)

```bash
# 正确的运算符使用
a + b * c - d / e
(a && b) || (c && d)
x = y + z * w

# 错误的运算符使用
+ a + b                    # 前缀 + 不支持
a + + b                    # 连续运算符
a && && b                  # 连续逻辑运算符
x = = y                    # 连续赋值符
a + b * * c                # 连续乘法符
|| a && b                  # 逻辑或开头
a + b -                    # 运算符结尾
* / + - %                  # 只有运算符
a +++ b                    # 三个连续加号
a --- b                    # 三个连续减号
a ** ** b                  # 连续幂运算符
a << << b                  # 连续位移运算符
a >> >> b                  # 连续位移运算符
a <=> b                    # 不存在的运算符
a ??? b                    # 不存在的三元运算符
```

### 函数调用和参数错误 (18个)

```bash
# 正确的函数调用
func(a, b, c)
obj.method(x, y)
nested.call().chain(arg)

# 错误的函数调用
func(                      # 函数调用未闭合
func(a, b, c              # 参数列表未闭合
func(a,, b)               # 连续逗号
func(, a, b)              # 前导逗号
func(a, b,)               # 尾随逗号
func(a b c)               # 缺少逗号分隔
func((a, b)               # 嵌套括号不匹配
func(a, (b, c)            # 嵌套参数未闭合
func(a + , b)             # 不完整表达式作为参数
func(a, + b)              # 参数中的前缀运算符错误
func(a, b +)              # 参数中的后缀运算符错误
obj.method(               # 方法调用未闭合
obj..method()             # 双点访问符
obj.()                    # 空方法名
.method()                 # 缺少对象
obj.method().             # 链式调用不完整
obj.method()(             # 链式调用括号不匹配
func(func(func(           # 深度嵌套未闭合
```

### 控制流语句错误 (12个)

```bash
# 正确的控制流
if condition { action } else { alternative }
for item in collection { process(item) }
match value { pattern => result }

# 错误的控制流
if condition { action } else    # else 后缺少块
if condition action             # 缺少大括号
if { action }                   # 缺少条件
for item collection { }         # 缺少 in 关键字
for in collection { }           # 缺少变量名
for item in { }                 # 缺少集合
match value { }                 # 缺少匹配分支
match { pattern => result }     # 缺少匹配值
if condition { action           # if 块未闭合
for item in collection {        # for 块未闭合
match value { pattern =>        # match 分支不完整
if condition { } else {         # else 块未闭合
```

### 字符串和模板错误 (10个)

```bash
# 正确的字符串
"hello world"
'raw string'
`template ${variable}`

# 错误的字符串
"unclosed string               # 未闭合的双引号字符串
'unclosed raw string          # 未闭合的单引号字符串
`unclosed template            # 未闭合的模板字符串
"string with \invalid escape" # 无效的转义序列
`template ${unclosed          # 模板中的表达式未闭合
`template ${}}`               # 模板中的空表达式
"string with \u{invalid}"     # 无效的 Unicode 转义
"string with \x"              # 不完整的十六进制转义
`nested ${`template ${var}`}` # 嵌套模板字符串
"string with unmatched \" quote" # 字符串内的引号问题
```

### 变量和赋值错误 (15个)

```bash
# 正确的变量和赋值
let x = 10
let a, b = 1, 2
x := delayed_expression

# 错误的变量和赋值
let = 10                      # 缺少变量名
let x =                       # 缺少赋值值
let x, = 1, 2                # 变量列表不完整
let , b = 1, 2               # 变量列表前导逗号
let x,, y = 1, 2             # 变量列表连续逗号
= 10                         # 缺少变量的赋值
x = = 10                     # 连续赋值符
x := := expression           # 连续延迟赋值符
let x = y =                  # 链式赋值不完整
$                            # 单独的变量前缀
$123                         # 变量名以数字开头
$"string"                    # 变量名为字符串
let x = $                    # 不完整的变量引用
let x = $invalid_var +       # 变量引用后的不完整表达式
x += += 1                    # 连续复合赋值符
```

### 数组和索引错误 (12个)

```bash
# 正确的数组和索引
[1, 2, 3]
arr[0]
arr[1:3]
matrix[i][j]

# 错误的数组和索引
[1, 2, 3                     # 数组未闭合
[1,, 2, 3]                   # 数组中连续逗号
[, 1, 2, 3]                  # 数组前导逗号
[1, 2, 3,]                   # 数组尾随逗号
arr[                         # 索引未闭合
arr]                         # 多余的右方括号
arr[1:                       # 切片不完整
arr[:3                       # 切片缺少右方括号
arr[:]                       # 空切片
arr[1::2]                    # 双冒号切片语法错误
arr[[1, 2]]                  # 嵌套数组作为索引
matrix[i][                   # 多维索引不完整
```

### 管道和重定向错误 (10个)

```bash
# 正确的管道和重定向
cmd1 | cmd2
output > file.txt
input < file.txt

# 错误的管道和重定向
|                            # 单独的管道符
| cmd                        # 管道符开头
cmd |                        # 管道符结尾
cmd | |                      # 连续管道符
cmd | | cmd2                 # 连续管道符
>                            # 单独的重定向符
> file                       # 重定向符开头
cmd >                        # 重定向符结尾
cmd > >                      # 连续重定向符
cmd | > file                 # 管道和重定向混合错误
```

### 注释和空白错误 (8个)

```bash
# 正确的注释
# This is a comment
let x = 10  # Inline comment

# 错误的注释和空白
# Unclosed string in comment "
let x = 10 # Comment with unclosed string "
                             # 只有空白的行
# Comment followed by incomplete expression +
# Comment with invalid escape \invalid
let x = 10 #                 # 注释后的空白
# Multi-line comment that's actually
  not multi-line in Lumesh   # 错误的多行注释理解
```

### 复杂表达式组合错误 (15个)

```bash
# 正确的复杂表达式
result = func(a + b) * arr[i] + {key: value}.key
pipeline = data | filter(x -> x > 0) | map(x -> x * 2)

# 错误的复杂表达式
result = func(a + b) * arr[i] + {key: value}.    # 属性访问不完整
pipeline = data | filter(x -> x > 0) | map(x ->  # lambda 不完整
complex = (a + b) * [1, 2, 3][0] + {key:        # 复合表达式不完整
nested = func(arr[i + ], {key: value})           # 嵌套表达式中的错误
chain = obj.method().property[index].            # 链式访问不完整
expr = (a + b) * (c +                           # 复杂括号表达式不完整
lambda = x -> { x + y +                         # lambda 体不完整
conditional = condition ? true_value :          # 三元运算符不完整
range_expr = 1..10 + 5..                       # 范围表达式不完整
map_access = {a: 1, b: 2}[                     # 映射索引不完整
func_chain = func1().func2().func3(             # 函数链调用不完整
array_slice = arr[1:5][2:                      # 数组切片链不完整
complex_pipe = data | func(x -> x +) | result   # 管道中的错误表达式
nested_call = outer(inner(deep(                 # 深度嵌套调用不完整
mixed_error = [1, 2} + {a: 1] + (func(         # 混合括号类型错误
```

这些测试用例专门设计来验证 Lumesh 解析器在各种复杂错误情况下的错误提示准确性，涵盖了：

1. **语法结构错误**：括号、大括号、方括号不匹配
2. **运算符错误**：连续运算符、无效前缀运算符
3. **函数调用错误**：参数列表问题、嵌套调用错误
4. **控制流错误**：if/for/match 语句的各种语法错误
5. **字符串错误**：未闭合字符串、无效转义序列
6. **变量错误**：赋值语法错误、变量引用错误
7. **复合表达式错误**：多种语法元素组合时的错误 [1](#6-0) 

## Notes

这些测试用例基于 Lumesh 的语法错误类型设计，特别关注了 `SyntaxErrorKind` 中定义的各种错误情况。建议在测试时验证每种错误是否能产生准确、有用的错误信息，帮助用户快速定位和修复语法问题。
