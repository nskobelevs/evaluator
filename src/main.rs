use std::fs;

use serde_json::Value;

use crate::rule::Rule;

mod eval;
mod rule;

fn main() {
    let input = fs::read_to_string("rules.json").unwrap();

    let rules: Vec<Rule> = serde_json::from_str(&input).unwrap();

    println!("{rules:?}");
}
