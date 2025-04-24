**Lumesh Shell Syntax Manual**

---

## I. Basic Syntax Structure

1. **Variable Declaration and Assignment**
   - **Declare Variables**: Use the `let` keyword, supporting multiple variable declarations.
     ```bash
     let x = 10                 # Single variable
     let a, b = 1, "hello"      # Multiple variables with separate assignments (the number of expressions on the right must match)
     let a, b, c = 1            # Multiple variables with a unified assignment
     let a = b = 1              # Multiple variables with a unified assignment
     ```

     Using `let`, create an alias:
     ```bash
     let cat = bat
     ```

     With `let`, you can define variables as strings, functions, macros, commands, and even custom operators:
     ```bash
     let _++ = x -> x + 1;
     _++ 3
     ```

   - **Assignment Operators**:
     - `=`: Standard assignment.
     - `:=`: Delayed assignment (stores the expression as a reference).
     ```bash
     x := 2 + 3  # Delayed evaluation, storing the expression rather than the result
     echo x      # Outputs the expression: 2+3
     eval x      # Outputs the result: 5
     ```
   - **Delete Variable**: Use `del`.
     ```bash
     del x
     ```
   - **Using Variables**: No need for `$`, simply use `echo a x`.

   **Edge Cases**:
   - In strict mode (`-s`), variables must be initialized; otherwise, an error will occur.
   - When declaring multiple variables, the number of expressions on the right must match the number of variables on the left or be a unified value; otherwise, a `SyntaxError` will be thrown.

2. **Data Types**
   - **Basic Types**:
     | Type       | Example                     |
     |------------|-----------------------------|
     | Integer    | `42`, `-3`                  |
     | Float      | `3.14`, `-0.5`              |
     | String     | `"Hello\n"`, `'raw'`        |
     | Boolean    | `True`, `False`             |
     | List       | `[1, "a", True]`            |
     | Map (Dict) | `{name: "Alice", age: 30}`  |
     | `None`     | `None`                       |

   - **Complex Types**:
     | Type       | Example                     |
     |------------|-----------------------------|
     | Variable   | `x`                         |
     | Function    | `fn add(x,y){return x+y}`  |
     | Lambda     | `let add = x -> y -> x + y` |
     | Built-in   | `math@floor`               |

   - **String Rules**:
     - Double quotes support escape sequences (e.g., `\n`), while single quotes are raw strings.

     ```bash
     let str = "Hello\nworld!"
     let str2 = 'Hello world!'
     ```

   **Edge Cases**:
   - In single-quoted strings, `\` is treated literally (e.g., `'Line\n'` outputs `Line\n`).

3. **Scope Rules**
   - Lambda and function definitions are treated as sub-environments.
   - Sub-environments inherit parent environment variables without modifying the parent scope.

---

## II. Operator Rules

### 1. Operator Classification and Precedence

**Precedence from highest to lowest** (lower numbers indicate higher precedence)

| Precedence | Operator/Structure         | Example/Description         |
|------------|-----------------------------|------------------------------|
| 1          | Parentheses `()`            | `(a + b) * c`               |
| 2          | Function calls, Lists `[]`  | `func arg`, `[1, 2]`        |
| 3          | Unary operators `!`, `-`, `__..` | `!flag`, `-5`               |
| 4          | Exponentiation `**`         | `2 ** 3`                    |
| 5          | Multiplication/Division/Modulus `*`, `/`, `%`, `_*..` | `a * b % c`            |
| 6          | Addition/Subtraction `+`, `-`, `_+..` | `a + b - c`            |
| 7          | Comparison `==`, `!=`, `>` etc. | `a > b`                   |
| 8          | Logical AND `&&`            | `cond1 && cond2`            |
| 9          | Logical OR `||`             |                              |
| 10         | Conditional operator `?:`    | `cond ? t : f`              |
| 11         | Assignment `=`, `:=`        | `x = 5`, `let y := 10`      |
| 12         | Pipe `|`                    | `ls | sort`                 |
| 13         | Redirection `<<`, `>>`, `>>>` | `date >> /tmp/day.txt`     |

### 2. Space Rules
| Operator Type         | Requires Spaces?           | Example                    |
|-----------------------|---------------------------|----------------------------|
| Regular Operators      | In strict mode, spaces required on both sides | `a + b`, `x <= 10`       |
|                       | In non-strict mode, spaces can be omitted | `a+b`, `x<=10`, `a=5`    |
|                       | `-` and `/` exceptions    | `b-3` is a string, `3-b` is subtraction |
| Single Character Operators | Spaces allowed (context must be clear) | `++x`, `arr@0`          |
| Custom Operators       | Must start with an underscore and have spaces on both sides | `x _*+ y`, `a _?= b`    |

**Custom Operators**
- Custom operators start with `_` and can only contain symbols, not numbers or letters.
- Custom unary operators start with `__`, e.g., `__+`, with precedence equal to unary operators.
- Custom binary operators start with `_+`, e.g., `_+%`, with precedence equal to `+` and `-`.
- Custom multiplication operators start with `_*`, e.g., `_*-`, with precedence equal to `*` and `/`.

**Edge Cases**:
- `x++y` is illegal, while `x + (++y)` is legal.
- In non-strict mode, `b-3` is a string, while `3-b` is subtraction.

---

### 3. Implicit Type Conversion
```bash
# Non-strict mode
3 + "5"    # → "35" (automatically converts to string)
"10" * 2   # → 20

# Strict mode (enabled with -s)
3 + "5"    # → TypeError: cannot add string and number
"10" * 2   # → "1010"
```

### 4. Special Operation Behavior
```bash
"5" + "3"  # → "53"
"5" * 3    # → "555" (non-strict mode)
5 / 2      # → 2 (integer division)
5.0 / 2    # → 2.5
```
_You can use `*` to repeat strings and lists multiple times, similar to Python usage._
`echo "+" * 3`
`[3,5,7] * 3`

**Edge Cases**:
- Division by zero will throw an error.

### 5. Pipe
- `|` pipes to standard output.
- `|>` pipes to the last parameter.

### 6. Redirection
- `<<` for reading.
- `>>` for output.
- `>>>` for appending output.

`1 + 2 >> result.txt`

---

## III. Array and Dictionary Indexing

### 1. Array Operations
```bash
let arr = [10, "a", True]

# Basic Indexing
arr@0
arr.1
arr[1]

arr[0]       # → 10
arr[-1]      # → True (supports negative indexing)

# Slicing
arr[1:3]     # → ["a", True] (left-inclusive, right-exclusive)
arr[::2]     # → [10, True] (step of 2)

# Modify Element
# arr[2] = 3.14 # → [10, "a", 3.14]
```

### 2. Dictionary Operations
```bash
let dict = {name: "Alice", age: 30}
let dict = {name="Alice", age=30}

# Basic Access
dict@name
dict.name
# dict["name"]     # → "Alice"
# dict.name        # → "Alice" (shorthand)

# Dynamic Key Support
# let key = "ag" + "e"
# dict[$key]       # → 30

# Nested Access
# let data = {user: {profile: {id: 100}}}
# data.user.profile.id # → 100
```

**Edge Cases**:
| Scenario                      | Behavior                       |
|-------------------------------|--------------------------------|
| Accessing a non-existent array index | Returns `None`                |

---

### 3. Chained Operations
```bash
let x = 5
x += 3 * (2 ** 4)  # x = 5 + 48 = 53
```

### 4. Dynamic Indexing
```bash
["a", "b", "c"]@((1 + 1) % 3)  # → "c"
```

## IV. Statements

1. **Statement Blocks**
   Represented with `{}`, can isolate variable scope.

2. **Statement Groups**
   Represented with parentheses for subcommands; subcommands do not create new processes and do not isolate variable scope.
   `echo (len [5,6])`

3. **Statements**
   Separated by `;` or `enter`.

   - **Newline**: `;` or enter.

   - **Continuation**: Use `\` + newline to write across lines.
   ```bash
   let long_str = "Hello \
                   World"  # Equivalent to "Hello World"
   ```

---

## V. Control Structures

1. **Conditional Statements**
   - **If Condition**
   Supports nesting:
   `if cond1 { ... } else if cond2 { ... } else { ... }`

   Does not use the `then` keyword; code blocks are wrapped in `{}`.
      ```bash
      if True 1 else if False 2 else 3

      if x > 10 {
          print("Large")
      } else if x == 10 {
          print("Equal")
      } else {
          print("Small")
      }
      ```

   - **Match Statement**
   Replaces the switch statement in bash.
      ```bash
      let x = "a"
      match x {
      "b" => echo "is letter",
      _ => echo "is others"}
      ```

2. **Loops**
   - **For Loop**: Range uses `to` (left-inclusive, right-exclusive).
     ```bash
     for i in 0 to 5 {    # Outputs 0,1,2,3,4
         print(i)
     }

     for i in [1,5,8] { echo i }
     ```
   - **While Loop**:
     ```bash
     let count = 0
     while count < 3 {
         print(count)
         count = count + 1
     }
     ```

   **Edge Cases**:
   - The end value of `to` is not included in the iteration range.

---

## VI. Functions

1. **Function Definition**
   - Defined using `fn`, supports default parameters, and supports `return`.
   ```bash
   fn add(a, b, c=10) {
       return a + b + c
   }
   echo add(2, 3)  # Outputs 5
   ```
2. **Lambda Expressions**
   - Defined using `->`. Does not support default parameters or return statements.
   Lambda will inherit the current environment and run in an isolated environment, not polluting the current environment.

   ```bash
   let add = x -> y -> x + y
   ```

3. **Macros**, anonymous macros (valid in the current scope):
   Macros run in the current environment without environment isolation.

   ```bash
   x ~> x + 1
   ```

4. **Function Calls**
   - Call functions with parameters listed.
   ```bash
   add 3 5
   add(3,5)
   ```

   **Edge Cases**:
   - When function names conflict, the new definition will override the old module.
   - Calling a function with mismatched parameter counts will throw an error.

---

## VII. Parameters and Environment Variables
1. **Command Line Parameters**:
   - Script parameters accessed via the `argv` list.
   ```bash
   # Run lumesh script.lsh Alice tom
   echo argv  # Outputs "[Alice, tom]"
   echo argv@0  # Outputs "Alice"
   ```
2. **Environment Variables**
   ```bash
   PATH             # System environment variable
   HOME             # System environment variable

   env              # Lists all current environment variables
   IS_LOGIN         # Is it a LOGIN-SHELL
   IS_INTERACTIVE   # Is it interactive mode
   IS_STRICT        # Is it strict mode
   ```

## VIII. Execution Modes

1. **REPL Interactive Mode**
   User interactive mode, handling user input and output, with syntax highlighting.
   **Shortcuts and Commands**:
   - `Right`: Autocomplete (supports paths and history commands).
   - `Ctrl+C`: Terminate the current operation.
   - Special commands: `cd`, `exit`, `clear`.

   **History**: Saved in `~/.lumesh-history`.

2. **Script Parsing Mode**
   Run scripts: ```lume ./my.lsh```

3. **LOGIN-SHELL Mode**
   Start shell, should configure system environment variables. Similar to `.bashrc` content, in a config file.

4. **Strict Mode**
   In strict mode, variables must be declared and cannot be redeclared.

   In non-strict mode, spaces around operators are allowed. Implicit type conversion is permitted.

---

## IX. Built-in Functions

View using the `help builtin` command.

> Console Operations
+ echo
+ print
+ println

- input
- cd
- prompt
- incomplete_prompt
`let x = input "your choice:"`

+ help
+ exit
+ quit

> Operations on Expressions
+ `unbind` to cancel a variable
+ `eval` to execute the function represented by the literal of the expression
+ `str` to get the literal of the expression
+ `report` same as `str`

> Operations on Lists
- `len` returns the length of a list/dictionary/string
- `chars` converts a string to a list of single characters
- `lines` splits a string into a list by lines
- `insert` inserts into a list
`insert [1,2,4] 2 3`

• The following built-in functions have been tested and found ineffective:
- `neg`
- `add`
- `sub`
- `div`
- `mul`
- `rem`
- `range`
- `remove`
- `index`
- `and`
- `or`
- `not`
- `eq`
- `neq`
- `lt`
- `gt`
- `lte`
- `gte`

> Help files not listed
- pwd
- clear
- join to convert a list to a string

## Built-in Libraries

View using the `help lib` command.

When using, reference with `lib_name@fn_name`.

For detailed content, please continue reading: [Built-in Libraries](lib.md)

Through this manual, users can master the core syntax and edge cases of Lumesh. It is recommended to practice with REPL to deepen understanding.
