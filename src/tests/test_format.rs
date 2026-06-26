use lumesh::{Expression, parse_script};
fn main() {
    let expr = parse_script("fn add(a, b) { a + b }").unwrap();
    println!("FN DEF: {:?}", expr);
    println!("---");
    let expr2 = parse_script("(x) -> x * 2").unwrap();
    println!("LAMBDA: {:?}", expr2);
    println!("---");
    let expr3 = parse_script("let x = 5").unwrap();
    println!("DECLARE: {:?}", expr3);
    println!("---");
    let expr4 = parse_script("1..10:2").unwrap();
    println!("RANGE_STEP: {:?}", expr4);
}
