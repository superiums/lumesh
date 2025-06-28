// use crate::{Diagnostic, SyntaxErrorKind, parse_script, tokenize};

// // ============================================================================
// // 测试辅助函数
// // ============================================================================

// #[track_caller]
// fn tokenize_test(input: &str, expected: &str) {
//     let (tokens, mut diagnostics) = tokenize(input);
//     diagnostics.retain(|d| d != &Diagnostic::Valid);
//     assert_eq!(
//         diagnostics.as_slice(),
//         [],
//         "Tokenization should not produce errors"
//     );

//     let got = format!("{:#?}", tokens);
//     assert_eq!(got.as_str(), expected, "Token output mismatch");
// }

// #[track_caller]
// fn tokenize_test_err(input: &str) {
//     let (_, mut diagnostics) = tokenize(input);
//     diagnostics.retain(|d| d != &Diagnostic::Valid);
//     assert_ne!(diagnostics.as_slice(), [], "Expected tokenization errors");
// }

// #[track_caller]
// fn parse_test(input: &str, expected: &str) -> Result<(), nom::Err<SyntaxErrorKind>> {
//     let expr = parse_script(input)?;
//     let got = format!("{}", expr);
//     assert_eq!(got.as_str(), expected, "Parse output mismatch");
//     Ok(())
// }

// // #[track_caller]
// // fn eval_test(input: &str, expected: Expression) {
// //     let expr = parse_script(input).expect("Parse should succeed");
// //     let mut env = Environment::new();
// //     let result = expr.eval(&mut env).expect("Evaluation should succeed");
// //     assert_eq!(result, expected, "Evaluation result mismatch");
// // }
// // #[track_caller]
// // fn eval_test_err(input: &str) {
// //     let expr = parse_script(input).expect("Parse should succeed");
// //     let mut env = Environment::new();
// //     assert!(expr.eval(&mut env).is_err(), "Expected evaluation error");
// // }

// // ============================================================================
// // 词法分析测试
// // ============================================================================
// #[test]
// fn test_tokenize_basic_literals() {
//     // 整数字面量
//     tokenize_test(
//         "42 -17 0",
//         r#"[
//     IntegerLiteral(0..2),
//     Whitespace(2..3),
//     IntegerLiteral(3..6),
//     Whitespace(6..7),
//     IntegerLiteral(7..8),
// ]"#,
//     );

//     // 浮点数字面量
//     tokenize_test(
//         "3.14 -2.5 0.0",
//         r#"[
//     FloatLiteral(0..4),
//     Whitespace(4..5),
//     FloatLiteral(5..9),
//     Whitespace(9..10),
//     FloatLiteral(10..13),
// ]"#,
//     );
// }

// #[test]
// fn test_tokenize_strings() {
//     // 基本字符串
//     tokenize_test(
//         r#""hello world""#,
//         r#"[
//     StringLiteral(0..13),
// ]"#,
//     );

//     // 转义字符
//     tokenize_test(
//         r#""Hello \t world \u{254B} \"\\""#,
//         r#"[
//     StringLiteral(0..30),
// ]"#,
//     );

//     // 单引号字符串（原始字符串）
//     tokenize_test(
//         r#"'raw\nstring'"#,
//         r#"[
//     StringLiteral(0..13),
// ]"#,
//     );
// }

// #[test]
// fn test_tokenize_identifiers_and_keywords() {
//     tokenize_test(
//         "let fn if else for while true false",
//         r#"[
//     Keyword(0..3),
//     Whitespace(3..4),
//     Keyword(4..6),
//     Whitespace(6..7),
//     Keyword(7..9),
//     Whitespace(9..10),
//     Keyword(10..14),
//     Whitespace(14..15),
//     Keyword(15..18),
//     Whitespace(18..19),
//     Keyword(19..24),
//     Whitespace(24..25),
//     Keyword(25..29),
//     Whitespace(29..30),
//     Keyword(30..35),
// ]"#,
//     );
// }

// #[test]
// fn test_tokenize_operators() {
//     tokenize_test(
//         "+ - * / % == != < > <= >= && || ! ? : = += -= *= /= %= |> .. -> =>",
//         r#"[
//     Operator(0..1),
//     Whitespace(1..2),
//     Operator(2..3),
//     Whitespace(3..4),
//     Operator(4..5),
//     Whitespace(5..6),
//     Operator(6..7),
//     Whitespace(7..8),
//     Operator(8..9),
//     Whitespace(9..10),
//     Operator(10..12),
//     Whitespace(12..13),
//     Operator(13..15),
//     Whitespace(15..16),
//     Operator(16..17),
//     Whitespace(17..18),
//     Operator(18..19),
//     Whitespace(19..20),
//     Operator(20..22),
//     Whitespace(22..23),
//     Operator(23..25),
//     Whitespace(25..26),
//     Operator(26..28),
//     Whitespace(28..29),
//     Operator(29..31),
//     Whitespace(31..32),
//     Operator(32..33),
//     Whitespace(33..34),
//     Operator(34..35),
//     Whitespace(35..36),
//     Operator(36..37),
//     Whitespace(37..38),
//     Operator(38..39),
//     Whitespace(39..40),
//     Operator(40..42),
//     Whitespace(42..43),
//     Operator(43..45),
//     Whitespace(45..46),
//     Operator(46..48),
//     Whitespace(48..49),
//     Operator(49..51),
//     Whitespace(51..52),
//     Operator(52..54),
//     Whitespace(54..55),
//     Operator(55..57),
//     Whitespace(57..58),
//     Operator(58..60),
//     Whitespace(60..61),
//     Operator(61..63),
//     Whitespace(63..64),
//     Operator(64..66),
// ]"#,
//     );
// }

// #[test]
// fn test_tokenize_punctuation() {
//     tokenize_test(
//         "( ) [ ] { } , ; .",
//         r#"[
//     Punctuation(0..1),
//     Whitespace(1..2),
//     Punctuation(2..3),
//     Whitespace(3..4),
//     Punctuation(4..5),
//     Whitespace(5..6),
//     Punctuation(6..7),
//     Whitespace(7..8),
//     Punctuation(8..9),
//     Whitespace(9..10),
//     Punctuation(10..11),
//     Whitespace(11..12),
//     Punctuation(12..13),
//     Whitespace(13..14),
//     Punctuation(14..15),
//     Whitespace(15..16),
//     Punctuation(16..17),
// ]"#,
//     );
// }

// #[test]
// fn test_tokenize_comments() {
//     tokenize_test(
//         r#"# This is a comment
// let x = 5 # inline comment"#,
//         r#"[
//     Comment(0..19),
//     LineBreak(19..20),
//     Keyword(20..23),
//     Whitespace(23..24),
//     Symbol(24..25),
//     Whitespace(25..26),
//     Operator(26..27),
//     Whitespace(27..28),
//     IntegerLiteral(28..29),
//     Whitespace(29..30),
//     Comment(30..45),
// ]"#,
//     );
// }

// #[test]
// fn test_tokenize_complex_expressions() {
//     // 函数定义
//     tokenize_test(
//         r#"fn add(a, b=10) { a + b }"#,
//         r#"[
//     Keyword(0..2),
//     Whitespace(2..3),
//     Symbol(3..6),
//     Punctuation(6..7),
//     Symbol(7..8),
//     Punctuation(8..9),
//     Whitespace(9..10),
//     Symbol(10..11),
//     Operator(11..12),
//     IntegerLiteral(12..14),
//     Punctuation(14..15),
//     Whitespace(15..16),
//     Punctuation(16..17),
//     Whitespace(17..18),
//     Symbol(18..19),
//     Whitespace(19..20),
//     Operator(20..21),
//     Whitespace(21..22),
//     Symbol(22..23),
//     Whitespace(23..24),
//     Punctuation(24..25),
// ]"#,
//     );

//     // 对象字面量
//     tokenize_test(
//         r#"{name: "Alice", age: 25, active: true}"#,
//         r#"[
//     Punctuation(0..1),
//     Symbol(1..5),
//     Operator(5..6),
//     Whitespace(6..7),
//     StringLiteral(7..14),
//     Punctuation(14..15),
//     Whitespace(15..16),
//     Symbol(16..19),
//     Operator(19..20),
//     Whitespace(20..21),
//     IntegerLiteral(21..23),
//     Punctuation(23..24),
//     Whitespace(24..25),
//     Symbol(25..31),
//     Operator(31..32),
//     Whitespace(32..33),
//     Keyword(33..37),
//     Punctuation(37..38),
// ]"#,
//     );

//     // 数组字面量
//     tokenize_test(
//         r#"[1, "two", [3, 4]]"#,
//         r#"[
//     Punctuation(0..1),
//     IntegerLiteral(1..2),
//     Punctuation(2..3),
//     Whitespace(3..4),
//     StringLiteral(4..9),
//     Punctuation(9..10),
//     Whitespace(10..11),
//     Punctuation(11..12),
//     IntegerLiteral(12..13),
//     Punctuation(13..14),
//     Whitespace(14..15),
//     IntegerLiteral(15..16),
//     Punctuation(16..17),
//     Punctuation(17..18),
// ]"#,
//     );
// }

// // ============================================================================
// // 词法分析错误测试
// // ============================================================================

// #[test]
// fn test_tokenize_invalid_numbers() {
//     tokenize_test_err("3.");
//     tokenize_test_err("-15.");
//     tokenize_test_err("1.2.3");
//     tokenize_test_err("1e");
//     tokenize_test_err("1e+");
// }

// #[test]
// fn test_tokenize_invalid_strings() {
//     tokenize_test_err(r#""\"#);
//     tokenize_test_err(r#""\x""#);
//     tokenize_test_err(r#""\u""#);
//     tokenize_test_err(r#""\u{}""#);
//     tokenize_test_err(r#""\u{FFFFFF}""#); // 超过5位十六进制数字
//     tokenize_test_err(r#""\u{D800}""#); // 低代理项
//     tokenize_test_err(r#""\u{g}""#); // 非十六进制数字
// }

// #[test]
// fn test_tokenize_invalid_symbols() {
//     tokenize_test_err("`");
//     tokenize_test_err("§");
//     tokenize_test_err("°");
//     tokenize_test_err("–"); // em dash
//     tokenize_test_err("ä"); // German umlaut
//     tokenize_test_err("€"); // Euro sign
// }

// // ============================================================================
// // 语法解析测试
// // ============================================================================

// #[test]
// fn test_parse_literals() -> Result<(), nom::Err<SyntaxErrorKind>> {
//     parse_test("42", "42")?;
//     parse_test("3.14", "3.14")?;
//     parse_test(r#""hello""#, r#""hello""#)?;
//     parse_test("true", "true")?;
//     parse_test("false", "false")?;
//     Ok(())
// }

// #[test]
// fn test_parse_binary_operations() -> Result<(), nom::Err<SyntaxErrorKind>> {
//     parse_test("1 + 2", "(1 + 2)")?;
//     parse_test("3 * 4 + 5", "((3 * 4) + 5)")?;
//     parse_test("2 + 3 * 4", "(2 + (3 * 4))")?;
//     parse_test("(2 + 3) * 4", "((2 + 3) * 4)")?;
//     Ok(())
// }

// #[test]
// fn test_parse_conditional_expressions() -> Result<(), nom::Err<SyntaxErrorKind>> {
//     parse_test("a ? b : c", "if a then b else c")?;
//     parse_test("x > 0 ? x : -x", "if (x > 0) then x else (-x)")?;
//     Ok(())
// }

// #[test]
// fn test_parse_function_definitions() -> Result<(), nom::Err<SyntaxErrorKind>> {
//     parse_test("fn add(a, b) { a + b }", "fn add(a, b) { (a + b) }")?;
//     parse_test(
//         "fn greet(name=\"World\") { \"Hello, \" + name }",
//         "fn greet(name=\"World\") { (\"Hello, \" + name) }",
//     )?;
//     Ok(())
// }

// #[test]
// fn test_parse_control_flow() -> Result<(), nom::Err<SyntaxErrorKind>> {
//     parse_test(
//         "if x > 0 { x } else { -x }",
//         "if (x > 0) then { x } else { (-x) }",
//     )?;
//     parse_test(
//         "for i in 0..10 { print(i) }",
//         "for i in (0..10) { print(i) }",
//     )?;
//     parse_test(
//         "while x > 0 { x = x - 1 }",
//         "while (x > 0) { (x = (x - 1)) }",
//     )?;
//     Ok(())
// }
