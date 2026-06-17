#[test]
fn test_section() {
    let (tokens, diags) = lumesh::tokenize("hello §world");
    println!("tokens: {tokens:?}");
    println!("diags: {diags:?}");
}
