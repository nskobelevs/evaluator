use std::fs;

use crate::rule::Rule;

mod rule;

fn main() {
    let input = fs::read_to_string("rules.json").unwrap();

    let rules: Vec<Rule> = serde_json::from_str(&input).unwrap();

    println!("{rules:#?}");

    let json = serde_json::to_string_pretty(&rules).unwrap();

    println!("{json}");
}
