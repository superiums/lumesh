## Functions for LIBS

USEAGE:
`<module-name>.<func-name>(arg1,arg2)`
`about.bin()`
`string.red('x')`
`string.red x`
NOTE:

- if arg is lambda expression, use square call method only.
- NEVER use lib name as var name.

### Top level Functions

usage:
`cd /opt`
`cd(/opt)`

assert <condition> [message]
cd [path]
cwd
  current working directory

ddebug <args>...
  pretty debug
debug <args>...
eprint <args>...
eprintln <args>...
eval <expr>
  evaluate expression in current env
eval_str <expr>
  evaluate string in current env
exec <expr>
  execute expression in new env
exec_str <string>
  execute string in new env
exit [status]
flatten <collection>
format <template> <args>...
  formatted string with vars
get <map|list|range> <path>
  get value from nested map/list/range using dot notation path
help [module]
import <path>
  evaluate file in new env
include <path>
  evaluate file in current env
len <collection>
not <boolean1>...
pprint <list>|<map>
  pretty print
print <args>...
println <args>...
read [prompt]
  get user input
repeat <expr> <n>
rev <string|list|bytes>
select <table> <columns...>
  select columns from list of maps
set_root <var> <val>
  define a variable in root environment
sortby <table> <col>
symof <value>
  get type of data symbol
tap <args>...
  print and return result
throw <msg>
  return a runtime error
typeof <value>
  get type of data value
unset_root <var>
when <condition> <execute>
where <table> <condition>
  filter rows by condition

### about

bin
info
prelude
version

### boolean

and <boolean1>...
not <boolean1>...
or <boolean1>...

### console

clear
cursor_down <n>
cursor_hide
cursor_left <n>
cursor_restore
cursor_right <n>
cursor_save
cursor_show
cursor_to <x> <y>
cursor_up <n>
flush
height
keys
mode_normal
mode_raw
read_key
read_line [prompt]
read_password [prompt]
screen_alternate
screen_normal
title <string>
width
write <text> <x> <y>

### filesize

b <filesize>
  get bytes of a filesize
from <size_str|byte_int>
gb <filesize>
kb <filesize>
mb <filesize>
tb <filesize>
to_string <filesize>

### from

cmd <cmd_output_string> [headers|header...]
csv <csv_string>
jq <query_string> <json_data>
json <json_string>
script <script_string>
toml <toml_string>

### fs

abs <path>
append <content> <file>
base_name <path> [split_ext?]
canon <path>
cp <source> <destination>
dir_name <path>
dirs
exists <path>
glob <pattern>
head <file> [n]
is_dir <path>
is_file <path>
join <path>...
ls [-l|a|h|t| L|c|u|m|p] [path]
mkdir <path>
mv <source> <destination>
parent <path>
read <file>
rm <path>
rmdir <path>
tail <file> [n]
tree [path]
write [content] <file>

### hmap

at <map> <key>
difference <map1> <map2>
filter <map> <predicate_fn>
find <map> <predicate_fn>
flatten <map>
from_items <items>
get <map|list|range> <path>
has <map> <key>
insert <map> <key> <value>
intersect <map1> <map2>
items <map>
keys <map>
len <map>
map <map> <key_fn> <val_fn>
merge <map1> <map2> [<map3> ...]
remove <map> <key>
set <map> <key> <value>
to_bmap <map>
union <map1> <map2>
values <map>

### into

boolean <value>
csv <expr>
filesize <size_str>
float <value>
highlighted <script_string>
int <value>
json <expr>
str <value>
striped <string>
table <command_output> [regex|headers...]
time <datetime_str> [datetime_template]
toml <expr>

### list

all <list> <fn>
any <list> <fn>
append <list> <element>
at <list> <index>
average <num1> <num2> ... | <array>
chunk <list> <size>
concat <list1|item1> <list2|item2> ...
contains <list> <item>
drop <list> <count>
filter <list> <fn>
filter_map <list> <fn>
find <list> <item|fn> [skip_n]
find_last <list> <item|fn> [skip_n]
first <list>
flatten <collection>
foldl <list> <fn> <init>
foldr <list> <fn> <init>
from <range|item...>
get <map|list|range> <path>
  get value via dot sperated path
group <list> <key_fn|key>
insert <list> <index> <value>
is_empty <list>
items <list>
join <list> <separator>
last <list>
len <list>
map <list> <fn>
max <num1> <num2> ... | <array>
min <num1> <num2> ... | <array>
prepend <list> <element>
remove <list> <item> [all?]
remove_at <list> <index> [count]
rev <list>
set <list> <index> <value>
sort <string|list> [key_fn|key_list|keys...]
split_at <list> <index>
sum <num1> <num2> ... | <array>
take <list> <count>
to_hmap <list> [key_fn] [val_fn]
to_map <list> [key_fn] [val_fn]
to_set <list>
transpose <matrix>
unique <list>
unzip <list_of_pairs>
zip <list1> <list2>

### log

debug <message>
disable
echo <message>
enabled <level>
error <message>
get_level
info <message>
set_level <level>
trace <message>
warn <message>

### map

at <map> <key>
difference <map1> <map2>
filter <map> <predicate_fn>
find <map> <predicate_fn>
flatten <map>
from_items <items>
get <map|list|range> <path>
has <map> <key>
insert <map> <key> <value>
intersect <map1> <map2>
items <map>
keys <map>
len <map>
map <map> <key_fn> <val_fn>
merge <map1> <map2> [<map3> ...]
remove <map> <key>
set <map> <key> <value>
to_hmap <map>
union <map1> <map2>
values <map>

### math

abs <number>
acos <value>
acosh <value>
asin <value>
asinh <value>
atan <value>
atanh <value>
average <num1> <num2> ... | <array>
bit_and <int1> <int2>
bit_not <integer>
bit_or <int1> <int2>
bit_shl <integer> <shift_bits>
bit_shr <integer> <shift_bits>
bit_xor <int1> <int2>
cbrt <number>
ceil <number>
clamp <value> <min> <max>
cos <radians>
cosh <value>
cospi <value>
eq <number_base> <number>
exp <exponent>
exp2 <exponent>
floor <number>
ge <number_base> <number>
gt <number_base> <number>
is_odd <integer>
le <number_base> <number>
ln <number>
log <base> <number>
log10 <number>
log2 <number>
lt <number_base> <number>
max <num1> <num2> ... | <array>
min <num1> <num2> ... | <array>
ne <number_base> <number>
pow <exponent> <base>
round <number>
sin <radians>
sinh <value>
sinpi <value>
sqrt <number>
sum <num1> <num2> ... | <array>
tan <radians>
tanh <value>
tanpi <value>
to_str <number>
trunc <number>

### rand

alpha [length]
alphanum [length]
choose <list>
int [min] [max]
ratio <probability>
shuffle <list>

### regex

capture <pattern> <text>
  get first capture groups as [full,group1,...]
capture_name <pattern> <text>
  get capture groups with names
captures <pattern> <text>
find <pattern> <text>
find_all <pattern> <text>
match <pattern> <text>
replace <text> <pattern> <replacement>
split <pattern> <text>

### set

add <set> <item>
contains <set> <item>
difference <set1> <set2>
filter <set> <predicate_fn>
find <set> <predicate_fn>
first <set>
from_items <items>
intersect <set1> <set2>
is_empty <set>
is_subset <set1> <set2>
is_superset <set1> <set2>
items <set>
last <set>
len <set>
map <set> <fn>
remove <set> <item>
to_list <set>
union <set1> <set2>

### string

black <string>
blink <string>
blue <string>
bold <string>
caesar <string> <shift>
center <string> <length> [pad_char]
chars <string>
clr <string> <color_spec>
clr_bg <string> <color_spec>
color <string> <hex_color|color_name|r,g,b>
color_bg <string> <hex_color|color_name|r,g,b>
colors [skip_colorized?]
concat <string>...
contains <string> <substring>
cyan <string>
dim <string>
ends_with <string> <substring>
green <string>
grep <string> <substring>
href <url> <text>
insert <string> <index> <string>
invert <string>
is_alpha <string>
is_alphanumeric <string>
is_empty <string>
is_lower <string>
is_numeric <string>
is_title <string>
is_upper <string>
is_whitespace <string>
italic <string>
len <string>
lines <string>
magenta <string>
max_len <string>
pad_end <string> <length> [pad_char]
pad_start <string> <length> [pad_char]
paragraphs <string>
red <string>
remove_prefix <string> <prefix>
remove_suffix <string> <suffix>
repeat <string> <count>
replace <string> <old> <new>
split <string> [delimiter]
split_at <string> <index>
starts_with <string> <substring>
strike <string>
strip <string>
substring <string> <start> <end>
to_filesize <size_str>
to_float <value>
to_int <value>
to_lower <string>
to_table <command_output>
to_time <datetime_str> [datetime_template]
to_title <string>
to_upper <string>
trim <string>
trim_end <string>
trim_start <string>
underline <string>
white <string>
words <string>
words_quoted <string>
wrap <string> <width>
yellow <string>

### sys

cds
defined <var>
  test if var was defined in current env
discard <arg>
ecodes_lm
ecodes_rt
env [var]
  show root env vars
has <var>
  test if var was defined in current/parent env
info
max_runtime [int]
max_syntax [int]
max_usemode [int]
modes
print_tty <arg>
quote <expr>
set_cfm <boolean>
set_pdm <boolean>
set_strict <boolean>
vars
  list vars

### table

append <table> <list|set>
at <table> <index> <to_map?>
filter <list> <cell|fn>
find <list> <cell|fn> [start_index]
find_last <list> <cell|fn> [start_index]
first <table> <to_map?>
getcol <table> <header|index>
grep <table> <string>
header_len <table>
headers <table>
last <table> <to_map?>
len <table>
rows <table> <to_map?>
select <table> <cols...>
sortby <table> <col>

### time

add <datetime> <duration>
day [datetime]
diff <datetime1> <datetime2> <unit>
display [datetime]
fmt <format_string> [datetime]
from_map <map>
hour [datetime]
is_leap_year [year]
minute [datetime]
month [datetime]
now [format_string]
parse <datetime_string> [format_string]
second [datetime]
seconds [datetime]
sleep <duration>
stamp [datetime]
stamp_ms [datetime]
timezone <datetime> <offset_hours>
to_string <datetime> [format_string]
weekday [datetime]
year [datetime]

### ui

confirm <msg>
  show confirm dialog
date_pick [msg|cfg_map]
editor [msg|cfg_map]
  long text input
float <msg> [decimal_places]
  float input
int <msg>
  int input
join_flow <max_width> <widgets...>
joinx <widget1> <widget2>
joiny <widget1> <widget2>
multi_pick <list|items...> [msg|cfg_map]
  show multiple select dialog
passwd <msg> [confirm?]
pick <list|items...> [msg|cfg_map]
  show select dialog
text <msg> [initValue]
  text input box
widget <content> <title> [width] [height]

## CONSTS

### COLOR

#### 8bit color

| foreground | foreground light | background | background light |
| ---------- | ---------------- | ---------- | ---------------- |
| MAGENTA    | LIGHT_MAGENTA    | BG_MAGENTA | BG_LIGHT_MAGENTA |
| CYAN       | LIGHT_CYAN       | BG_CYAN    | BG_LIGHT_CYAN    |

...

usage：

```
COLOR.RED + 'lume' + COLOR.RESET
# same as
string.red('lume')
```

#### 256bit color

| foreground | background |
| ---------- | ---------- |
| FG_1       | BG_1       |
| FG_2       | BG_2       |
| ...        | ...        |
| FG_256     | BG_256     |

usage：

```
COLOR.FG_50 + 'lume' + COLOR.RESET
# same as
string.clr('lume',50)
```

#### true color

- by name

| foreground      | background      |
| --------------- | --------------- |
| aliceblue       | BG_aliceblue    |
| BG_antiquewhite | BG_antiquewhite |
| ...             | ...             |
| yellowgreen     | BG_yellowgreen  |

to list the avaluable colors, use：

```
string.colors(false)
```

usage：

```
COLOR.green + 'lume' + COLOR.RESET
# same as
string.color('lume','green')
```

- by hex code

| foreground | background |
| ---------- | ---------- |
| FGX_000000 | BGX_000000 |
| FGX_000001 | BGX_000001 |
| ...        | ...        |
| FGX_ffffff | BGX_ffffff |

usage：

```
COLOR.FGX_aaff22 + 'lume'
# same as
string.color('lume','#aaff22')
```

### MATH

MATH.E
MATH.PHI
MATH.PI

### STYLE

STYLE.BLINK
STYLE.BOLD
STYLE.DIM
STYLE.HIDDEN
STYLE.ITALIC
STYLE.NORMAL
STYLE.RESET
STYLE.RESET_BLINK
STYLE.RESET_BOLD
STYLE.RESET_DIM
STYLE.RESET_HIDDEN
STYLE.RESET_ITALIC
STYLE.RESET_NORMAL
STYLE.RESET_REVERSE
STYLE.RESET_STRIKE
STYLE.RESET_UNDERLINE
STYLE.REVERSE
STYLE.STRIKE
STYLE.UNDERLINE
