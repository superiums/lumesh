# Rules for Lumesh Param Completion File 

- format: csv
- columns: cmd,conds,short,long,params,dirs,pri,desc
- delimiter: `,`
- column type:
  + cmd: symbol.
  + condtions: symbols joined with whitespace.
  + short: symbol.
  + long: symbol.
  + params: symbols joined with whitespace.
  + dirs: directives joined with whitespace.
  + priority: number.
  + description: string.
- conditions:
  if one or more symbol, means require the cmd line contains any one of listed conditions as subcmd.
  if empty, means the cmd should have no subcmd.
  if there's a `@n` in directives, reverse the result.
- directives:
  + `@n`: means NOT, this will reverse the result of condition match.
  + `@t`: means TRUE, this will always return true while checking conditions.[^1]
  eg.
  > if you mean no subcmd, you should have ` ` in conditions.
  > if you mean any subcmd, you should have ` ` in conditions and `@n` in directives.
  > if you mean any special subcmd, you should have them listed in conditions.
  > if you mean not special subcmd, you should have them listed in conditions and `@n` in directives.
  > if you mean any subcmd or no subcmd, you should have `@t` in directives.

  + `@m` multi, allow options to be used multi-times.
  + `@D` Directory complete[^1]

  > following means as same as fish complete
  + `@F` File complete
  + `@f` no file
  + `@r` required
  + `@x` no file and required[^1]
  + `@k` keep[^1]

[^1] not supported now. maybe later.
  
