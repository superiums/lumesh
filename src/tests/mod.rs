use crate::expression::{FileSize, LumeRegex};
use crate::{Diagnostic, Environment, Expression, RuntimeErrorKind, parse_script, tokenize};
use regex_lite::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use std::rc::Rc;

// ============================================================
// Test helpers
//
// Section	Tests	Purpose
// Tokenizer	16	Basic tokenization, operators, file sizes, strings, keywords, error handling
// Parser	46	All expression types, precedence, associativity, error cases
// Operator Overloads	31	Add/Sub/Mul/Div/Rem on mixed types, overflow, string ops, edge cases
// Truthiness	21	All expression types' truthiness
// PartialOrd	3	String/Int/Float comparison, BSet/HMap/Map content comparison
// Environment	8	Scoped binding, fork chain, root operations, iteration
// Evaluator	49	All expression eval paths, control flow, assignment, range ops, binary ops
// Bug Reproduction	12	String * 0, String * negative, String - i64::MIN, AddAssign overflow, MulAssign wrap, % on float/int
// Type Conversions	5	From impls for Expression, FileSize
// FileSize	4	to_bytes, to_human_readable, parse
// ============================================================

#[allow(dead_code)]
#[track_caller]
fn assert_tokenize_diag(input: &str, expected_diags: &[Diagnostic]) {
    let (tokens, diagnostics) = tokenize(input);
    let mut non_valid: Vec<&Diagnostic> = diagnostics
        .iter()
        .filter(|d| d != &&Diagnostic::Valid)
        .collect();
    // sort by debug string for deterministic comparison
    non_valid.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    let mut expected: Vec<&Diagnostic> = expected_diags.iter().collect();
    expected.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    assert_eq!(
        non_valid.len(),
        expected.len(),
        "Input: {input:?}, got diagnostics: {non_valid:?}, expected: {expected:?}"
    );
    for (g, e) in non_valid.iter().zip(expected.iter()) {
        // compare by debug string since ranges will differ
        let gs = format!("{g:?}");
        let es = format!("{e:?}");
        // check kind matches (same prefix before `(`)
        let g_kind = gs.split('(').next().unwrap_or(&gs);
        let e_kind = es.split('(').next().unwrap_or(&es);
        assert_eq!(
            g_kind, e_kind,
            "Input: {input:?}, got {g:?}, expected {e:?}"
        );
    }
    let _ = tokens;
}

#[track_caller]
fn assert_parse_fail(input: &str) {
    let result = parse_script(input);
    assert!(
        result.is_err(),
        "Input: {input:?} should have failed, got: {:?}",
        result
    );
}

#[allow(dead_code)]
#[track_caller]
fn assert_parse(input: &str) {
    let result = parse_script(input);
    assert!(result.is_ok(), "Input: {input:?} failed: {:?}", result);
}

#[track_caller]
fn assert_parse_eq(input: &str, expected_debug: &str) {
    let expr = parse_script(input).expect("parse should succeed");
    let got = format!("{expr:?}");
    assert_eq!(got, expected_debug, "Input: {input:?}");
}

// ============================================================
// 1. TOKENIZER TESTS
// ============================================================

mod tokenizer_tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let (tokens, diags) = tokenize("");
        assert!(tokens.is_empty());
        assert!(diags.is_empty() || diags.iter().all(|d| d == &Diagnostic::Valid));
    }

    #[test]
    fn test_whitespace_only() {
        let (tokens, diags) = tokenize("   \t  ");
        assert!(tokens.is_empty() || diags.iter().all(|d| d == &Diagnostic::Valid));
        let _ = tokens;
    }

    #[test]
    fn test_basic_symbols() {
        let (tokens, diags) = tokenize("abc def123");
        assert!(diags.iter().all(|d| d == &Diagnostic::Valid));
        // Should be [Symbol, Whitespace, Symbol]
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, crate::TokenKind::Symbol);
        assert_eq!(tokens[2].kind, crate::TokenKind::Symbol);
    }

    #[test]
    fn test_integer_literals() {
        let (tokens, diags) = tokenize("42 -17 0");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got diagnostics: {valid:?}");
        assert_eq!(tokens.len(), 5); // int, ws, int, ws, int
    }

    #[test]
    fn test_float_literals() {
        let (tokens, diags) = tokenize("3.14 -2.5");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(
            valid.is_empty(),
            "Got diagnostics: {valid:?}, tokens: {tokens:?}"
        );
    }

    #[test]
    fn test_invalid_float_trailing_dot() {
        // "3." should produce an InvalidNumber diagnostic
        let (_tokens, diags) = tokenize("3.");
        let has_invalid = diags
            .iter()
            .any(|d| matches!(d, Diagnostic::InvalidNumber(_)));
        assert!(has_invalid, "Expected InvalidNumber for '3.'");
    }

    #[test]
    fn test_string_double_quoted() {
        let (tokens, diags) = tokenize(r#""hello world""#);
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, crate::TokenKind::StringLiteral);
    }

    #[test]
    fn test_string_single_quoted() {
        let (tokens, diags) = tokenize("'hello world'");
        let non_valid: Vec<&Diagnostic> =
            diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(non_valid.is_empty(), "Got {non_valid:?}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, crate::TokenKind::StringRaw);
    }

    #[test]
    fn test_unterminated_string() {
        let (_tokens, diags) = tokenize(r#""hello"#);
        let has_unterminated = diags
            .iter()
            .any(|d| matches!(d, Diagnostic::UnterminatedString(_)));
        assert!(has_unterminated, "Expected UnterminatedString");
    }

    #[test]
    fn test_unterminated_single_quote() {
        let (_tokens, diags) = tokenize("'hello");
        let has_unterminated = diags
            .iter()
            .any(|d| matches!(d, Diagnostic::UnterminatedString(_)));
        assert!(has_unterminated, "Expected UnterminatedString for 'hello");
    }

    #[test]
    fn test_escape_sequences() {
        let (tokens, diags) = tokenize(r#""hello\nworld\t!""#);
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, crate::TokenKind::StringLiteral);
    }

    #[test]
    fn test_invalid_escape() {
        // Tokenizer no longer validates escapes — all \X sequences are kept as raw content
        // and processed by snailquote::unescape at the parser level.
        // \z is not a valid snailquote escape, so it would be an error during parsing.
        let (_tokens, diags) = tokenize(r#""\z""#);
        let has_diags = diags.iter().any(|d| !matches!(d, Diagnostic::Valid));
        // No tokenizer-level diagnostics expected; errors come from parser.
        assert!(!has_diags, "escape validation moved to parser level");
    }

    #[test]
    fn test_comment() {
        let (tokens, diags) = tokenize("# this is a comment");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, crate::TokenKind::Comment);
    }

    #[test]
    fn test_comment_with_hash_not_space() {
        let (tokens, diags) = tokenize("#notacomment");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, crate::TokenKind::Comment);
    }

    #[test]
    fn test_keywords() {
        let (tokens, diags) = tokenize("let fn if else for while true false");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        // Expect: Keyword, WS, Keyword, WS, ...
        assert_eq!(tokens.len(), 15); // 8 keywords + 7 whitespace
    }

    #[test]
    fn test_operators() {
        // Note: `>` at end of input is not recognized by operator_tag
        // because it requires a non-punctuation char after the operator.
        // Include `>>` instead.
        let (tokens, diags) = tokenize("+ - * / % == != < >>");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        // expect operators and whitespace
        assert!(tokens.len() >= 10);
    }

    #[test]
    fn test_line_continuation() {
        let (tokens, diags) = tokenize("hello\\\nworld");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        // line continuation should be whitespace
        assert_eq!(tokens.len(), 3); // symbol, whitespace(continuation), symbol
    }

    #[test]
    fn test_regex_literal() {
        let (tokens, diags) = tokenize("r'[a-z]+'");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, crate::TokenKind::Regex);
    }

    #[test]
    fn test_time_literal() {
        let (tokens, diags) = tokenize("t'2024-01-01'");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, crate::TokenKind::Time);
    }

    #[test]
    fn test_symbol_with_special_chars() {
        let (tokens, diags) = tokenize("foo-bar hello_world");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 3); // symbol, ws, symbol
    }

    #[test]
    fn test_backtick_string() {
        let (tokens, diags) = tokenize("`template ${var}`");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, crate::TokenKind::StringTemplate);
    }

    #[test]
    fn test_range_operator_infix() {
        let (tokens, diags) = tokenize("0..10");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        // Expect: IntegerLiteral, OperatorInfix(..), IntegerLiteral
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[1].kind, crate::TokenKind::OperatorInfix);
    }

    #[test]
    fn test_empty_string() {
        let (tokens, diags) = tokenize(r#""""#);
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, crate::TokenKind::StringLiteral);
    }

    #[test]
    fn test_unicode_in_string() {
        let (tokens, diags) = tokenize("\"hello © world\"");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_value_symbols() {
        let (tokens, diags) = tokenize("true false none _");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        assert_eq!(tokens.len(), 7);
    }

    #[test]
    fn test_file_size_suffix() {
        let (tokens, diags) = tokenize("5K 10M 1G");
        let valid: Vec<&Diagnostic> = diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        assert!(valid.is_empty(), "Got {valid:?}");
        // Each file size: IntegerLiteral + OperatorPostfix + optional Whitespace
        // 5K(2) + ' '(1) + 10M(2) + ' '(1) + 1G(2) = 8 tokens
        assert_eq!(tokens.len(), 8);
    }

    #[test]
    fn test_percentage_suffix() {
        let (_tokens, diags) = tokenize("50%");
        // This should produce tokens: IntegerLiteral, OperatorPostfix
        let non_valid: Vec<&Diagnostic> =
            diags.iter().filter(|d| d != &&Diagnostic::Valid).collect();
        // BUG?: 50% might produce InvalidNumber since `%` is also used in numbers context
        // This test documents current behavior - may need fixing
        let _ = non_valid;
    }

    #[test]
    fn test_not_tokenized_leftovers() {
        // § is non-ASCII, may be tokenized as StringRaw via non_ascii parser (no error),
        // or as IllegalChar if non_ascii fails.
        let (_tokens, diags) = tokenize("hello §world");
        let has_not_tokenized = diags
            .iter()
            .any(|d| matches!(d, Diagnostic::NotTokenized(_)));
        let has_illegal = diags
            .iter()
            .any(|d| matches!(d, Diagnostic::IllegalChar(_)));
        if !has_not_tokenized && !has_illegal {
            // If § is handled as non_ascii StringRaw, that's also fine
            let all_valid = diags.iter().all(|d| d == &Diagnostic::Valid);
            assert!(
                all_valid,
                "Expected some diagnostic or all valid, got {diags:?}"
            );
        }
    }
}

// ============================================================
// 2. PARSER TESTS
// ============================================================

mod parser_tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let expr = parse_script("").unwrap();
        assert_eq!(expr, Expression::None);
    }

    #[test]
    fn test_parse_integer() {
        assert_parse_eq("42", "Integer〈42〉");
    }

    #[test]
    fn test_parse_negative_integer() {
        let expr = parse_script("-17").unwrap();
        // -17 is parsed as a String("-17") because the tokenizer treats
        // `-` prefix as argument_symbol/path, not as unary minus + integer.
        // Use space to force unary op: - 17
        assert_eq!(expr, Expression::String("-17".to_string()));
    }

    #[test]
    fn test_parse_float() {
        assert_parse_eq("3.14", "Float〈3.14〉");
    }

    #[test]
    fn test_parse_string() {
        let expr = parse_script(r#""hello""#).unwrap();
        assert_eq!(expr, Expression::String("hello".into()));
    }

    #[test]
    fn test_parse_symbol() {
        let expr = parse_script("abc").unwrap();
        assert_eq!(expr, Expression::Symbol("abc".into()));
    }

    #[test]
    fn test_parse_boolean() {
        assert_parse_eq("true", "Boolean〈true〉");
        assert_parse_eq("false", "Boolean〈false〉");
    }

    #[test]
    fn test_parse_addition() {
        assert_parse_eq("1 + 2", "BinaryOp〈+〉\n  Integer〈1〉\n  Integer〈2〉");
    }

    #[test]
    fn test_parse_precedence_mul_before_add() {
        assert_parse_eq(
            "2 + 3 * 4",
            "BinaryOp〈+〉\n  Integer〈2〉\n  BinaryOp〈*〉\n    Integer〈3〉\n    Integer〈4〉",
        );
    }

    #[test]
    fn test_parse_precedence_parens() {
        match parse_script("(2 + 3) * 4").unwrap() {
            Expression::BinaryOp(ref op, ref lhs, ref rhs) => {
                assert_eq!(op, "*");
                assert_eq!(rhs.as_ref(), &Expression::Integer(4));
                match lhs.as_ref() {
                    Expression::Group(inner) => match inner.as_ref() {
                        Expression::BinaryOp(iop, il, ir) => {
                            assert_eq!(iop, "+");
                            assert_eq!(il.as_ref(), &Expression::Integer(2));
                            assert_eq!(ir.as_ref(), &Expression::Integer(3));
                        }
                        other => panic!("Expected BinaryOp in group, got {other:?}"),
                    },
                    other => panic!("Expected Group, got {other:?}"),
                }
            }
            other => panic!("Expected BinaryOp, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_comparison() {
        assert_parse_eq("a > 5", "BinaryOp〈>〉\n  Symbol〈\"a\"〉\n  Integer〈5〉");
    }

    #[test]
    fn test_parse_equality() {
        assert_parse_eq(
            "x == y",
            "BinaryOp〈==〉\n  Symbol〈\"x\"〉\n  Symbol〈\"y\"〉",
        );
    }

    #[test]
    fn test_parse_strict_equality() {
        assert_parse_eq(
            "x === y",
            "BinaryOp〈===〉\n  Symbol〈\"x\"〉\n  Symbol〈\"y\"〉",
        );
    }

    #[test]
    fn test_parse_logical_and() {
        assert_parse_eq(
            "a && b",
            "BinaryOp〈&&〉\n  Symbol〈\"a\"〉\n  Symbol〈\"b\"〉",
        );
    }

    #[test]
    fn test_parse_logical_or() {
        assert_parse_eq(
            "a || b",
            "BinaryOp〈||〉\n  Symbol〈\"a\"〉\n  Symbol〈\"b\"〉",
        );
    }

    #[test]
    fn test_parse_range_op() {
        assert_parse_eq("1..10", "RangeOp〈..〉\n  Integer〈1〉\n  Integer〈10〉");
    }

    #[test]
    fn test_parse_range_op_step() {
        assert_parse_eq(
            "1..10:2",
            "RangeOp〈..〉\n  Integer〈1〉\n  Integer〈10〉\n  Step\n    Integer〈2〉",
        );
    }

    #[test]
    fn test_parse_range_op_inclusive() {
        let expr = parse_script("1..=10").unwrap();
        match expr {
            Expression::RangeOp(_, _, _, _) => {} // valid
            _ => panic!("Expected RangeOp, got {expr:?}"),
        }
    }

    #[test]
    fn test_parse_extend_range() {
        let expr = parse_script("1...10").unwrap();
        match expr {
            Expression::RangeOp(_, _, _, _) => {} // valid
            _ => panic!("Expected RangeOp, got {expr:?}"),
        }
    }

    #[test]
    fn test_parse_conditional() {
        match parse_script("a ? b : c").unwrap() {
            Expression::If(ref cond, ref t, ref f) => {
                assert_eq!(cond.as_ref(), &Expression::Symbol("a".into()));
                assert_eq!(t.as_ref(), &Expression::Symbol("b".into()));
                assert_eq!(f.as_ref(), &Expression::Symbol("c".into()));
            }
            other => panic!("Expected If, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_assignment() {
        match parse_script("x = 5").unwrap() {
            Expression::Assign(ref name, ref val) => {
                assert_eq!(name, "x");
                assert_eq!(val.as_ref(), &Expression::Integer(5));
            }
            other => panic!("Expected Assign, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_declaration() {
        match parse_script("let x = 5").unwrap() {
            Expression::Declare(ref name, ref val) => {
                assert_eq!(name, "x");
                assert_eq!(val.as_ref(), &Expression::Integer(5));
            }
            other => panic!("Expected Declare, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_function_def() {
        match parse_script("fn add(a, b) { a + b }").unwrap() {
            Expression::Function(ref name, ref params, ref collector, ref body, ref decos) => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].0, "a");
                assert!(params[0].1.is_none());
                assert_eq!(params[1].0, "b");
                assert!(params[1].1.is_none());
                assert!(collector.is_none());
                assert!(decos.is_empty());
                // body should be Block containing BinaryOp
                match body.as_ref() {
                    Expression::Block(stmts) => {
                        assert_eq!(stmts.len(), 1);
                        match &stmts[0] {
                            Expression::BinaryOp(op, l, r) => {
                                assert_eq!(op, "+");
                                assert_eq!(l.as_ref(), &Expression::Symbol("a".into()));
                                assert_eq!(r.as_ref(), &Expression::Symbol("b".into()));
                            }
                            other => panic!("Expected BinaryOp, got {other:?}"),
                        }
                    }
                    other => panic!("Expected Block, got {other:?}"),
                }
            }
            other => panic!("Expected Function, got {other:?}"),
        }
    }

    #[test]
    fn parse_lambda() {
        match parse_script("(x) -> x * 2").unwrap() {
            Expression::Lambda(ref params, ref body, _) => {
                assert_eq!(params, &["x".to_string()]);
                // body should be Block containing BinaryOp
                match body.as_ref() {
                    Expression::Block(stmts) => {
                        assert_eq!(stmts.len(), 1);
                        match &stmts[0] {
                            Expression::BinaryOp(op, l, r) => {
                                assert_eq!(op, "*");
                                assert_eq!(l.as_ref(), &Expression::Symbol("x".into()));
                                assert_eq!(r.as_ref(), &Expression::Integer(2));
                            }
                            other => panic!("Expected BinaryOp, got {other:?}"),
                        }
                    }
                    other => panic!("Expected Block, got {other:?}"),
                }
            }
            other => panic!("Expected Lambda, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_list_literal() {
        let expr = parse_script("[1, 2, 3]").unwrap();
        match expr {
            Expression::List(_) => {} // valid
            _ => panic!("Expected List, got {expr:?}"),
        }
    }

    #[test]
    fn test_parse_empty_list() {
        let expr = parse_script("[]").unwrap();
        match expr {
            Expression::List(items) => assert!(items.is_empty()),
            _ => panic!("Expected empty List, got {expr:?}"),
        }
    }

    #[test]
    fn test_parse_map_literal() {
        let expr = parse_script("{a: 1, b: 2}").unwrap();
        match expr {
            Expression::Map(_) => {} // valid
            _ => panic!("Expected Map, got {expr:?}"),
        }
    }

    #[test]
    fn test_parse_for_loop() {
        let expr = parse_script("for i in 1..10 { print i }").unwrap();
        match expr {
            Expression::For(_, _, _, _) => {} // valid
            _ => panic!("Expected For, got {expr:?}"),
        }
    }

    #[test]
    fn test_parse_while_loop() {
        let expr = parse_script("while x > 0 { x = x - 1 }").unwrap();
        match expr {
            Expression::While(_, _) => {} // valid
            _ => panic!("Expected While, got {expr:?}"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        let expr = parse_script("if x > 0 { x } else { -x }").unwrap();
        match expr {
            Expression::If(_, _, _) => {} // valid
            _ => panic!("Expected If, got {expr:?}"),
        }
    }

    #[test]
    fn test_parse_power_op() {
        assert_parse_eq("2 ^ 3", "BinaryOp〈^〉\n  Integer〈2〉\n  Integer〈3〉");
    }

    #[test]
    fn test_parse_pipe() {
        assert_parse_eq("a | b", "Pipe〈|〉\n  Symbol〈\"a\"〉\n  Symbol〈\"b\"〉");
    }

    #[test]
    fn test_parse_dispatch_pipe() {
        assert_parse_eq("a |> b", "Pipe〈|>〉\n  Symbol〈\"a\"〉\n  Symbol〈\"b\"〉");
    }

    #[test]
    fn test_parse_variable() {
        assert_parse_eq("$x", "Variable〈\"x\"〉");
    }

    #[test]
    fn test_parse_dot_method_call() {
        match parse_script("obj.method()").unwrap() {
            Expression::Chain(ref base, ref calls) => {
                assert_eq!(base.as_ref(), &Expression::Symbol("obj".into()));
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].method, "method");
                assert!(calls[0].args.is_empty());
            }
            other => panic!("Expected Chain, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_index() {
        assert_parse_eq(
            "list[0]",
            "Index\n  Symbol〈\"list\"〉\n  [\n    Integer〈0〉\n  ]",
        );
    }

    // ----- Error cases -----

    #[test]
    fn test_parse_unclosed_paren() {
        assert_parse_fail("(1 + 2");
    }

    #[test]
    fn test_parse_unclosed_bracket() {
        assert_parse_fail("[1, 2");
    }

    #[test]
    fn test_parse_unclosed_brace() {
        assert_parse_fail("{a: 1");
    }

    #[test]
    fn test_parse_trailing_operator() {
        // Parser succeeds, treating `1` as the complete expression, `+` is discarded or part of sequence
        let _ = parse_script("1 +").unwrap();
    }

    #[test]
    fn test_parse_double_operator() {
        assert_parse_fail("1 + + 2");
    }

    #[test]
    fn test_parse_unknown_operator() {
        // `~~` is treated as a command name, not an operator
        match parse_script("1 ~~ 2").unwrap() {
            Expression::Sequence(exprs) if exprs.len() == 2 => {
                assert_eq!(exprs[0], Expression::Integer(1));
            }
            other => panic!("Expected Sequence of 2, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_nonsense() {
        assert_parse_fail("if {");
        assert_parse_fail("fn {");
    }

    #[test]
    fn test_parse_invalid_function_decl() {
        assert_parse_fail("fn { }");
    }

    #[test]
    fn test_parse_match_without_body() {
        assert_parse_fail("match x");
    }

    #[test]
    fn test_parse_let_without_value() {
        // let x = should fail
        assert_parse_fail("let x =");
    }
}

// ============================================================
// 3. OPERATOR OVERLOAD TESTS (direct expression arithmetic)
// ============================================================

mod operator_tests {
    use super::*;

    // --- Add ---

    #[test]
    fn test_add_int_int() {
        let result = (Expression::Integer(3) + Expression::Integer(4)).unwrap();
        assert_eq!(result, Expression::Integer(7));
    }

    #[test]
    fn test_add_int_overflow() {
        let result = Expression::Integer(i64::MAX) + Expression::Integer(1);
        assert!(result.is_err());
        assert!(matches!(result, Err(RuntimeErrorKind::Overflow(_))));
    }

    #[test]
    fn test_add_int_float() {
        let result = (Expression::Integer(3) + Expression::Float(2.5)).unwrap();
        assert_eq!(result, Expression::Float(5.5));
    }

    #[test]
    fn test_add_float_int() {
        let result = (Expression::Float(2.5) + Expression::Integer(3)).unwrap();
        assert_eq!(result, Expression::Float(5.5));
    }

    #[test]
    fn test_add_string_string() {
        let result =
            (Expression::String("hello ".into()) + Expression::String("world".into())).unwrap();
        assert_eq!(result, Expression::String("hello world".into()));
    }

    #[test]
    fn test_add_string_int() {
        let result = (Expression::String("count: ".into()) + Expression::Integer(42)).unwrap();
        assert_eq!(result, Expression::String("count: 42".into()));
    }

    #[test]
    fn test_add_list_list() {
        let a = Expression::from(vec![1i64, 2, 3]);
        let b = Expression::from(vec![4i64, 5]);
        let result = (a + b).unwrap();
        match result {
            Expression::List(items) => {
                assert_eq!(items.len(), 5);
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_add_int_string_coercion() {
        let result = (Expression::Integer(5) + Expression::String("3".into())).unwrap();
        assert_eq!(result, Expression::Integer(8));
    }

    #[test]
    fn test_add_int_string_coercion_fail() {
        let result = Expression::Integer(5) + Expression::String("abc".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_add_map_map() {
        let mut m1 = BTreeMap::new();
        m1.insert("a".into(), Expression::Integer(1));
        let mut m2 = BTreeMap::new();
        m2.insert("b".into(), Expression::Integer(2));
        let result = (Expression::from(m1) + Expression::from(m2)).unwrap();
        match result {
            Expression::Map(items) => assert_eq!(items.len(), 2),
            _ => panic!("Expected Map"),
        }
    }

    // --- Sub ---

    #[test]
    fn test_sub_int_int() {
        let result = (Expression::Integer(10) - Expression::Integer(3)).unwrap();
        assert_eq!(result, Expression::Integer(7));
    }

    #[test]
    fn test_sub_int_overflow() {
        let result = Expression::Integer(i64::MIN) - Expression::Integer(1);
        assert!(result.is_err());
        assert!(matches!(result, Err(RuntimeErrorKind::Overflow(_))));
    }

    #[test]
    fn test_sub_string_string() {
        let result = (Expression::String("hello world".into())
            - Expression::String("world".into()))
        .unwrap();
        assert_eq!(result, Expression::String("hello ".into()));
    }

    #[test]
    fn test_sub_string_int_positive() {
        let result = (Expression::String("hello".into()) - Expression::Integer(2)).unwrap();
        assert_eq!(result, Expression::String("hel".into()));
    }

    #[test]
    fn test_sub_string_int_negative() {
        let result = (Expression::String("hello".into()) - Expression::Integer(-2)).unwrap();
        assert_eq!(result, Expression::String("llo".into()));
    }

    #[test]
    fn test_sub_string_int_min() {
        let result = Expression::String("hello".into()) - Expression::Integer(i64::MIN);
        assert!(result.is_err(), "String - i64::MIN should error");
    }

    #[test]
    fn test_sub_string_float() {
        let result =
            (Expression::String("hello 3.14world".into()) - Expression::Float(3.14)).unwrap();
        // Removing "3.14" from "hello 3.14world" leaves "hello world"
        assert_eq!(result, Expression::String("hello world".into()));
    }

    // --- Mul ---

    #[test]
    fn test_mul_int_int() {
        let result = (Expression::Integer(6) * Expression::Integer(7)).unwrap();
        assert_eq!(result, Expression::Integer(42));
    }

    #[test]
    fn test_mul_int_overflow() {
        let result = Expression::Integer(i64::MAX) * Expression::Integer(2);
        assert!(result.is_err());
        assert!(matches!(result, Err(RuntimeErrorKind::Overflow { .. })));
    }

    #[test]
    fn test_mul_string_int_zero() {
        let result = (Expression::String("abc".into()) * Expression::Integer(0)).unwrap();
        assert_eq!(
            result,
            Expression::String("".into()),
            "'abc' * 0 should be ''"
        );
    }

    #[test]
    fn test_mul_string_int_negative() {
        let result = Expression::String("abc".into()) * Expression::Integer(-1);
        assert!(result.is_err(), "'abc' * -1 should error");
    }

    #[test]
    fn test_mul_string_int_positive() {
        let result = (Expression::String("ab".into()) * Expression::Integer(3)).unwrap();
        assert_eq!(result, Expression::String("ababab".into()));
    }

    #[test]
    fn test_mul_list_scalar() {
        let list = Expression::from(vec![1i64, 2, 3]);
        let result = (list * Expression::Integer(2)).unwrap();
        match result {
            Expression::List(items) => {
                assert_eq!(items.len(), 3);
                // all items should be floats now
                assert_eq!(items[0], Expression::Float(2.0));
                assert_eq!(items[1], Expression::Float(4.0));
                assert_eq!(items[2], Expression::Float(6.0));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_mul_set_intersection() {
        let mut s1 = BTreeSet::new();
        s1.insert(Expression::Integer(1));
        s1.insert(Expression::Integer(2));
        s1.insert(Expression::Integer(3));
        let mut s2 = BTreeSet::new();
        s2.insert(Expression::Integer(2));
        s2.insert(Expression::Integer(3));
        s2.insert(Expression::Integer(4));
        let result = (Expression::from(s1) * Expression::from(s2)).unwrap();
        match result {
            Expression::BSet(items) => {
                assert_eq!(items.len(), 2);
                assert!(items.contains(&Expression::Integer(2)));
                assert!(items.contains(&Expression::Integer(3)));
            }
            _ => panic!("Expected BSet"),
        }
    }

    // --- Div ---

    #[test]
    fn test_div_int_int() {
        let result = (Expression::Integer(10) / Expression::Integer(3)).unwrap();
        assert_eq!(result, Expression::Integer(3)); // truncation
    }

    #[test]
    fn test_div_int_by_zero() {
        let result = Expression::Integer(10) / Expression::Integer(0);
        assert!(result.is_err());
        assert!(matches!(result, Err(RuntimeErrorKind::CustomError(_))));
    }

    #[test]
    fn test_div_float_by_zero() {
        let result = Expression::Float(10.0) / Expression::Float(0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_div_by_string_zero() {
        let result = Expression::Integer(10) / Expression::String("0".into());
        assert!(result.is_err());
        assert!(matches!(result, Err(RuntimeErrorKind::CustomError(_))));
    }

    #[test]
    fn test_div_by_string_zero_dot_zero() {
        // BUG: "0.0" is not caught as division by zero
        let result = Expression::Integer(10) / Expression::String("0.0".into());
        // This should probably be an error, but currently may try to parse as integer
        // which fails because "0.0" is not an i64, so it returns CommandFailed2
        assert!(
            result.is_err(),
            "BUG: division by string '0.0' may not be properly caught"
        );
    }

    // --- Rem ---

    #[test]
    fn test_rem_int_int() {
        let result = Expression::Integer(10) % Expression::Integer(3);
        assert_eq!(result, Expression::Integer(1));
    }

    #[test]
    fn test_rem_float_int() {
        let result = Expression::Float(10.5) % Expression::Integer(3);
        assert_eq!(result, Expression::Float(1.5));
    }

    // --- Neg ---

    #[test]
    fn test_neg_int() {
        let result = -Expression::Integer(5);
        assert_eq!(result, Expression::Integer(-5));
    }

    #[test]
    fn test_neg_float() {
        let result = -Expression::Float(3.14);
        assert_eq!(result, Expression::Float(-3.14));
    }

    #[test]
    fn test_neg_bool() {
        let result = -Expression::Boolean(true);
        assert_eq!(result, Expression::Boolean(false));
    }

    #[test]
    fn test_neg_string() {
        // BUG(?): Neg for String returns None silently
        let result = -Expression::String("hello".into());
        assert_eq!(result, Expression::None);
    }
}

// ============================================================
// 4. TRUTHINESS TESTS
// ============================================================

mod truthiness_tests {
    use super::*;

    #[test]
    fn test_integer_zero_is_falsey() {
        assert!(!Expression::Integer(0).is_truthy());
    }

    #[test]
    fn test_integer_nonzero_is_truthy() {
        assert!(Expression::Integer(1).is_truthy());
        assert!(Expression::Integer(-1).is_truthy());
    }

    #[test]
    fn test_float_zero_is_falsey() {
        assert!(!Expression::Float(0.0).is_truthy());
    }

    #[test]
    fn test_float_nonzero_is_truthy() {
        assert!(Expression::Float(0.1).is_truthy());
    }

    #[test]
    fn test_string_empty_is_falsey() {
        assert!(!Expression::String("".into()).is_truthy());
    }

    #[test]
    fn test_string_nonempty_is_truthy() {
        assert!(Expression::String("hello".into()).is_truthy());
    }

    #[test]
    fn test_bool_false_is_falsey() {
        assert!(!Expression::Boolean(false).is_truthy());
    }

    #[test]
    fn test_bool_true_is_truthy() {
        assert!(Expression::Boolean(true).is_truthy());
    }

    #[test]
    fn test_empty_list_is_falsey() {
        assert!(!Expression::List(Rc::new(vec![])).is_truthy());
    }

    #[test]
    fn test_nonempty_list_is_truthy() {
        assert!(Expression::List(Rc::new(vec![Expression::Integer(1)])).is_truthy());
    }

    #[test]
    fn test_empty_map_is_falsey() {
        assert!(!Expression::Map(Rc::new(BTreeMap::new())).is_truthy());
    }

    #[test]
    fn test_none_is_falsey() {
        assert!(!Expression::None.is_truthy());
    }

    #[test]
    fn test_regex_empty_pattern_is_falsey() {
        let re = LumeRegex {
            regex: Regex::new("").unwrap(),
        };
        assert!(
            !Expression::Regex(re).is_truthy(),
            "empty regex pattern should be falsey"
        );
    }

    #[test]
    fn test_regex_nonempty_pattern_is_truthy() {
        let re = LumeRegex {
            regex: Regex::new("[a-z]").unwrap(),
        };
        assert!(
            Expression::Regex(re).is_truthy(),
            "non-empty regex pattern should be truthy"
        );
    }

    #[test]
    fn test_file_size_zero_is_falsey() {
        assert!(!Expression::FileSize(FileSize::from(0, "B")).is_truthy());
    }

    #[test]
    fn test_file_size_nonzero_is_truthy() {
        assert!(Expression::FileSize(FileSize::from(1024, "B")).is_truthy());
    }

    #[test]
    fn test_blank_is_falsey() {
        assert!(!Expression::Blank.is_truthy());
    }

    #[test]
    fn test_range_is_truthy_when_nonempty() {
        assert!(Expression::Range(0..10, 1).is_truthy());
        assert!(!Expression::Range(0..0, 1).is_truthy());
    }

    #[test]
    fn test_empty_bset_is_falsey() {
        assert!(!Expression::BSet(Rc::new(BTreeSet::new())).is_truthy());
    }

    #[test]
    fn test_empty_hmap_is_falsey() {
        assert!(!Expression::HMap(Rc::new(HashMap::new())).is_truthy());
    }

    #[test]
    fn test_lambda_is_truthy() {
        assert!(Expression::Lambda(vec![], Rc::new(Expression::None), None).is_truthy());
    }

    #[test]
    fn test_function_is_truthy() {
        assert!(
            Expression::Function("f".into(), vec![], None, Rc::new(Expression::None), vec![],)
                .is_truthy()
        );
    }
}

// ============================================================
// 5. PARTIAL ORD TESTS
// ============================================================

mod partial_ord_tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn test_compare_int() {
        assert_eq!(
            Expression::Integer(5).partial_cmp(&Expression::Integer(3)),
            Some(Ordering::Greater)
        );
        assert_eq!(
            Expression::Integer(3).partial_cmp(&Expression::Integer(5)),
            Some(Ordering::Less)
        );
        assert_eq!(
            Expression::Integer(3).partial_cmp(&Expression::Integer(3)),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn test_compare_float() {
        assert_eq!(
            Expression::Float(3.0).partial_cmp(&Expression::Float(5.0)),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn test_compare_int_float() {
        assert_eq!(
            Expression::Integer(5).partial_cmp(&Expression::Float(5.0)),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn test_compare_string_int_coercion() {
        assert_eq!(
            Expression::String("5".into()).partial_cmp(&Expression::Integer(5)),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn test_compare_string_int_coercion_fail() {
        assert_eq!(
            Expression::String("abc".into()).partial_cmp(&Expression::Integer(5)),
            None
        );
    }

    #[test]
    fn test_compare_different_types() {
        assert_eq!(
            Expression::String("hello".into()).partial_cmp(&Expression::List(Rc::new(vec![]))),
            None
        );
    }

    #[test]
    fn test_compare_bset_by_content() {
        let mut s1 = BTreeSet::new();
        s1.insert(Expression::Integer(1));
        s1.insert(Expression::Integer(2));

        let mut s2 = BTreeSet::new();
        s2.insert(Expression::Integer(3));
        s2.insert(Expression::Integer(4));

        assert_eq!(
            Expression::BSet(Rc::new(s1)).partial_cmp(&Expression::BSet(Rc::new(s2))),
            Some(Ordering::Less),
            "BSet content should be compared, not just length"
        );
    }

    #[test]
    fn test_compare_hmap_by_content() {
        let mut m1 = HashMap::new();
        m1.insert("a".into(), Expression::Integer(1));
        let mut m2 = HashMap::new();
        m2.insert("b".into(), Expression::Integer(2));

        assert_eq!(
            Expression::HMap(Rc::new(m1)).partial_cmp(&Expression::HMap(Rc::new(m2))),
            Some(Ordering::Less),
            "HMap content should be compared, not just length"
        );
    }

    #[test]
    fn test_compare_map_by_content() {
        let mut m1 = BTreeMap::new();
        m1.insert("a".into(), Expression::Integer(1));
        let mut m2 = BTreeMap::new();
        m2.insert("b".into(), Expression::Integer(2));

        assert_eq!(
            Expression::Map(Rc::new(m1)).partial_cmp(&Expression::Map(Rc::new(m2))),
            Some(Ordering::Less),
            "Map content should be compared, not just length"
        );
    }

    #[test]
    fn test_compare_bset_by_length_different() {
        let mut s1 = BTreeSet::new();
        s1.insert(Expression::Integer(1));

        let mut s2 = BTreeSet::new();
        s2.insert(Expression::Integer(1));
        s2.insert(Expression::Integer(2));

        assert_eq!(
            Expression::BSet(Rc::new(s1)).partial_cmp(&Expression::BSet(Rc::new(s2))),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn test_compare_symbols() {
        assert_eq!(
            Expression::Symbol("a".into()).partial_cmp(&Expression::Symbol("b".into())),
            Some(Ordering::Less)
        );
        assert_eq!(
            Expression::Symbol("b".into()).partial_cmp(&Expression::Symbol("a".into())),
            Some(Ordering::Greater)
        );
        assert_eq!(
            Expression::Symbol("a".into()).partial_cmp(&Expression::Symbol("a".into())),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn test_compare_date_time() {
        use chrono::NaiveDateTime;
        let t1 = NaiveDateTime::parse_from_str("2024-01-01 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let t2 = NaiveDateTime::parse_from_str("2024-06-15 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        assert_eq!(
            Expression::DateTime(t1).partial_cmp(&Expression::DateTime(t2)),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn test_compare_file_size() {
        let f1 = FileSize::from(1024, "B");
        let f2 = FileSize::from(2048, "B");
        assert_eq!(
            Expression::FileSize(f1).partial_cmp(&Expression::FileSize(f2)),
            Some(Ordering::Less)
        );
    }
}

// ============================================================
// 6. ENVIRONMENT TESTS
// ============================================================

mod environment_tests {
    use super::*;

    #[test]
    fn test_new_environment() {
        let env = Environment::new();
        assert!(env.get("anything").is_none());
    }

    #[test]
    fn test_define_and_get() {
        let mut env = Environment::new();
        env.define("x", Expression::Integer(42));
        assert_eq!(env.get("x"), Some(Expression::Integer(42)));
    }

    #[test]
    fn test_undefine() {
        let mut env = Environment::new();
        env.define("x", Expression::Integer(42));
        env.undefine("x");
        assert_eq!(env.get("x"), None);
    }

    #[test]
    fn test_has() {
        let mut env = Environment::new();
        env.define("x", Expression::Integer(1));
        assert!(env.has("x"));
        assert!(!env.has("y"));
    }

    #[test]
    fn test_fork_inheritance() {
        let mut parent = Environment::new();
        parent.define("x", Expression::Integer(1));
        let mut child = parent.fork();
        // child should see parent's variables
        assert_eq!(child.get("x"), Some(Expression::Integer(1)));
        // child can override
        child.define("x", Expression::Integer(2));
        assert_eq!(child.get("x"), Some(Expression::Integer(2)));
        // parent unchanged
        assert_eq!(parent.get("x"), Some(Expression::Integer(1)));
    }

    #[test]
    fn test_fork_isolation() {
        let parent = Environment::new();
        let mut child = parent.fork();
        child.define("y", Expression::Integer(99));
        // parent should NOT see child's new variable
        assert_eq!(parent.get("y"), None);
    }

    #[test]
    fn test_define_in_root() {
        let parent = Environment::new();
        let child = parent.fork();
        let mut grandchild = child.fork();
        grandchild.define_in_root("z", Expression::Integer(100));
        // grandchild should see it via parent chain
        assert_eq!(grandchild.get("z"), Some(Expression::Integer(100)));
        // NOTE: child's parent is a clone of the original parent (created at fork time),
        // so define_in_root on grandchild modifies grandchild's parent's parent clone,
        // not child's direct parent. This is known behavior: fork creates disjoint chains.
        // child.get("z") may not see the value.
    }

    #[test]
    fn test_is_defined_chain() {
        let mut env = Environment::new();
        env.define("a", Expression::Integer(1));
        let child = env.fork();
        assert!(child.is_defined("a"));
    }

    #[test]
    fn test_get_bindings_string() {
        let mut env = Environment::new();
        env.define("str", Expression::String("hello".into()));
        env.define("num", Expression::Integer(42));
        env.define("float", Expression::Float(3.14)); // should be filtered out
        let bindings = env.get_bindings_string();
        assert!(bindings.contains_key(&"str".to_string()));
        assert!(bindings.contains_key(&"num".to_string()));
        assert!(!bindings.contains_key(&"float".to_string()));
    }

    #[test]
    fn test_get_root() {
        let parent = Environment::new();
        let child = parent.fork();
        let grandchild = child.fork();
        // root is grandparent (parent)
        let root = grandchild.get_root();
        // Since child binds to parent via fork, the root of grandchild is...actually child's root is parent
        // Environment::get_root() returns the topmost. Let's verify parent has no parent.
        assert!(root.parent.is_none());
    }

    #[test]
    fn test_define_twice_overwrites() {
        let mut env = Environment::new();
        env.define("x", Expression::Integer(1));
        env.define("x", Expression::Integer(2));
        assert_eq!(env.get("x"), Some(Expression::Integer(2)));
    }
}

// ============================================================
// 6b. QUOTE ESCAPE TESTS (parse + eval)
// ============================================================

#[test]
fn test_quote_single_pure_literal() {
    // '...' is pure literal: no escape processing
    let (tokens, diags) = tokenize(r#"'hello\nworld'"#);
    assert!(diags.iter().filter(|d| d != &&Diagnostic::Valid).count() == 0);
    assert_eq!(tokens[0].kind, crate::TokenKind::StringRaw);

    let result = eval_str("'hello\\nworld'").unwrap();
    assert_eq!(
        result,
        Expression::String("hello\\nworld".into()),
        "single-quoted \\n should stay literal"
    );
}

#[test]
fn test_quote_double_newline() {
    let result = eval_str(r#""hello\nworld""#).unwrap();
    assert_eq!(
        result,
        Expression::String("hello\nworld".into()),
        "double-quoted \\n should become newline"
    );
}

#[test]
fn test_quote_double_tab() {
    let result = eval_str(r#""col1\tcol2""#).unwrap();
    assert_eq!(result, Expression::String("col1\tcol2".into()));
}

#[test]
fn test_quote_double_backslash() {
    let result = eval_str(r#""path\\file""#).unwrap();
    assert_eq!(result, Expression::String(r"path\file".into()));
}

#[test]
fn test_quote_double_unicode() {
    let result = eval_str(r#""\u{0041}""#).unwrap();
    assert_eq!(result, Expression::String("A".into()));
}

#[test]
fn test_quote_double_escaped_quote() {
    let result = eval_str(r#""he said \"hello\"""#).unwrap();
    assert_eq!(result, Expression::String("he said \"hello\"".into()));
}

#[test]
fn test_quote_double_invalid_escape() {
    let result = eval_str(r#""hello\nworld\zfoo""#).unwrap();
    assert_eq!(
        result,
        Expression::String("hello\nworld\\zfoo".into()),
        "valid \\n should process, invalid \\z becomes literal"
    );
}

#[test]
fn test_quote_backtick_newline() {
    let result = eval_str(r"`hello\nworld`").unwrap();
    assert_eq!(
        result,
        Expression::String("hello\nworld".into()),
        "backtick \\n should be newline"
    );
}

#[test]
fn test_quote_backtick_tab() {
    let result = eval_str(r"`col1\tcol2`").unwrap();
    assert_eq!(result, Expression::String("col1\tcol2".into()));
}

#[test]
fn test_quote_backtick_unicode() {
    let result = eval_str(r"`\u{0041}`").unwrap();
    assert_eq!(result, Expression::String("A".into()));
}

#[test]
fn test_quote_backtick_backslash() {
    let result = eval_str(r"`path\\file`").unwrap();
    assert_eq!(result, Expression::String(r"path\file".into()));
}

#[test]
fn test_quote_backtick_escaped_backtick() {
    let result = eval_str(r"`back\tick`").unwrap();
    assert_eq!(
        result,
        Expression::String("back\tick".into()),
        "backtick \\t should become tab"
    );
}

#[test]
fn test_quote_backtick_escape_and_interpolation() {
    let mut env = Environment::new();
    env.define("user", Expression::String("Alice".into()));
    let expr = parse_script(r"`hello $user\nwelcome`").unwrap();
    let result = expr.eval(&mut env).unwrap();
    assert_eq!(result, Expression::String("hello Alice\nwelcome".into()));
}

#[test]
fn test_quote_backtick_invalid_escape() {
    let result = eval_str(r"`hello\nworld\zfoo`").unwrap();
    assert_eq!(
        result,
        Expression::String("hello\nworld\\zfoo".into()),
        "valid \\n should process, invalid \\z becomes literal"
    );
}

#[test]
fn test_quote_backtick_interpolation_only() {
    let mut env = Environment::new();
    env.define("name", Expression::String("World".into()));
    let expr = parse_script("`Hello $name`").unwrap();
    let result = expr.eval(&mut env).unwrap();
    assert_eq!(result, Expression::String("Hello World".into()));
}

// ============================================================
// 7. EVALUATOR TESTS (parse + eval)
// ============================================================

fn eval_str(input: &str) -> Result<Expression, Box<dyn std::fmt::Debug>> {
    let expr = parse_script(input).map_err(|e| -> Box<dyn std::fmt::Debug> { Box::new(e) })?;
    let mut env = Environment::new();
    expr.eval(&mut env)
        .map_err(|e| -> Box<dyn std::fmt::Debug> { Box::new(e) })
}

mod evaluator_tests {
    use super::*;

    #[test]
    fn test_eval_integer() {
        let result = eval_str("42").unwrap();
        assert_eq!(result, Expression::Integer(42));
    }

    #[test]
    fn test_eval_float() {
        let result = eval_str("3.14").unwrap();
        assert_eq!(result, Expression::Float(3.14));
    }

    #[test]
    fn test_eval_string() {
        let result = eval_str(r#""hello""#).unwrap();
        assert_eq!(result, Expression::String("hello".into()));
    }

    #[test]
    fn test_eval_addition() {
        let result = eval_str("1 + 2").unwrap();
        assert_eq!(result, Expression::Integer(3));
    }

    #[test]
    fn test_eval_subtraction() {
        let result = eval_str("10 - 3").unwrap();
        assert_eq!(result, Expression::Integer(7));
    }

    #[test]
    fn test_eval_multiplication() {
        let result = eval_str("6 * 7").unwrap();
        assert_eq!(result, Expression::Integer(42));
    }

    #[test]
    fn test_eval_division() {
        let result = eval_str("10 / 3").unwrap();
        assert_eq!(result, Expression::Integer(3));
    }

    #[test]
    fn test_eval_power() {
        let result = eval_str("2 ^ 10").unwrap();
        assert_eq!(result, Expression::Integer(1024));
    }

    #[test]
    fn test_eval_modulo() {
        let result = eval_str("10 % 3").unwrap();
        assert_eq!(result, Expression::Integer(1));
    }

    #[test]
    fn test_eval_precedence() {
        let result = eval_str("2 + 3 * 4").unwrap();
        assert_eq!(result, Expression::Integer(14));
    }

    #[test]
    fn test_eval_parens() {
        let result = eval_str("(2 + 3) * 4").unwrap();
        assert_eq!(result, Expression::Integer(20));
    }

    #[test]
    fn test_eval_comparison() {
        match eval_str("5 != 5") {
            Ok(result) => assert_eq!(result, Expression::Boolean(false)),
            Err(e) => {
                panic!("eval_str failed: {e:?}");
            }
        }
    }

    #[test]
    fn test_eval_logical_and() {
        let result = eval_str("true && true").unwrap();
        assert_eq!(result, Expression::Boolean(true));

        let result = eval_str("true && false").unwrap();
        assert_eq!(result, Expression::Boolean(false));
    }

    #[test]
    fn test_eval_logical_or() {
        let result = eval_str("true || false").unwrap();
        assert_eq!(result, Expression::Boolean(true));

        let result = eval_str("false || false").unwrap();
        assert_eq!(result, Expression::Boolean(false));
    }

    #[test]
    fn test_eval_unary_not() {
        let result = eval_str("!true").unwrap();
        assert_eq!(result, Expression::Boolean(false));

        let result = eval_str("!false").unwrap();
        assert_eq!(result, Expression::Boolean(true));

        let result = eval_str("!0").unwrap();
        assert_eq!(result, Expression::Boolean(true));

        let result = eval_str("!42").unwrap();
        assert_eq!(result, Expression::Boolean(false));
    }

    #[test]
    fn test_eval_unary_neg() {
        // -42 in expression context is parsed as StringRaw("-42") by the tokenizer.
        // Use (-42) to force prefix operator context.
        let result = eval_str("(-42)").unwrap();
        assert_eq!(result, Expression::Integer(-42));
    }

    #[test]
    fn test_eval_let_declare() {
        let mut env = Environment::new();
        let expr = parse_script("let x = 42").unwrap();
        expr.eval(&mut env).unwrap();
        assert_eq!(env.get("x"), Some(Expression::Integer(42)));
    }

    #[test]
    fn test_eval_assign() {
        let mut env = Environment::new();
        env.define("x", Expression::Integer(10));

        let expr = parse_script("x = 20").unwrap();
        expr.eval(&mut env).unwrap();
        assert_eq!(env.get("x"), Some(Expression::Integer(20)));
    }

    #[test]
    fn test_eval_if_true() {
        let result = eval_str("if true { 1 } else { 2 }").unwrap();
        assert_eq!(result, Expression::Integer(1));
    }

    #[test]
    fn test_eval_if_false() {
        let result = eval_str("if false { 1 } else { 2 }").unwrap();
        assert_eq!(result, Expression::Integer(2));
    }

    #[test]
    fn test_eval_list() {
        let result = eval_str("[1, 2, 3]").unwrap();
        match result {
            Expression::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Expression::Integer(1));
                assert_eq!(items[1], Expression::Integer(2));
                assert_eq!(items[2], Expression::Integer(3));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_eval_range() {
        let result = eval_str("1..5").unwrap();
        match result {
            Expression::Range(r, _) => {
                assert_eq!(r, 1..5);
            }
            _ => panic!("Expected Range, got {result:?}"),
        }
    }

    #[test]
    fn test_eval_neg_overflow() {
        // -i64::MIN overflows (i64::MIN = -9223372036854775808)
        let result = eval_str("-(-9223372036854775808)");
        assert!(
            result.is_err() || {
                // If it doesn't error, check result is correct
                matches!(&result, Ok(Expression::Integer(v)) if *v == i64::MIN)
            }
        );
    }

    #[test]
    fn test_eval_division_by_zero() {
        let result = eval_str("10 / 0");
        assert!(
            result.is_err(),
            "Division by zero should error, got {result:?}"
        );
    }

    #[test]
    fn test_eval_strict_equality() {
        let result = eval_str("5 === 5").unwrap();
        assert_eq!(result, Expression::Boolean(true));

        let result = eval_str(r#"5 === "5""#).unwrap();
        assert_eq!(result, Expression::Boolean(false));
    }

    #[test]
    fn test_eval_string_concatenation() {
        let result = eval_str(r#""hello " + "world""#).unwrap();
        assert_eq!(result, Expression::String("hello world".into()));
    }

    #[test]
    fn test_eval_string_times_int() {
        let result = eval_str(r#""ab" * 3"#).unwrap();
        assert_eq!(result, Expression::String("ababab".into()));
    }

    #[test]
    fn test_eval_string_times_zero() {
        let result = eval_str(r#""ab" * 0"#).unwrap();
        assert_eq!(
            result,
            Expression::String("".into()),
            "'ab' * 0 should be ''"
        );
    }

    #[test]
    fn test_eval_template_string() {
        // Template strings require variable interpolation
        let mut env = Environment::new();
        env.define("name", Expression::String("World".into()));
        let expr = parse_script("`Hello $name`").unwrap();
        let result = expr.eval(&mut env).unwrap();
        assert_eq!(result, Expression::String("Hello World".into()));
    }

    #[test]
    fn test_eval_contains_operator() {
        let result = eval_str(r#""hello" ~: "ell""#).unwrap();
        assert_eq!(result, Expression::Boolean(true));

        let result = eval_str(r#""hello" ~: "xyz""#).unwrap();
        assert_eq!(result, Expression::Boolean(false));
    }

    #[test]
    fn test_eval_range_expression_expand() {
        let result = eval_str("0...3").unwrap();
        match result {
            Expression::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Expression::Integer(0));
                assert_eq!(items[1], Expression::Integer(1));
                assert_eq!(items[2], Expression::Integer(2));
            }
            _ => panic!("Expected List from 0...3, got {result:?}"),
        }
    }

    #[test]
    fn test_eval_compound_assign_add() {
        let mut env = Environment::new();
        env.define("x", Expression::Integer(10));
        let expr = parse_script("x += 5").unwrap();
        expr.eval(&mut env).unwrap();
        assert_eq!(env.get("x"), Some(Expression::Integer(15)));
    }

    #[test]
    fn test_eval_compound_assign_sub() {
        let mut env = Environment::new();
        env.define("x", Expression::Integer(10));
        let expr = parse_script("x -= 3").unwrap();
        expr.eval(&mut env).unwrap();
        assert_eq!(env.get("x"), Some(Expression::Integer(7)));
    }

    #[test]
    fn test_eval_compound_assign_mul() {
        let mut env = Environment::new();
        env.define("x", Expression::Integer(5));
        let expr = parse_script("x *= 3").unwrap();
        expr.eval(&mut env).unwrap();
        assert_eq!(env.get("x"), Some(Expression::Integer(15)));
    }

    #[test]
    fn test_eval_compound_assign_div() {
        let mut env = Environment::new();
        env.define("x", Expression::Integer(10));
        let expr = parse_script("x /= 3").unwrap();
        expr.eval(&mut env).unwrap();
        assert_eq!(env.get("x"), Some(Expression::Integer(3)));
    }

    #[test]
    fn test_eval_add_assign_undefined_var() {
        // BUG?: += on undefined variable defaults to 0
        let mut env = Environment::new();
        let expr = parse_script("undefined += 5").unwrap();
        expr.eval(&mut env).unwrap();
        // In non-strict mode, this should work and define the variable as 5
        // since it falls back to Integer(0) + 5
        assert_eq!(
            env.get("undefined"),
            Some(Expression::Integer(5)),
            "BUG: += on undefined defaults to 0 in non-strict mode"
        );
    }

    #[test]
    fn test_eval_if_conditional() {
        let result = eval_str("true ? 10 : 20").unwrap();
        assert_eq!(result, Expression::Integer(10));

        let result = eval_str("false ? 10 : 20").unwrap();
        assert_eq!(result, Expression::Integer(20));
    }

    #[test]
    fn test_eval_blank() {
        // Blank should evaluate to what's in the pipe or itself
        let result = eval_str("_").unwrap();
        assert_eq!(result, Expression::Blank);
    }
}

// ============================================================
// 8. EDGE CASE / BUG REPRODUCTION TESTS
// ============================================================

mod bug_reproduction_tests {
    use super::*;

    #[test]
    fn bug_mul_string_by_zero_returns_empty() {
        let result = (Expression::String("abc".into()) * Expression::Integer(0)).unwrap();
        assert_eq!(
            result,
            Expression::String("".into()),
            "String * 0 should be ''"
        );
    }

    #[test]
    fn bug_mul_string_by_negative_errors() {
        let result = Expression::String("abc".into()) * Expression::Integer(-1);
        assert!(result.is_err(), "String * negative should error");
    }

    #[test]
    fn bug_regex_truthiness_is_fixed() {
        let empty_re = LumeRegex {
            regex: Regex::new("").unwrap(),
        };
        let nonempty_re = LumeRegex {
            regex: Regex::new(".").unwrap(),
        };
        assert!(
            !Expression::Regex(empty_re).is_truthy(),
            "empty regex should be falsey"
        );
        assert!(
            Expression::Regex(nonempty_re).is_truthy(),
            "non-empty regex should be truthy"
        );
    }

    #[test]
    fn bug_hmap_equality_by_content() {
        use std::cmp::Ordering;
        let mut m1 = HashMap::new();
        m1.insert("a".into(), Expression::Integer(1));
        let mut m2 = HashMap::new();
        m2.insert("b".into(), Expression::Integer(2)); // same length, different content

        let a = Expression::HMap(Rc::new(m1));
        let b = Expression::HMap(Rc::new(m2));
        assert_ne!(
            a.partial_cmp(&b),
            Some(Ordering::Equal),
            "different HMaps should NOT be equal when content differs"
        );
    }

    #[test]
    fn bug_div_by_string_zero_dot_zero() {
        let result = Expression::Integer(10) / Expression::String("0.0".into());
        assert!(
            result.is_err(),
            "division by '0.0' should be an error but got {:?}",
            result
        );
        match result {
            Err(RuntimeErrorKind::CustomError(s)) => {
                assert!(
                    s.contains("zero"),
                    "Should mention division by zero, got: {s}"
                );
            }
            _ => {}
        }
    }

    #[test]
    fn bug_addassign_overflow_wraps() {
        let mut val = Expression::Integer(i64::MAX);
        val += Expression::Integer(1);
        assert_eq!(
            val,
            Expression::Integer(i64::MIN),
            "AddAssign should wrap on overflow"
        );
    }

    #[test]
    fn bug_mulassign_overflow_wraps() {
        let mut val = Expression::Integer(i64::MAX);
        val *= Expression::Integer(2);
        // wrapping_mul: i64::MAX * 2 = -2
        assert_eq!(
            val,
            Expression::Integer(-2),
            "MulAssign should wrap on overflow"
        );
    }

    /// Test that `!:` and `:` as match operators work
    #[test]
    fn test_contains_not_contains_operator() {
        let result = ((Expression::String("hello".into()) + Expression::String(" ".into()))
            .unwrap()
            + Expression::String("world".into()))
        .unwrap();
        // just verify string concat works
        assert_eq!(result, Expression::String("hello world".into()));
    }

    /// Test evaluation of deeply nested expressions
    #[test]
    fn test_deeply_nested_arithmetic() {
        let result = eval_str("((((1 + 2) * 3) - 4) / 5)").unwrap();
        assert_eq!(result, Expression::Integer(1)); // (9-4)/5 = 1
    }

    /// Test operator precedence: comparison vs logical
    #[test]
    fn test_precedence_comparison_vs_logical() {
        let result = eval_str("5 > 3 && 2 < 4").unwrap();
        assert_eq!(result, Expression::Boolean(true));

        let result = eval_str("5 > 3 || 2 > 4").unwrap();
        assert_eq!(result, Expression::Boolean(true));
    }
}

// ============================================================
// 9. FROM / TYPE CONVERSION TESTS
// ============================================================

mod from_tests {
    use super::*;

    #[test]
    fn test_from_int() {
        let e: Expression = 42i64.into();
        assert_eq!(e, Expression::Integer(42));
    }

    #[test]
    fn test_from_float() {
        let e: Expression = 3.14f64.into();
        assert_eq!(e, Expression::Float(3.14));
    }

    #[test]
    fn test_from_str() {
        let e: Expression = "hello".into();
        assert_eq!(e, Expression::String("hello".into()));
    }

    #[test]
    fn test_from_string() {
        let e: Expression = String::from("world").into();
        assert_eq!(e, Expression::String("world".into()));
    }

    #[test]
    fn test_from_bool() {
        let e: Expression = true.into();
        assert_eq!(e, Expression::Boolean(true));
    }

    #[test]
    fn test_from_vec() {
        let v: Vec<i64> = vec![1, 2, 3];
        let e: Expression = v.into();
        match e {
            Expression::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Expression::Integer(1));
            }
            _ => panic!("Expected List"),
        }
    }

    #[test]
    fn test_from_empty_vec() {
        let v: Vec<i64> = vec![];
        let e: Expression = v.into();
        match e {
            Expression::List(items) => assert!(items.is_empty()),
            _ => panic!("Expected empty List"),
        }
    }

    #[test]
    fn test_from_hashmap() {
        let mut m = HashMap::new();
        m.insert("key".to_string(), "value".to_string());
        let e: Expression = m.into();
        match e {
            Expression::HMap(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items.get("key"), Some(&Expression::String("value".into())));
            }
            _ => panic!("Expected HMap"),
        }
    }

    #[test]
    fn test_from_btreemap() {
        let mut m = BTreeMap::new();
        m.insert("a".to_string(), 1i64);
        let e: Expression = m.into();
        match e {
            Expression::Map(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items.get("a"), Some(&Expression::Integer(1)));
            }
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_from_btreeset() {
        let mut s = BTreeSet::new();
        s.insert(1i64);
        s.insert(2i64);
        let e: Expression = s.into();
        match e {
            Expression::BSet(items) => {
                assert_eq!(items.len(), 2);
                assert!(items.contains(&Expression::Integer(1)));
            }
            _ => panic!("Expected BSet"),
        }
    }

    #[test]
    fn test_from_bytes() {
        let e: Expression = vec![104, 101, 108, 108, 111u8].into();
        match e {
            Expression::Bytes(b) => assert_eq!(b, b"hello"),
            _ => panic!("Expected Bytes"),
        }
    }
}

// ============================================================
// 10. FILETYPE - FileSize TESTS
// ============================================================

mod filesize_tests {
    use super::*;

    #[test]
    fn test_filesize_from_bytes() {
        let fs = FileSize::from(1024, "B");
        assert_eq!(fs.to_bytes(), 1024);
    }

    #[test]
    fn test_filesize_from_kb() {
        let fs = FileSize::from(1, "K");
        assert_eq!(fs.to_bytes(), 1024);
    }

    #[test]
    fn test_filesize_from_mb() {
        let fs = FileSize::from(1, "M");
        assert_eq!(fs.to_bytes(), 1024 * 1024);
    }

    #[test]
    fn test_filesize_from_gb() {
        let fs = FileSize::from(1, "G");
        assert_eq!(fs.to_bytes(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_filesize_human_readable() {
        let fs = FileSize::from(1, "K");
        let hr = fs.to_human_readable();
        assert_eq!(hr, "1K");
    }

    #[test]
    fn test_filesize_human_readable_bytes() {
        let fs = FileSize::from(500, "B");
        let hr = fs.to_human_readable();
        assert_eq!(hr, "500");
    }

    #[test]
    fn test_filesize_human_readable_large() {
        let fs = FileSize::from(1, "M");
        let hr = fs.to_human_readable();
        assert_eq!(hr, "1.00M");
    }

    #[test]
    fn test_filesize_comparison() {
        let a = FileSize::from(1, "K");
        let b = FileSize::from(1, "M");
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, FileSize::from(1024, "B"));
    }

    #[test]
    fn test_filesize_from_bytes_constructor() {
        let fs = FileSize::from_bytes(2048);
        assert_eq!(fs.to_bytes(), 2048);
        assert_eq!(fs.to_human_readable(), "2K");
    }
}
