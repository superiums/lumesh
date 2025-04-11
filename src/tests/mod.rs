use crate::{Diagnostic, Expression, SyntaxError, parse_script, tokenize};

#[track_caller]
fn tokenize_test(input: &str, expected: &str) {
    let (tokens, mut diagnostics) = tokenize(input);
    diagnostics.retain(|d| d != &Diagnostic::Valid);
    assert_eq!(diagnostics.as_slice(), []);

    let got = format!("{:#?}", tokens);
    assert_eq!(got.as_str(), expected);
}

#[track_caller]
fn tokenize_test_err(input: &str) {
    let (_, mut diagnostics) = tokenize(input);
    diagnostics.retain(|d| d != &Diagnostic::Valid);
    assert_ne!(diagnostics.as_slice(), []);
}

#[track_caller]
fn parse_test(input: &str, expected: &str) -> Result<(), nom::Err<SyntaxError>> {
    let expr = parse_script(input)?;
    let got = format!("{:#?}", expr);
    assert_eq!(got.as_str(), expected);
    Ok(())
}

// #[test]
// fn test_operator_precedence() {
//     parse_expr("a + b * c**d").should_parse_as("a + (b * (c**d))");
//     parse_expr("x = y ?? z").should_fail_with("Expected '??' operator handler");
// }
#[test]
fn test_conditional_operator() {
    let expr = parse_script("a ? b + c : d * e").unwrap();
    assert_eq!(
        expr,
        Expression::If(
            Box::new(Expression::Symbol("a".into())),
            Box::new(Expression::BinaryOp(
                "+".to_string(),
                Box::new(Expression::Symbol("b".into())),
                Box::new(Expression::Symbol("c".into()))
            )),
            Box::new(Expression::BinaryOp(
                "*".to_string(),
                Box::new(Expression::Symbol("d".into())),
                Box::new(Expression::Symbol("e".into()))
            ))
        )
    );
}

#[test]
fn test_unary_priority() {
    let expr = parse_script("-a ** 2").unwrap();
    assert_eq!(
        expr,
        Expression::UnaryOp(
            "-".into(),
            Box::new(Expression::BinaryOp(
                "**".to_string(),
                Box::new(Expression::Symbol("a".into())),
                Box::new(Expression::Integer(2))
            )),
            true
        )
    );
}

#[test]
fn test_operator_precedence() {
    assert_eq!(
        parse_script("2 + 3 * 4 ** 5").unwrap(),
        Expression::Do(vec![Expression::BinaryOp(
            "+".into(),
            Box::new(Expression::Integer(2)),
            Box::new(Expression::BinaryOp(
                "*".into(),
                Box::new(Expression::Integer(3)),
                Box::new(Expression::BinaryOp(
                    "**".into(),
                    Box::new(Expression::Integer(4)),
                    Box::new(Expression::Integer(5)),
                ))
            ))
        )])
    );
}

#[test]
fn test_multi_char_ops() {
    assert!(parse_script("a && b || c ** d").is_ok());
    assert!(parse_script("x = y |> filter()").is_ok());
}
// #[test]
// fn test_nested_control_flow() {
//     parse_script(
//         r#"
//         if a {
//             for i in [1,2,3] {
//                 echo $i |> filter
//             }
//         } else if b {
//             while true { ... }
//         }
//     "#,
//     );
// }
#[test]
fn tokenize_function() {
    tokenize_test(
        r#"let a = foo -> bar -> {
    foo == bar
}"#,
        r#"[
    Keyword(0..3),
    Whitespace(3..4),
    Symbol(4..5),
    Whitespace(5..6),
    Operator(6..7),
    Whitespace(7..8),
    Symbol(8..11),
    Whitespace(11..12),
    Punctuation(12..14),
    Whitespace(14..15),
    Symbol(15..18),
    Whitespace(18..19),
    Punctuation(19..21),
    Whitespace(21..22),
    Punctuation(22..23),
    Whitespace(23..28),
    Symbol(28..31),
    Whitespace(31..32),
    Operator(32..34),
    Whitespace(34..35),
    Symbol(35..38),
    LineBreak(38..39),
    Punctuation(39..40),
]"#,
    );
}

#[test]
fn tokenize_string() {
    tokenize_test(
        r#"let a = "Hello \t world \u{254B} \"\\";"#,
        r#"[
    Keyword(0..3),
    Whitespace(3..4),
    Symbol(4..5),
    Whitespace(5..6),
    Operator(6..7),
    Whitespace(7..8),
    StringLiteral(8..38),
    LineBreak(38..39),
]"#,
    );
}

#[test]
fn tokenize_object() {
    tokenize_test(
        r#"{a=5, b = "hello"}"#,
        r#"[
    Punctuation(0..1),
    Symbol(1..2),
    Operator(2..3),
    IntegerLiteral(3..4),
    Punctuation(4..5),
    Whitespace(5..6),
    Symbol(6..7),
    Whitespace(7..8),
    Operator(8..9),
    Whitespace(9..10),
    StringLiteral(10..17),
    Punctuation(17..18),
]"#,
    );
}

#[test]
fn tokenize_unclosed_string() {
    tokenize_test(
        r#""Hello"#,
        r#"[
    StringLiteral(0..6),
]"#,
    );
}

#[test]
fn tokenize_numbers() {
    tokenize_test(
        r#"3 -4 1.1 4646345653 -3.14159"#,
        r#"[
    IntegerLiteral(0..1),
    Whitespace(1..2),
    IntegerLiteral(2..4),
    Whitespace(4..5),
    FloatLiteral(5..8),
    Whitespace(8..9),
    IntegerLiteral(9..19),
    Whitespace(19..20),
    FloatLiteral(20..28),
]"#,
    );
}

#[test]
fn tokenize_comments() {
    tokenize_test(
        r#"# test
let x # test
= 3;"#,
        r#"[
    Comment(0..6),
    LineBreak(6..7),
    Keyword(7..10),
    Whitespace(10..11),
    Symbol(11..12),
    Whitespace(12..13),
    Comment(13..19),
    LineBreak(19..20),
    Operator(20..21),
    Whitespace(21..22),
    IntegerLiteral(22..23),
    LineBreak(23..24),
]"#,
    );
}

#[test]
fn tokenize_symbols_and_operators() {
    tokenize_test(
        r#"to == != >= <= && || \\ // < > + - * / % | >> @ _ -.~\/?&$^: _*+ a%b+c>d abcX"#,
        r#"[
    Operator(0..2),
    Whitespace(2..3),
    Operator(3..5),
    Whitespace(5..6),
    Operator(6..8),
    Whitespace(8..9),
    Operator(9..11),
    Whitespace(11..12),
    Operator(12..14),
    Whitespace(14..15),
    Operator(15..17),
    Whitespace(17..18),
    Operator(18..20),
    Whitespace(20..21),
    Symbol(21..23),
    Whitespace(23..24),
    Symbol(24..26),
    Whitespace(26..27),
    Operator(27..28),
    Whitespace(28..29),
    Operator(29..30),
    Whitespace(30..31),
    Operator(31..32),
    Whitespace(32..33),
    Operator(33..34),
    Whitespace(34..35),
    Operator(35..36),
    Whitespace(36..37),
    Operator(37..38),
    Whitespace(38..39),
    Operator(39..40),
    Whitespace(40..41),
    Operator(41..42),
    Whitespace(42..43),
    Operator(43..45),
    Whitespace(45..46),
    Operator(46..47),
    Whitespace(47..48),
    Symbol(48..49),
    Whitespace(49..50),
    Symbol(50..59),
    Operator(59..60),
    Whitespace(60..61),
    Operator(61..64),
    Whitespace(64..65),
    Symbol(65..66),
    Operator(66..67),
    Symbol(67..68),
    Operator(68..69),
    Symbol(69..70),
    Operator(70..71),
    Symbol(71..72),
    Whitespace(72..73),
    Symbol(73..77),
]"#,
    );
}

#[test]
fn tokenize_invalid_numbers() {
    tokenize_test_err(r#"3."#);
    tokenize_test_err(r#"-15."#);
}

#[test]
fn tokenize_invalid_strings() {
    tokenize_test_err(r#""\"#);
    tokenize_test_err(r#""\x""#);
    tokenize_test_err(r#""\u""#);
    tokenize_test_err(r#""\u{}""#);
    tokenize_test_err(r#""\u{FFFFFF}""#); // at most 5 hex digits allowed
    tokenize_test_err(r#""\u{D800}""#); // lower surrogate
    tokenize_test_err(r#""\u{g}""#); // not a hex digit
}

#[test]
fn tokenize_invalid_symbols() {
    tokenize_test_err(r#"`"#);
    tokenize_test_err(r#"§"#);
    tokenize_test_err(r#"°"#);
    tokenize_test_err(r#"–"#); // em dash
    tokenize_test_err(r#"ä"#); // German umlaut
    tokenize_test_err(r#"€"#); // Euro sign
}

#[test]
fn parse1() -> Result<(), nom::Err<SyntaxError>> {
    parse_test(r#""String\t\r\n\"""#, r#"{ "String\t\r\n\"" }"#)
}

#[test]
fn parse2() -> Result<(), nom::Err<SyntaxError>> {
    parse_test(
        r#"let hello = "world\u{21}";"#,
        r#"{ let hello = "world!" }"#,
    )
}

#[test]
fn parse3() -> Result<(), nom::Err<SyntaxError>> {
    parse_test(
        r#"let + = a -> b -> c -> (+ a b c)"#,
        r#"{ let + = a -> b -> c -> (+ a b c) }"#,
    )
}
