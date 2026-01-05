use dummy_markdown::parser;

const DOC: &str = r#"
Test math:

Inline \(x^2\) and dollar $y^2$

Block math:
\[
\int_0^\infty e^{-x^2} dx
\]

Dollar block:
$$
\sum_{i=1}^n i
$$
"#;

fn main() {
    let (ui_elems, link_urls) = parser::run(DOC, true);
    println!("Total elements: {}", ui_elems.len());
    for (i, elem) in ui_elems.iter().enumerate() {
        println!("{}: {:?}", i, elem);
    }
    println!("\nLinks: {:?}", link_urls);
}
