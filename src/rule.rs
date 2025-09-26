use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Rule {
    name: String,
    predicate: Predicate,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields, rename_all = "camelCase")]
pub enum Predicate {
    Raw(RawPredicate),
    Compound(CompoundPredicate),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawPredicate {
    path: String,
    operator: Operator,
    value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum CompoundPredicate {
    Not(Box<Predicate>),
    Any(Vec<Predicate>),
    All(Vec<Predicate>),
    None(Vec<Predicate>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Operator {
    #[serde(alias = "==")]
    Equal,
    #[serde(alias = ">")]
    Greater,
    #[serde(alias = "<")]
    Less,
    #[serde(alias = ">=")]
    GreaterEqual,
    #[serde(alias = "<=")]
    LessEqual,
    #[serde(alias = "!=")]
    NotEqual,
    #[serde(alias = "in")]
    Contains,
}
