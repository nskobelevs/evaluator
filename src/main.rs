use std::fs;

use evaluator::core::rule::Rule;

fn main() {
    let input = fs::read_to_string("rules.json").unwrap();

    let rules: Vec<Rule> = serde_json::from_str(&input).unwrap();

    println!("{rules:?}");
}
