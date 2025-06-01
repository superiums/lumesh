# Changelog
## [0.4.4]
- fix a lot in libs

## [0.4.3]
- update libs to newest.
except nom.
- enhance string loop
support loop in string result now, we cut it to array via newline, witespace, `,`,`;`
- fix pty for fish/vi
- fix hotkey

## [0.4.2]
- introduce pty

## [0.4.1]
- optimize expr execution
- fix break in while

## [0.4.0]
- refactor pipes and redirect
- introduce bmap back
- enhance list and map display
- support background cmd
```bash
thunar &         # run in background, and shutdown stdout and stderr
ls &-            # shutdown stdout
ls /o &?         # shtudown stderr
ls &.            # shutdown stdout and stderr
ls &+            # try to append stderr to stdout

```
- support output shutdown
- enhance contains operator : `~:`
now support : String/List/Map keys containning check.
- support datetime expression
- fix some issue


## [0.3.9]
- add keybindings
- fix parser for bare utf8 chars
- redefine `>>` and `>>!`, `~=`,`~~`,`~:`
- fix path for windows
- enhance list and map display
- use btreemap in expr, but hashmap for performanc
- enhanced libs, `fs.ls`, `parse.cmd`,`where`,`select`...


## [0.3.8]
- add loop and break
- change `?-` to `?+`, which means to merge errs to stdout.
`6 / 0 ?-    # print err to stdout`
- fix path for windows
- add some config

## [0.3.7]
- refactor expression and builtin, pipes with ref and reduce clone of expr

after rebuild, it becomes 3 times faster than previous!
Great!

## [0.3.6]
- refactor expression with Rc.
this makes expr clone runs 4 times faster than Box.
- enhanced lib functions.

## [0.3.5]
- enhance operator overload.
- allow $var, allow blank line in block.
- split alias from env, enhance cmd executor. fix fmt arg check. add parse recursdepth err.
- remove STRICT from operator_tag.
- remove macro.
- optimize index; adjust operator tag.
- filter env map to cmd
- change expr and env to hashmap; lambda without env capture. tick out env from math/fs module.
- allow symbol as string in dict.
- fix symbol catch, fix for result. doc renew.
- optimize pram collector. fix literal type in pipe and error catch.
- support args collector

## [0.3.4]
- enhance pipe to be more clever
auto pass value or stdin;
auto suite pipeout to stdout or param out.

- error catch could be used in pipes
- fix groups in pipes
- print returns value for pipe use


## [0.3.3]
- add local ai auto complete.
you can config your local ai assistant like this:
```bash
let LUME_AI_CONFIG = {
    host: "localhost:11434",
    complete_url: "/completion",
    chat_url: "/v1/chat/completions",
    complete_max_tokens: 10,
    chat_max_tokens: 100,
    model: "",
    system_prompt: "you're a lumesh shell helper",
}
```
- add alias
- custom profile, no history mode.
- read config
default config in `$config_dir/lumesh/config.lsh`
or specify your own via `lume -c your/path`
- allow mutline map, and tailing with `,`
- parse cmd after = as string/symbol.
subcmd capture should use `()` or ````
- allow empty params def in lambda.
- allow function in circle. fix env for function.
- fix custom op define.

## [0.3.2]
- add support for error handling.
handling errs never have been so easy:
```bash

6 / 0 ?.    # ignore err
6 / 0 ?-    # print err to stdout
6 / 0 ??    # print err to stderr
6 / 0 ?!    # use this err msg as result

let e = x -> echo x.code
6 /0 ?: e   # deeling the err with a function/lambda

# also funcions could use err handling too.
fn divide(x,y){
    x / y
}?: e

```
- fix chained conditional expr.
chained exprs like `a?b:c?d:e` works well now.

## [0.3.1]
- rewrite parser again
pratt parser workflow.
differ inner functions from system cmds.
- optimize builtin storage in hashmap.
- fixed other lots.

```bash
# system cmds:
ls -l

# func call:
fmt.red("hello", 3 + 5)
# or as flat as cmds:
fmt.red! "hello" 3 + 5

# math expression directly:
3 + 5

```

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
