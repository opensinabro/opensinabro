fn main() {
    let src = std::env::args().nth(1).unwrap();
    println!("{:#?}", namumark_syntax::parse(&src).root());
}
