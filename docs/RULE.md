# Rules for Lumesh Param Completion File

- format: csv
- columns: cmd,conds,short,long,argument,dirs,pri,[desc|exec]
- delimiter: `,`
- column type:
  - cmd: symbol.
  - condtions: symbols joined with whitespace.
  - short: symbol.
  - long: symbol.
  - argument: symbols joined with whitespace.
  - dirs: directives joined with whitespace.
  - priority: number.
  - description: string.
  - exec (optional): command template for dynamic completion. only read when `@E` directive is present.
    `{}` is replaced by the current token at runtime.
- conditions:
  if one or more symbol, means require the cmd line contains any one of listed conditions as subcmd (OR logic).
  if empty, means the cmd should have no subcmd.
  if there's a `@i` in directives, reverse the result.
  - `+` in a condition token means AND logic: all joined parts must match.
    eg. `stash+push` matches only when both `stash` and `push` are present.
  - `!` prefix on a condition (or sub-condition) means NOT: the condition must NOT match.
    eg. `!help` matches only when `help` is absent.
    eg. `stash+!push` matches when `stash` is present and `push` is absent.
- directives:
  - `@i`: means INVERT, this will reverse the result of condition match.
  - `@t`: means TRUE, this will always return true while checking conditions.[^1]
    eg.

    > if you mean no subcmd, you should have ` ` in conditions.
    > if you mean any subcmd, you should have ` ` in conditions and `@i` in directives.
    > if you mean any special subcmd, you should have them listed in conditions.
    > if you mean not special subcmd, you should have them listed in conditions and `@i` in directives.
    > if you mean any subcmd or no subcmd, you should have `@t` in directives.

  - `@E` Execute lume script to get completion candidates. The 8th column `exec` must be set. no description for this item.
    variable `$T` was injected for current token.
    Example: `"let dir = fs.dir_name $T; $dir | adb shell ls _ |> $dir + '/' + _"` → when completing `adb pull /sdcard/`, the completion executor will runs this script and uses output lines as completion items.
  - `@m` multi, allow options to be used multi-times.
  - `@D` Directory complete[^1]

  > following means as same as fish complete
  - `@F` File complete
  - `@f` no file
  - `@r` required
  - `@x` no file and required[^1]
  - `@k` keep[^1]

[^1] not supported now. maybe later.
