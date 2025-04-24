# Changelog
## [0.3.0]
- rewrite parser
- rewrite expr excutor
- rewrite interactive repl
- rewrite pipe and redirect

### break changes
- lambda comes to : `(x,y) -> { x+y }`
- ranges comes to : `3..9`


### other changes
- allow `.` to be index of maps.
```bash
fmt.red logo        # same as fmt@red
```
- allow `[]` to be index of list, and slice supported.
```bash
let ar = 2..8
ar[2:5:2]           # [4,6]
ar[2:5]           # [4,5,6]
ar[2:]           # [4,5,6,7]
let i = -1
ar[i]            # [7]
```
- cmd completition supported.
- error msg becomes more clear.
- math power support.
- pipe, redirect works more effient
- pipe support 2 modes: stream pipe `|` and param pipe `|>`
pipes to interactive programs like vi, less fixed.
- lambda and macros support particle apply

- cmds never neeeds None param now
- allow custom operators:
```bash
# all custom operators starts with _
# unary operators starts with __
let __+ = x -> x + 1
__+ 5               # 6

# binary operators with prec level same as + and -
# starts with _+
let _+ = (x,y) -> x + y
2 _+ 3 * 5          # 17

# binary operators with prec level same as * and /
# starts with _*
let _* = (x,y) -> x + y
2 _* 3 + 5          # 11

```

## [0.2.3] 2025-4-9
- fix test usecase.
- fix `-` in command string.
  as there're lots of cmds has `-`, eg :
  `connman-gtk`

  > in strict mode:
  `a + 3` is pretty.

  > in non-strict mode:
  + `-` or `/` :
    when operate with vars, space around was recomended.
    for differ with strings. eg:
    `a-a`, `a-3` was *string*;

    `3-a`, `8-3` was *plus*;

  + other operaters:
    `a*a` `a+3` `3-2` and all other operator will be recognized as math operate.


- move default cmd after config. only load if no same config.

- change log to logs to avoid conflict with `git log`

- refactor config init.

  now config files moved to `dirs.config`.

  in linux, it's `~/.config/lumesh/config.lsh`

- system env added. unifiy status env.

`PATH`,`HOME` and other enviroment vars are visible in interactive shell.

`IS_LOGIN` implies wheather it was started as LOGIN SHELL.

`IS_INTERACTIVE` implies wheather it was interactive.

`STRICT` implies wheather it was running in STRICT mode.

## [0.2.2] 2025-4-8
-  env vars:

use `SCRIPT` var to get the script file path.

use `STRICT` var to know wheather in strict mode.

use `argv` to receive vars parsed in.

-  apply strict mode.

`lume -s` to start in strict mode.

in strict mode, var redeclare will be rejected and warned.

also, operator without space was not allowed.(act same as dune)

in nonstrict mode, they are all allowed.

-  fix let a =6; a=9 report redclaration.

-  add more fs@dirs.

such as `(fs@dirs)@config` is config dir.

also `cache` for cache dir. `data` for data dir.

-  change all builtin `-` to `_` and remove ? after func name.simplify cursor move name.

this is because in nonstrict mode, `-` was recognized as decrease.

so in vars symbol, `_` is better than `-`.

-  help as feather; fix list@head/tail to first/last; drop curry for take/drop.

this is done to reduce the size of binary.

help docs will be published alone.

-  update os-info, chrono. reorganize and add functions for time_model.

now

`time.minute`,`time.stamp`, `time.display`, `time.fmt` ....

was avaluable.

-  remove clap from runner.

-  split repl and runner

a pure edition was added to parse script only.

more efficient.

-  fix env influent in list, if, while, match.

now vars declared in these blocks, was only visible in.

no influent to outer enviroment.

-  add fn define and return statement.

```bash
fn myfunction(x, y=2) {
  let z = 0
  return x + y + z
}```

-  refactor parse_expression_prec_op.
- adjust parse_range and parse_not before parse_expression_prec_five.

## [0.2.1] 2025-4-5

-  args to argv to compite with fish.
-  Merge branch 'feather/linesplit' into fix/symbol
-  move os help func from bin to os_module.
-  update clap 4.5.35; add interactive back.
-  update clap to 4.4

## [0.2.0-lineclip] - merged from feather/linesplit 2025-3-29
- use `\n` or `;` to split statement. which means you don't have to type `;` to every lineend.
- use `\\n` to continue a line.

## [0.2.0-symbol3]
- update clap
- add `argv` env to receive args.
`lumesh myscript.dn -- arg1 arg2 ...`
`echo argv;
 echo argv@0;
 echo (len argv)
`
- add conditional statement
`let a = b ? 1: 0`

- add match statement
`match len a{
 1 => echo "only one"
 "never" => echo "never"
 _ => echo "default brache" }
 `
 you can split your braches via `,` or `;` or `\n`.

 - add whiel statement
 `while a>0 {
 echo a
 a = a + 1
 }`

## [0.2.0-symbol2]
- regix module
used regix_lite

- quick match operators;

`~=` to test match regex
`~~` to test string contains.

- while statement.
`while (x<10) {print x;}`

- add var declare and assign
now let was used to declare a var, with or with an initial value:
`let a`
`let a = 1`

also multi declare is supported:
`let a,b,c`
`let a,b,c = 1,2,3`
`let a,b,c = 10`

assign value never need `let` now:
`let a ;
 a = 2;
 a = 3`

- lasy sign (previously Quote) now changes to `:=`
`let e := ls` instead of ` let e = 'ls`

- del var
`del a`
this is useful while a stores big data, such as file content.

- `''`was used to raw string without escape.
type `'a\tb\nc'` and `"a\tb\nc"` to see the difference.

- change div from `//` to `/`
- custom operators begin with `_`


## [0.2.0-symbol]


this branch fix symbols:

- allow nonspace operators.
such as `let a=2+3;` `a>3` `let add=x->x+1`

but space is needed when you need to differ negtive numbers with operator:
such as `let a=2+ -3`

- allow args in command.
such as `ls -l --color=auto /tmp`
  + short args: `-c`
  + long args: `--chars`
  + paths: `./dir` or `/dir` or `..`

  but unfortunlately, single `/` is not added currently, as this may be used as operator someday.

  single `.` was ignored and default to cwd.

- allow `:` to define dict.
`let dict={x:1,y:2}` as well as the old one :
`let dict={x=1,y=2}`


### [0.2.0-lineclip] - 2025-3-29
- use `\n` or `;` to split statement. which means you don't have to type `;` to every lineend.
- use `\\n` to continue a line.


## [Unreleased]

*No unreleased changes yet*

## [0.1.8] - 2022-01-02

### Added
- [#64](https://github.com/adam-mcdaniel/lumesh/pull/64): Add Changelog
- [#65](https://github.com/adam-mcdaniel/lumesh/pull/65): Added keys and vals functions
- [#76](https://github.com/adam-mcdaniel/lumesh/pull/76): Added command line argument parser

### Changed
- [#66](https://github.com/adam-mcdaniel/lumesh/pull/66): Report error when `cd` fails
- [#75](https://github.com/adam-mcdaniel/lumesh/pull/75): CWD Init

### Fixed
- [#76](https://github.com/adam-mcdaniel/lumesh/pull/76): Fixed `fmt@white`

## [0.1.7] - 2021-10-18

### Added
- [#59](https://github.com/adam-mcdaniel/lumesh/pull/59): Add recursion depth limit
- [#61](https://github.com/adam-mcdaniel/lumesh/pull/61):
    * Add builtin `parse` module for parsing JSON, TOML and lumesh
    * Add `width` and `height` methods to console module
- [#65](https://github.com/adam-mcdaniel/lumesh/pull/65): Add `keys` and `vals` functions
- [#67](https://github.com/adam-mcdaniel/lumesh/pull/67): Add GitHub workflow to create releases with pre-built binaries

### Changed
- [#45](https://github.com/adam-mcdaniel/lumesh/pull/45), [#51](https://github.com/adam-mcdaniel/lumesh/pull/51): Improve parser error messages and parsing performance
- [#54](https://github.com/adam-mcdaniel/lumesh/pull/54): Improve syntax highlighting by recovering from tokenizing errors
- [#61](https://github.com/adam-mcdaniel/lumesh/pull/61):
    * Change `eval` to never modify the current scope
    * Add `exec` for `eval`'s old behavior
    * A script must now be parsed with `parse@expr` before evaluating it
    * `console@write` now accepts values other than strings
- [#63](https://github.com/adam-mcdaniel/lumesh/pull/63): Allow builtin operators to be used like symbols; the operators are now used directly for operator overloading

### Fixed
- [#56](https://github.com/adam-mcdaniel/lumesh/pull/56): Fix widgets not working correctly on Windows
- [#57](https://github.com/adam-mcdaniel/lumesh/pull/57): Fix history permissions error
- [#60](https://github.com/adam-mcdaniel/lumesh/pull/60): Fix incorrect line number 0 in syntax errors
- [#63](https://github.com/adam-mcdaniel/lumesh/pull/63): Fix parsing of `!` (logical *not*) operator
- [#66](https://github.com/adam-mcdaniel/lumesh/pull/66): Report error when `cd` command fails

---------

*No changelog available for older releases*

## [0.1.6] - 2019-10-09
## [0.1.5] - 2019-10-05
## [0.1.4] - 2019-10-02
## [0.1.3] - 2021-09-27
## [0.1.2] - 2021-09-27
## [0.1.1] - 2021-09-27
## [0.1.0] - 2019-09-09

[Unreleased]: https://github.com/adam-mcdaniel/lumesh/compare/v0.1.8...HEAD
[0.1.8]: https://crates.io/crates/lumesh/0.1.8
[0.1.7]: https://crates.io/crates/lumesh/0.1.7
[0.1.6]: https://crates.io/crates/lumesh/0.1.6
[0.1.5]: https://crates.io/crates/lumesh/0.1.5
[0.1.4]: https://crates.io/crates/lumesh/0.1.4
[0.1.3]: https://crates.io/crates/lumesh/0.1.3
[0.1.2]: https://crates.io/crates/lumesh/0.1.2
[0.1.1]: https://crates.io/crates/lumesh/0.1.1
[0.1.0]: https://crates.io/crates/lumesh/0.1.0
