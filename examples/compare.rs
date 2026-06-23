use lumesh::tokenize;

fn main() {
    // Key inputs where we expect differences between old and new
    let inputs = vec![
        // 1. Comment rule
        ("#foo", "comment no-space prefix"),
        ("#!bash", "shebang comment"),
        // 2. Dot handling
        ("a.b", "single dot infix"),
        ("a..b", "double dot infix"),
        ("a...b", "triple dot infix"),
        (".5", "decimal prefix"),
        ("./path", "path argument"),
        ("10..", "trailing dots after number"),
        ("..", "bare double dot"),
        ("...", "bare triple dot"),
        ("..=", "range-equal"),
        // 3. Minus handling
        ("-42", "negative number at start"),
        ("x-1", "minus in symbol"),
        ("--flag", "double dash arg"),
        ("a - b", "minus with spaces"),
        ("-x", "minus prefix"),
        ("a-b", "minus infix in symbol"),
        ("x--y", "double minus"),
        ("a - -b", "space minus space minus"),
        // 4. Bang handling
        ("foo!", "postfix bang"),
        ("!x", "prefix bang"),
        ("a!=b", "not-equal"),
        ("a!==b", "not-not-equal"),
        ("a!~:b", "bang-tilde-colon"),
        ("x!!!", "triple bang"),
        ("!!x", "double bang prefix"),
        ("a! b!", "bang with space"),
        ("x!y", "bang infix no-space"),
        // 5. Parentheses
        ("arr[0]", "array index"),
        ("func(arg)", "function call"),
        ("(a + b)", "grouping parens"),
        ("x[y", "open bracket no close"),
        ("f()(g)", "chained calls"),
        // 6. @ symbol
        ("a@0", "at symbol"),
        ("@decorator", "at prefix"),
        ("x@@y", "double at"),
        ("@@x", "double at prefix"),
        // 7. :: module
        ("mod::func", "module call"),
        ("x::y::z", "chained module"),
        ("a::b c", "module with space"),
        // 8. Underscore
        ("foo_bar", "underscore in symbol"),
        ("_", "standalone underscore"),
        ("foo _ bar", "underscore with spaces"),
        // 9. Question mark
        ("x?y", "question mark in symbol"),
        ("??", "double question"),
        ("x?.b", "safe access dot"),
        ("x?+y", "question plus"),
        ("x?~y", "question tilde"),
        ("x??y", "double question infix"),
        ("x?:y", "question colon"),
        ("x?!y", "question bang"),
        ("x?>y", "question greater"),
        // 10. Plus
        ("a+b", "plus infix"),
        ("+flag", "plus prefix"),
        ("a+b-c*d/e", "mixed operators"),
        // 11. Numbers
        ("1.5", "float"),
        ("1.", "float trailing dot"),
        ("10..20", "range number"),
        ("..10", "range prefix number"),
        ("10..", "number trailing dots"),
        // 12. Long operators
        ("a===b", "strict equal"),
        ("x=>y", "arrow"),
        ("x->y", "fat arrow"),
        ("x|>", "pipe"),
        ("x|>y", "pipe infix"),
        ("x&&y", "and"),
        ("x||y", "or"),
        ("x<<y", "shift left"),
        ("x>>y", "shift right"),
        ("x+=1", "assign add"),
        ("x-=1", "assign sub"),
        ("x:=1", "assign col"),
        ("x~:y", "tilde colon"),
        ("x?+y", "q plus"),
        // 13. CFM mode
        ("> ls -la", "CFM mode"),
        ("ls -la --color=auto", "CFM args"),
        ("ls -la", "CFM simple"),
        // 14. Semicolons
        ("x;y", "semicolon"),
        ("echo hello; echo world", "semicolon commands"),
        // 15. Percent
        ("a%b", "modulo"),
        ("x%%y", "double percent"),
        ("%{", "percent block"),
        // 16. Caret
        ("x^y", "caret"),
        ("x^y^z", "double caret"),
        // 17. Pipe
        ("x|y", "pipe"),
        ("x|y&&z", "pipe and"),
        ("x|>y|>z", "chained pipe"),
        // 18. Various edge cases
        ("x/y", "slash"),
        ("x/z", "slash in symbol"),
        ("x{y", "brace open"),
        ("x,z", "comma"),
        ("x z", "space"),
    ];

    for (input, desc) in inputs {
        println!("=== {:<30} | {:?} ===", desc, input);
        let (tokens, diags) = tokenize(input);
        for tok in &tokens {
            print!(
                "  {:<20} '{}'",
                format!("{:?}", tok.kind),
                tok.range.to_str(input)
            );
        }
        println!();
        for diag in &diags {
            println!("  DIAG: {:?}", diag);
        }
        if tokens.is_empty() {
            println!("  (no tokens)");
        }
        println!();
    }
}
