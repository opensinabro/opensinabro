fn main() {
    let document = namumark::parse(" 1. 하나\n  * 점\n 2. 둘");
    println!("{:#?}", document.blocks);
}
