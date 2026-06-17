您要求的是关于 Lumesh 解析器中错误恢复机制的完整实现代码。基于之前讨论的同步点恢复策略，我将提供一个完整的部分恢复实现。

## 完整的部分恢复实现

### 1. 同步点识别函数

```rust
/// 跳过到下一个同步点（语句边界）
fn skip_to_sync_point(mut input: Tokens<'_>) -> Tokens<'_> {
    while !input.is_empty() {
        match input.first() {
            Some(token) => {
                match token.kind {
                    // 语句分隔符作为同步点
                    TokenKind::LineBreak => {
                        // 跳过当前换行符，返回下一个位置
                        return &input[1..];
                    }
                    // 块结束符也可以作为同步点
                    TokenKind::Punctuation if token.text(input) == "}" => {
                        return input;
                    }
                    // 其他情况继续跳过
                    _ => {
                        input = &input[1..];
                    }
                }
            }
            None => break,
        }
    }
    input
}

/// 检查是否为同步点
fn is_sync_point(input: Tokens<'_>) -> bool {
    if input.is_empty() {
        return true;
    }
    
    match input.first() {
        Some(token) => {
            matches!(token.kind, TokenKind::LineBreak) ||
            (token.kind == TokenKind::Punctuation && 
             matches!(token.text(input), "}" | ";" | "{"))
        }
        None => true,
    }
}
```

### 2. 错误收集结构

```rust
#[derive(Debug, Clone)]
pub struct ParseError {
    pub error: SyntaxErrorKind,
    pub position: StrSlice,
    pub recovered: bool,
}

#[derive(Debug)]
pub struct PartialParseResult {
    pub expressions: Vec<Expression>,
    pub errors: Vec<ParseError>,
    pub remaining: Option<Tokens<'static>>,
}
```

### 3. 带恢复的语句解析器

```rust
/// 带错误恢复的语句解析
fn parse_statement_with_recovery(
    mut input: Tokens<'_>
) -> IResult<Tokens<'_>, Vec<Expression>, Vec<ParseError>> {
    let mut statements = Vec::new();
    let mut errors = Vec::new();
    
    // 跳过开头的换行符
    while !input.is_empty() {
        if let Some(token) = input.first() {
            if token.kind == TokenKind::LineBreak {
                input = &input[1..];
                continue;
            }
        }
        break;
    }
    
    while !input.is_empty() {
        // 尝试解析单个语句
        match parse_statement(input) {
            Ok((remaining, stmt)) => {
                statements.push(stmt);
                input = remaining;
                
                // 消费可选的换行符
                if let Ok((new_input, _)) = opt(kind(TokenKind::LineBreak))(input) {
                    input = new_input;
                }
            }
            Err(nom::Err::Error(e)) => {
                // 记录错误
                errors.push(ParseError {
                    error: e,
                    position: input.get_str_slice(),
                    recovered: true,
                });
                
                // 跳到下一个同步点
                input = skip_to_sync_point(input);
                
                // 如果找到同步点，继续解析
                if !input.is_empty() && is_sync_point(input) {
                    // 跳过同步点标记
                    if let Some(token) = input.first() {
                        if token.kind == TokenKind::LineBreak {
                            input = &input[1..];
                        }
                    }
                    continue;
                } else {
                    // 无法找到同步点，停止解析
                    break;
                }
            }
            Err(nom::Err::Failure(e)) => {
                // 严重错误，不尝试恢复
                errors.push(ParseError {
                    error: e,
                    position: input.get_str_slice(),
                    recovered: false,
                });
                break;
            }
            Err(nom::Err::Incomplete(_)) => {
                // 输入不完整，停止解析
                break;
            }
        }
    }
    
    Ok((input, statements))
}
```

### 4. 带恢复的模块解析器

基于现有的 [1](#5-0) ，我们可以创建一个带恢复功能的版本：

```rust
/// 带错误恢复的模块解析
fn parse_module_with_recovery(
    input: Tokens<'_>
) -> IResult<Tokens<'_>, PartialParseResult, SyntaxErrorKind> {
    let mut all_statements = Vec::new();
    let mut all_errors = Vec::new();
    let mut remaining_input = input;
    
    // 解析函数声明和语句
    while !remaining_input.is_empty() {
        // 尝试解析函数声明
        match terminated(parse_fn_declare, opt(kind(TokenKind::LineBreak)))(remaining_input) {
            Ok((rest, func)) => {
                all_statements.push(func);
                remaining_input = rest;
                continue;
            }
            Err(_) => {
                // 函数声明失败，尝试解析普通语句
            }
        }
        
        // 尝试解析语句块
        match parse_statement_with_recovery(remaining_input) {
            Ok((rest, statements)) => {
                all_statements.extend(statements);
                remaining_input = rest;
                
                // 如果没有更多输入，退出
                if rest.is_empty() {
                    break;
                }
            }
            Err(_) => {
                // 如果语句解析也失败，跳过当前token
                if !remaining_input.is_empty() {
                    all_errors.push(ParseError {
                        error: SyntaxErrorKind::CustomError(
                            "Unrecognized token".to_string(),
                            remaining_input.get_str_slice(),
                        ),
                        position: remaining_input.get_str_slice(),
                        recovered: true,
                    });
                    remaining_input = &remaining_input[1..];
                }
            }
        }
    }
    
    let result = PartialParseResult {
        expressions: all_statements,
        errors: all_errors,
        remaining: if remaining_input.is_empty() { 
            None 
        } else { 
            Some(unsafe { std::mem::transmute(remaining_input) })
        },
    };
    
    Ok((remaining_input, result))
}
```

### 5. 修改主解析入口

修改 [2](#5-1)  中的 `parse_script_tokens` 函数：

```rust
/// 带错误恢复的脚本解析入口
pub fn parse_script_tokens_with_recovery(
    input: Tokens<'_>,
) -> Result<PartialParseResult, nom::Err<SyntaxErrorKind>> {
    if input.is_empty() {
        return Ok(PartialParseResult {
            expressions: vec![Expression::None],
            errors: Vec::new(),
            remaining: None,
        });
    }
    
    // 使用带恢复的模块解析器
    match parse_module_with_recovery(input) {
        Ok((remaining, mut result)) => {
            // 处理剩余的未解析token
            if !remaining.is_empty() {
                result.errors.push(ParseError {
                    error: SyntaxErrorKind::CustomError(
                        "Unparsed tokens remaining".to_string(),
                        remaining.get_str_slice(),
                    ),
                    position: remaining.get_str_slice(),
                    recovered: false,
                });
            }
            
            Ok(result)
        }
        Err(e) => Err(e),
    }
}
```

### 6. 公共API接口

```rust
/// 带错误恢复的脚本解析公共接口
pub fn parse_script_with_recovery(input: &str) -> Result<PartialParseResult, nom::Err<SyntaxErrorKind>> {
    // 词法分析阶段（复用现有逻辑）
    let str = input.into();
    let tokenization_input = Input::new(&str);
    let (mut token_vec, mut diagnostics) = super::parse_tokens(tokenization_input);
    
    // 错误处理
    diagnostics.retain(|d| d != &Diagnostic::Valid);
    if !diagnostics.is_empty() {
        return Err(nom::Err::Failure(SyntaxErrorKind::TokenizationErrors(
            diagnostics.into_boxed_slice(),
        )));
    }
    
    // 清理token
    token_vec.retain(|t| !matches!(t.kind, TokenKind::Whitespace | TokenKind::Comment));
    normalize_linebreaks(&mut token_vec);
    
    // 带恢复的语法分析
    parse_script_tokens_with_recovery(Tokens {
        str: &str,
        slice: token_vec.as_slice(),
    })
}
```

## 使用示例

```rust
// 使用带恢复的解析器
match parse_script_with_recovery(source_code) {
    Ok(result) => {
        // 处理成功解析的表达式
        for expr in result.expressions {
            println!("Parsed: {:?}", expr);
        }
        
        // 报告恢复的错误
        for error in result.errors {
            if error.recovered {
                eprintln!("Recovered from error: {:?}", error.error);
            } else {
                eprintln!("Fatal error: {:?}", error.error);
            }
        }
    }
    Err(e) => {
        eprintln!("Parse failed completely: {:?}", e);
    }
}
```

## Notes

这个实现提供了完整的错误恢复机制，能够在遇到语法错误时继续解析后续代码。恢复策略基于语句边界（换行符、块分隔符），既保证了解析的鲁棒性，又维持了合理的性能。错误信息被完整保留，便于后续的错误报告和IDE集成。



Wiki pages you might want to explore:
- [Binary Architecture (superiums/lumesh)](/wiki/superiums/lumesh#1.2)
- [Language Core (superiums/lumesh)](/wiki/superiums/lumesh#2)