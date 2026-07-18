fn main() {
    let src = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "||a ||[include(틀:국기, 국명=파라과이)][* 각주] ||".to_string());
    println!("{:#?}", namumark_parser::parse(&src));
}
