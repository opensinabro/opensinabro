fn main() {
    let source = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    println!("{:#?}", namumark_syntax::parse(&source).root());
}
