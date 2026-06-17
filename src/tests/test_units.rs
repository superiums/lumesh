use lumesh::tokenize;
fn main() {
    let (tokens, diags) = tokenize("5K 10M 1G");
    println!("tokens ({}):", tokens.len());
    for t in &tokens {
        println!("  {:?} kind={:?}", t.span(), t.kind());
    }
    println!("diags ({}):", diags.len());
    for d in &diags {
        println!("  {:?}", d);
    }
}
