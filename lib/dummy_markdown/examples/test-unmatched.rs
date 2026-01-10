use dummy_markdown::parser;

const TEST_CASES: &[(&str, &str)] = &[
    // 正常情况
    ("Normal: $x^2$", "Should parse x^2"),
    // 不匹配的起始分隔符
    ("Unmatched start: $x^2", "Should keep as is"),
    // 不匹配的结束分隔符
    ("Unmatched end: x^2$", "Should keep as is"),
    // 多个不匹配
    ("Multiple: $a $b$ $c", "Should only parse $b$"),
    // 空公式
    ("Empty: $$", "Should keep as is (no content)"),
    // 嵌套（应该不处理）
    ("Nested: $a $b$ c$", "Should handle as best effort"),
];

fn main() {
    for (i, (input, description)) in TEST_CASES.iter().enumerate() {
        println!("Test {}: {}", i + 1, description);
        println!("Input: {}", input);
        let (ui_elems, _) = parser::run(input, true);
        println!("Output: {:?}\n", ui_elems);
    }
}
