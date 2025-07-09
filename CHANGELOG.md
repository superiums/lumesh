# Changelog
## [0.6.8]
- add boolean module;
- add concat to string module;
- optimize capture_name for Regex;
- add `gt/ge/lt/le/eq/ne` to math;

## [0.6.7]
- eval args for builtin in assign mode
so user could call `String.red((ls))` and got the expected result.
- add `grep` for String module.
- add `highlight` for Parse module.
- pull down echo for function declare.
- allow var after `!`/`=` like `!$x`, `=$x`
- add `exit` to repl.
while `exit [status]` used for script, `quit` used for repl.
- expand home during `for`,`use` and in `Fs` functions.

## [0.6.6]
- remove the following command warp, keep it as it was.

> 3. ` (a)` single symbo in *group*
> 4. `alias x = a` single symbo in *alias declaration*
> 5. `let x := a` single symbo in *lasy declaration*

- make use of IN_ASSIGN:
  - check IN_ASSIGN in Group, and convert single symbo to Command.
  - check IN_ASSIGN mode in cmd, set it to pipe mode if true.
  - set IN_ASSIGN mode for args.

- allow symbol as alias cmd
- optimize display and debug of expression
- link some func from Math to List
- link some fn from into/parse to String.
  such as:
    to_int
    to_float
    to_filesize
    to_time
    to_table
- rename some fn in List
- link get to List/Map.
- add at to Map.
- link pprint to List/Map/String
pprint in String accecpt headers define.
- detailed contains err
- fix map like `{a,}` and `{a:b}`
- detaied err while no module;
- reduce unkown module seek;
- make range able to chain call most List funcs.
- link to_str to Math
so user could call `3 | .to_str()`

## [0.6.5]
- move os to sys;
- move fmt to string;
- move widget to ui;
- fix chaincall arg eval;
- fix % in parse_cmd;
- allow linebreak in List;
- update readme.
- link len/insert/rev/flattern to list/map.
- optimize ctrl+j to never include param hint.
- add more fn for list module, and optimize some existing fn.
- add find/filter to map module.

## [0.6.4]
- allow set modules path with `LUME_MODULES_PATH`
- pass only root env to cmd.
- allow set strict mode in script.
```bash
STRICT=True
```
- strict fn redeclare;
- strict param covers in deco.
- add support to config max recursion
via `LUME_MAX_SYNTAX_RECURSION` and `LUME_MAX_SYNTAX_RECURSION`

- add `^` to force a symbo to be a third part command
`sort^` call cmd `sort` in your system insteadof lumesh function.

- stop wrap command as symbo after assign
`x = a b; let x= a b` now keeps as command.

- stop wrap symbo as command after lazy assign
`x := a` now keeps as symbo.


now, the wraps action works as follow:
1. `a` single symbo was warped as command.
2. `./a` single path was warped as command.
3. ` (a)` single symbo in *group* was warped as command.
4. `alias x = a` single symbo in *alias declaration* was warped as commnd.
5. `let x := a` single symbo in *lasy declaration* was warped as commnd.
note this not include the lasy assign: `x := a`, which keeps as symbo.
  and `eval x` or `x` or `x other_args` could launch it.



## [0.6.3]
- add decorator support
- add `LUME_IFS_MODE` to control where to apply `IFS`

with bit meanings:
```rust
IFS_CMD: u8 = 1 << 1; // cmd str_arg
IFS_FOR: u8 = 1 << 2; // for i in str; str |> do
IFS_STR: u8 = 1 << 3; // string.split
IFS_CSV: u8 = 1 << 4; // parse.to_csv
IFS_PCK: u8 = 1 << 5; // ui.pick
```

- allow `{k,}` as Map


## [0.6.2]
**broken changes**
- rename builtin modules with Uppercase

normal changes

- add dispatch pipe
```bash
    let add = (x,y) -> x+y
    0...5 |> add(_,10)

    # outpu: [10, 11, 12, 13, 14, 15]
```
which means every item of the left list is dispatched to right, and runs right handside 6 times.
this action is simlar as `xargs`

- remove `report`


## [0.6.1]
- optimize error report for pratt parser
- optimize depth for pratt parser.
- allow `|_` to chaincall. replace arg of first call with `_`
- allow chaincall as alias.

## [0.6.0]
**broken changes**
- change `?!` to `?>` : return err map as result.
- add `?!` as quiet terminate when err occurs, for pipes.
- change `>>!` to `>!` : overwrite to file.

normal changes

- add destructure assign support
- add error popup function: `sys.error`
- disable string args split
- fix err deel. optimize output
- fix err fmt, optimize display
- detailed io error
- allow bare func in pipe without apply tag
- fix unkonwn operator error report

## [0.5.9]
- add chain call support
```bash
"a b c".split(' ').join(',')
```

- add chain pipe method support
```bash
"a b c" | .split(' ')
```
- dim syntax tree debug

## [0.5.8]
- add |_ as position pipe
- add |^ as pty pipe
- pretty print with tabled
- fmt debug tree
- allow linebreak in arguments of fn call
- optimize with Cow

## [0.5.7]
- more friendly runtime errors
now display the expression context and ast, depth for runtime errors.


## [0.5.6]
- tip early line end; remove IFS from parse.cmd
- rewrite match to support regex.
- optimize error showing.

## [0.5.5]
- optimize cmd execute
- support IFS in
  cmd args/for/str.spit/parse.cmd/csv/to_csv/ui.pick/mpick/

- fix ( { aa first arg
- support multi match in one branch
- add !~: and !~~
- allow !$x
- remove none result from for
- fix str arg parse without quote

## [0.5.4]
- more ui component
- fix highlight frequence
- fix IN_PIPE covers last.
- ansi code; pwd;
- reorganize parse_string* ; more ansi seq.
- add discard/base_name/join.
- print_tty; rename err_codes/stamp_ms;
- rename to fs.is_dir/is_file
- alone env for root.
- root env only clone std:vars. set /unset to top.
- set/unset root env. fix var set in child env.
- more keyword alone.
- fix $var in after`,:
- fix blank $var as cmd; fix $var in ().
- fix $arg in cmd
- $var as cmd, list args flatten, fix template execution pipe capture.
- template parse fix;render exec; pretty help;
- filesize to boolean

## [0.5.3]
- add fuzzy picker
- add pretty print

## [0.5.2]
- fix where condition executed too early
- fix capture output of comand as args for builtin
`fmt.red(lspci -k)`

- support args with cmd mode without file
this is for compatible with `lf` file manager.

- add `LINENO` and `LINES` for where/list.filter
`fs.ls -l | where(LINENO>1)`

- support string template render
`a is $a or ${a}`

## [0.5.1]
- enhanced get for mixed list/range and map path.
- add range step.
`0..8:2`
- change range: closure as `..` and right open with `..<`

## [0.5.0]
- fix max for only int
- fix pipeout capture for builtin funcs.
## [0.4.9]
- add func arg hint.
- fix some func param position.
- reorgnize console mode/cursor/keyboard.
- fix map display
- prompt with func.
- prompt from func.
- prompt template
- use lm extention
## [0.4.8]
- part choose for path.
- only hint in cmd place.
- alias hint
- full module hint. accecpt one world of hint.
- complete and hint with builtin/cmds
- into module
- hint gray
- add filesize convert.

## [0.4.7]
- add csv
- add jq
- add to_csv/to_json/to_toml
- fix err msg for parsing module.
- prompt settings.

## [0.4.6]
- refactor sort function
- optimize filesize, time display. optimize list.group.
- add FileSize and cmp. add % as float.
- fix 1M in line end.
- fix non-ascii path broken
- add ..., suport justify range start via +/-; fix fs.mv/cp
- fix history.
- add vi mode.


## [0.4.5]
- fix stdin read for third cmd
- optimize io
- fix combo keys in pty


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
default config in `$config_dir/lumesh/config.lm`
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

  in linux, it's `~/.config/lumesh/config.lm`

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
