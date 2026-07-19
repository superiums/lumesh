Lumesh is a modern shell and scripting language with JavaScript-like syntax and Bash-like functionality.

## Core Syntax

### Variables and Assignment

- `let x = 1` `let x,y = 1`- Variable declare
- `x = 1` - Assignment (Scope limited)
- `let x := value` - Declare as Lazy (keeps as symbol)
- `set var = value` - Assign variable of Parent Scope
- `del var` - Delete variable
- `$var` or `var` - Variable access
- `$argv` - script args
- `let {name, age:renamed_age} = user` or `let [a, b, *rest] = [1, 2,3,4]` - Destructuring

NOTE:
  - NEVER use lib name as a var name, eg: `list` `string`
  - vars has implicit type, changed while assign.
  - type annotation not supported!

### Data Structures

- String: `'raw'`,`"escaped\n"`,`` `templated, ${age>18 ? "Mr.":"Dear"} $name !` ``
Note: use '' insteadof "" if no escape needed.

- List: `[1, 2, [3,4]]` or `1...5`
- Set: `S{a,b,c}`
- Range: `1..11` or `1..=10`, `_` for unclosed: `1.._` `_..10`
- BtreeMap: `{key: value, name: 'Alice'}` or `M{ ... }`
- HashMap: `H{ ... }`
- Regex: `r'\w+\d`
- DateTime: `t'2025-8-20'`
- FileSize: `B` `K` `M` `G` `T` `P` after number: `2.5M`
- Integer: `3`
- Float: `0.5` `0.5%`
- Blank: `_` used for blank arg in cmd, unclosed range, end slice, and arg placeholder in pipe
- Boolean: `true` `false`
- None: `none`

Structure Related:

- index/slice: `a[0]`, `a[2.._]`
- property: `obj.prop`
- wildcard: `**/*.jpg`
- type check: `typeof(x) == 'Integer'`

### Functions

- Arrow lambda: `x -> x + 1` or `(x,y) -> x+y`
- lambda support partial application and closure capture
- Named function: `fn name(p1,p2=default,*other) { ... }`
- Call: `name(a,b)` `name! a b`
- Decorators: `@decorator
fn my_func() { ... }`
- Module import: `use module as alias; alias::function()`
- the last expression of a block was returned implicitly, so `return` keyword was optional on last line

### Pipelines

- Standard pipe: `data | process` (supports structured data)
- Dispatch pipe: `data |> function(_)` (loop dispatch like xargs)
- PTY pipe: `data |^ interactive` (for interactive programs)
- Positional pipe: `data | positional a _ c` (placeholders)

### Error Handling

- `expr ?.` - Ignore error
- `expr ?: default` - Error capture or default value
- `expr ?+` - Print error to stdout
- `expr ??` - Print error to stderr
- `expr ?>` - Override print (data channel)
- `expr ?!` - Terminate on error
- `expr ?~` - Convert error to boolean
- `expr ?: handler_func` - Handle error with function/default_value
  use `debug data` or `ddebug data` to debug data structure

### Output Handling

- `&` - run in background
- `&?` - shutdown stderr: `ls not_exists &?`
- `&-` - shutdown stdout
- `&.` - shutdown stdout and stderr
- `&+` - redirect stderr to stdout
- `>>` - append to a file
- `>!` - override to a file

### Statement and Scope

- Separate statements with `;` or newlines
- all loop flows and fuction/lambda was scope isolated
- Use `%{ ... }` for explicit scope isolation

### Control Flow

- `if condition { ... } else { ... }`
- `for item in list/range/wildcard { ... }`
- `while condition { ... }`
- `loop { ... }`
- `match value { pattern => result }`
  `match v {
    1 => "number"
    xx => "symbol/string"
    r'\w' => "regex"
    _ => "default"
}`

- `test ? true : false`
- all flow is an expression, could be assigned to a var

### Method Call and Chaining

- method call
  `string.red(msg)` or `string.red msg` or `string.red! msg`
  `string` is a builtin lib name
- chaining call
  `"hi lume".split(' ').join(',')`
  "hi lume" was recognized as string and the function `split` in lib `string` was called
- pipe method
  `data | .filter(x -> x > 0)`
  the corresponding lib name will be discovered via data. square call is a must.

### Operators

- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `&&`, `||`, `!`
  NOTE: `&&` and `||` is pure logic operator which compute the boolean value.
  if you need 'success and execute' or 'fail and execute' flow control(like in bash), just use error cather instead:
  + `a; b` or `if a ?~ {b}` works like `a && b` of bash
  + `a ?: b` or `if !(a ?~) {b}` works like `a || b` of bash

- String concatenate: `+`
- Variable interpolation: `` `a is $a or {a}` ``,`` `a-b={a-b}` ``, use `\{` if need the raw `{`
- String format: `format('a is {a} b={}',b)`
- contains test for string/list/set/range/map, regex supported: `~:`, `!~:`

### Math Compute

- `1 + 2`, `2 ^ 3` - write directly
- Implicit cast to higher precision or first operand's type: `2+5.1`, `3+'5'`, `'5'+3`
- Extended arithmetic ops support complex data types:`'remain4.2' - 4.2`, `[2,4,6] / 2`, `{a:b,c:d} - c`

### Normal Mode

- bare symbo is not command, but the symbo it self: `ls`, add a blank arg to form a command: `ls _`
- following symbols are allowed in word: `_~?&#$-/\\'`
- to force normal mode add a leading `:`

### Command First Mode

- bare symbo is command: `ls`
- allow more symbols in word, they're not operators: `=`,`.`,`:`,`+`
- to force cmf mode add a leading `>`

### Built-in Modules

usage:
`module_name.method(arg1,arg2)`
`module_name.method arg1 arg2`
`arg1.method(arg2)`
`arg1.method arg2`
`arg1 | module_name.method arg2`
`arg1 | .method(arg2)`
lumesh could recognize the type of arg1 and choose correct module to execute.

modules:

> about,boolean,console,filesize,from,fs,hmap,into,list,log,map,math,rand,regex,set,string,sys,table,time,ui

consts:

> COLOR,MATH,STYLE

## Directions

When providing completions or suggestions:

1. Follow Lumesh syntax conventions
2. Prefer functional style with pipes
3. Use error handling operators appropriately
4. Suggest built-in module functions when applicable
5. Keep completions concise and practical
6. Use `ui` lib instead of `read` while need interactive with user
