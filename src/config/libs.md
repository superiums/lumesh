## Built-in Modules Detail
the following module is structured as:
```
### module_name
group description
- method => "description", "params"
```

### top method
calling without module_name.

- exit => "exit the shell", "[status]"
- cd => "change current directory", "[path]"
- cwd => "print current working directory", ""

env control
- set => "define a variable in root environment", "<var> <val>"
- unset => "undefine a variable in root environment", "<var>"

I/O operations
- symof => "get type of data symbol", "<value>"
- tap => "print and return result", "<args>..."
- print => "print arguments without newline", "<args>..."
- pprint => "pretty print", "<list>|<map>"
- println => "print arguments with newline", "<args>..."
- printf => "print formatted string with vars", "<template> <args>..."
- eprint => "print to stderr without newline", "<args>..."
- eprintln => "print to stderr with newline", "<args>..."
- debug => "print debug representation", "<args>..."
- ddebug => "pretty debug", "<args>..."
- read => "get user input", "[prompt]"
- throw => "return a runtime error", "<msg>"

Data manipulation
- get => "get value from nested map/list/range using dot notation path", "<map|list|range> <path>"
- typeof => "get data type", "<value>"
- len => "get length of expression", "<collection>"
- rev => "reverse sequence", "<string|list|bytes>"
- flatten => "flatten nested structure", "<collection>"
- where => "filter rows by condition", "<list[map]> <condition> "
- select => "select columns from list of maps", "<table> <columns...>"
- sortby => "sort a table by column", "<table> <col>"
- not => "logic not", "<boolean1>..."

Execution control
- repeat => "evaluate without env change", "<expr>"
- eval => "evaluate expression in current env", "<expr>"
- exec => "execute expression in new env", "<expr>"
- eval_str => "evaluate string in current env", "<expr>"
- exec_str => "execute string in new env", "<string>"
- include => "evaluate file in current env", "<path>"
- import => "evaluate file in new env", "<path>"

env
- set_root => "define a variable in root environment", "<var> <val>"
- unset_root => "undefine a variable in root environment", "<var>"
- getvar => "get a variable value", "<var>"

Help system
- help => "display help", "[module]"

### string
转换
- to_int => "convert a float or string to an int", "<value>"
- to_float => "convert an int or string to a float", "<value>"
- to_filesize => "parse a string representing a file size into bytes", "<size_str>"
- to_time => "convert a string to a datetime", "<datetime_str> [datetime_template]"
- to_table => "convert third-party command output to a table", "<command_output>"

基础检查
- is_empty => "is this string empty?", "<string>"
- is_whitespace => "is this string whitespace?", "<string>"
- is_alpha => "is this string alphabetic?", "<string>"
- is_alphanumeric => "is this string alphanumeric?", "<string>"
- is_numeric => "is this string numeric?", "<string>"
- is_lower => "is this string lowercase?", "<string>"
- is_upper => "is this string uppercase?", "<string>"
- is_title => "is this string title case?", "<string>"
- len => "get length of string", "<string>"

子串检查
- starts_with => "check if a string starts with a given substring", "<string> <substring>"
- ends_with => "check if a string ends with a given substring", "<string> <substring>"
- contains => "check if a string contains a given substring", "<string> <substring>"

分割操作
- split => "split a string on a given character", "<string> [delimiter]"
- split_at => "split a string at a given index", "<string> <index>"
- chars => "split a string into characters", "<string>"
- words => "split a string into words", "<string>"
- words_quoted => "split a string into words,quoted as one", "<string>"
- lines => "split a string into lines", "<string>"
- paragraphs => "split a string into paragraphs", "<string>"
- concat => "concat strings", "<string>..."

修改操作
- insert => "insert chars to a string", "<string> <index> <string>"
- repeat => "repeat string specified number of times", "<string> <count>"
- replace => "replace all instances of a substring", "<string> <old> <new>"
- substring => "get substring from start to end indices", "<string> <start> <end>"
- remove_prefix => "remove prefix if present", "<string> <prefix>"
- remove_suffix => "remove suffix if present", "<string> <suffix>"
- trim => "trim whitespace from a string", "<string>"
- trim_start => "trim whitespace from the start", "<string>"
- trim_end => "trim whitespace from the end", "<string>"
- to_lower => "convert a string to lowercase", "<string>"
- to_upper => "convert a string to uppercase", "<string>"
- to_title => "convert a string to title case", "<string>"

高级操作
- caesar => "encrypt a string using a caesar cipher", "<string> <shift>"
- max_len => "get max length of lines", "<string>"
- grep => "find lines which contains the substring", "<string> <substring>"
- strip => "remove all ANSI escape codes from string", "<string>"

格式化
- pad_start => "pad string to specified length at start", "<string> <length> [pad_char]"
- pad_end => "pad string to specified length at end", "<string> <length> [pad_char]"
- center => "center string by padding both ends", "<string> <length> [pad_char]"
- wrap => "wrap text to fit in specific number of columns", "<string> <width>"

样式
- href => "create terminal hyperlink", "<url> <text>"
- bold => "apply bold styling", "<string>"
- dim => "apply dim styling", "<string>"
- italic => "apply italic styling", "<string>"
- underline => "apply underline styling", "<string>"
- blink => "apply blinking effect", "<string>"
- invert => "invert foreground/background colors", "<string>"
- strike => "apply strikethrough styling", "<string>"

标准颜色
- black => "apply black foreground", "<string>"
- red => "apply red foreground", "<string>"
- green => "apply green foreground", "<string>"
- yellow => "apply yellow foreground", "<string>"
- blue => "apply blue foreground", "<string>"
- magenta => "apply magenta foreground", "<string>"
- cyan => "apply cyan foreground", "<string>"
- white => "apply white foreground", "<string>"

高级颜色
- clr => "apply color using 256-color code", "<string> <color_spec>"
- clr_bg => "apply background color using 256-color code", "<string> <color_spec>"
- color => "apply true color using RGB values or color_name", "<string> <hex_color|color_name|r,g,b>"
- color_bg => "apply True Color background using RGB values or color_name", "<string> <hex_color|color_name|r,g,b>"
- colors => "list all color_name for True Color", "[skip_colorized?]"

### list
数学统计
- max => "get max value in an array or multi args", "<num1> <num2> ... | <array>"
- min => "get min value in an array or multi args", "<num1> <num2> ... | <array>"
- sum => "sum a list of numbers", "<num1> <num2> ... | <array>"
- average => "get the average of a list of numbers", "<num1> <num2> ... | <array>"

读取操作
- get => "get value from nested map/list/range using dot notation path", "<map|list|range> <path>"
- len => "get length of list", "<list>"
- insert => "insert item into list", "<list> <index> <value>"
- rev => "reverse sequence", "<list>"
- flatten => "flatten nested structure", "<collection>"
- is_empty => "is this list empty?", "<list>"

- first => "get the first element of a list", "<list>"
- last => "get the last element of a list", "<list>"
- at => "get the nth element of a list", "<list> <index>"
- take => "take the first n elements of a list", "<list> <count>"
- drop => "drop the first n elements of a list", "<list> <count>"
查找操作
- contains => "check if list contains an item", "<list> <item>"
- find => "find first index of matching element", "<list> <item|fn> [skip_n]"
- find_last => "find last index of item", "<list> <item|fn> [skip_n]"

修改操作
- append => "append an element to a list", "<list> <element>"
- prepend => "prepend an element to a list", "<list> <element>"
- unique => "remove duplicates from a list while preserving order", "<list>"
- split_at => "split a list at a given index", "<list> <index>"
- splice => "change contents by removing/adding elements", "<start> <deleteCount> [items...] <list>"
- sort => "sort a string/list, optionally with a key function or key_list", "<string|list> [key_fn|key_list|keys...]"
- group => "group list elements by key function", "<list> <key_fn|key>"
- remove_at => "remove n elements starting from index", "<list> <index> [count]"
- remove => "remove first matching element", "<list> <item> [all?]"
- set => "set element at existing index", "<list> <index> <value>"
创建操作
- concat => "concatenate multiple lists into one", "<list1|item1> <list2|item2> ..."
- from => "create a list from a range", "<range|item...>"

遍历操作
- map => "apply function for each element", "<list> <fn>"
- items => "iterate over index-value pairs", "<list>"
- filter => "filter elements by condition", "<list> <fn>"
- filter_map => "filter and map in one pass", "<list> <fn>"
- any => "test if any element passes condition", "<list> <fn>"
- all => "test if all elements pass condition", "<list> <fn>"

转换操作
- join => "join string list with separator", "<list> <separator>"
- to_map => "convert list to btreeMap using key function", "<list> [key_fn] [val_fn]"
- to_hmap => "convert list to hashMap using key function", "<list> [key_fn] [val_fn]"
- to_set => "convert list to btreeSet", "<list>"

结构操作
- transpose => "transpose matrix (list of lists)", "<matrix>"
- chunk => "split list into chunks of size n", "<list> <size>"
- foldl => "fold list from left with function", "<list> <fn> <init>"
- foldr => "fold list from right with function", "<list> <fn> <init>"
- zip => "zip two lists into list of pairs", "<list1> <list2>"
- unzip => "unzip list of pairs into two lists", "<list_of_pairs>"
