use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Rule {
    pub(crate) name: String,
    pub(crate) predicate: Predicate,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields, rename_all = "camelCase")]
pub enum Predicate {
    Raw(RawPredicate),
    Compound(CompoundPredicate),
}

impl From<RawPredicate> for Predicate {
    fn from(value: RawPredicate) -> Self {
        Predicate::Raw(value)
    }
}

impl From<CompoundPredicate> for Predicate {
    fn from(value: CompoundPredicate) -> Self {
        Predicate::Compound(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawPredicate {
    pub(crate) path: String,
    pub(crate) operator: Operator,
    pub(crate) value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum CompoundPredicate {
    Not(Box<Predicate>),
    Any(Vec<Predicate>),
    All(Vec<Predicate>),
    None(Vec<Predicate>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

mod macros {
    macro_rules! rule {
        ($name:literal, $message:literal, $predicate:expr) => {
            crate::rule::Rule {
                name: String::from($name),
                message: String::from($message),
                predicate: crate::rule::Predicate::from($predicate),
            }
        };
    }

    macro_rules! predicate {
            ($path:literal $operator:tt $value:expr) => {
                    crate::rule::RawPredicate {
                    path: $path.to_owned(),
                    operator: predicate!(operator $operator),
                    value: serde_json::Value::from($value)
                }
            };
            (operator ==) => {crate::rule::Operator::Equal};
            (operator >) => {crate::rule::Operator::Greater};
            (operator <) => {crate::rule::Operator::Less};
            (operator >=) => {crate::rule::Operator::GreaterEqual};
            (operator <=) => {crate::rule::Operator::LessEqual};
            (operator !=) => {crate::rule::Operator::NotEqual};
            (operator contains) => {crate::rule::Operator::Contains};
        }

    macro_rules! any { ($($predicate:expr),*) => {crate::rule::CompoundPredicate::Any(vec![$(crate::rule::Predicate::from($predicate),)*])}; }
    macro_rules! all { ($($predicate:expr),*) => {crate::rule::CompoundPredicate::All(vec![$(crate::rule::Predicate::from($predicate),)*])}; }
    macro_rules! none { ($($predicate:expr),*) => {crate::rule::CompoundPredicate::None(vec![$(crate::rule::Predicate::from($predicate),)*])}; }
    macro_rules! not {
        ($predicate:expr) => {
            crate::rule::CompoundPredicate::Not(Box::new(crate::rule::Predicate::from($predicate)))
        };
    }

    pub(crate) use all;
    pub(crate) use any;
    pub(crate) use none;
    pub(crate) use not;
    pub(crate) use predicate;
    pub(crate) use rule;
}

pub(crate) use macros::*;

#[cfg(test)]
mod tests {
    use super::*;

    mod deserialize {
        use serde_json::json;

        use super::*;

        macro_rules! assert_deserialize {
            ($kind:tt, $input:literal, $expected:expr) => {{
                let parsed: $kind = serde_json::from_str($input).expect("unable to parse JSON");

                assert_eq!(parsed, $expected);
            }};
        }

        #[test]
        fn test_raw_predicate() {
            assert_deserialize!(
                RawPredicate,
                r#"{"path": "foo", "operator": "==", "value": 10}"#,
                predicate!("foo" == 10)
            );

            assert_deserialize!(
                RawPredicate,
                r#"{"path": "foo.bar.baz", "operator": "==", "value": {"bob": 10, "alice": "Red", "eve": [1, 2, 3]}}"#,
                predicate!("foo.bar.baz" == json!({"bob": 10, "alice": "Red", "eve": [1,2,3]}))
            );
        }

        #[test]
        fn test_compound() {
            assert_deserialize!(
                CompoundPredicate,
                r#"
            {
                "any": [
                    {
                        "path": "baz",
                        "operator": "contains",
                        "value": 1810
                    },
                    {
                        "path": "bar",
                        "operator": "contains",
                        "value": 1950
                    }
                ]
            }
            "#,
                any!(
                    predicate!("baz" contains 1810),
                    predicate!("bar" contains 1950)
                )
            );
        }

        #[test]
        fn test_predicate() {
            assert_deserialize!(
                Predicate,
                r#"{
                    "all": [
                        {
                            "path": "age",
                            "operator": ">=",
                            "value": 12
                        },
                        {
                            "any": [
                                {
                                    "path": "height.feet",
                                    "operator": ">",
                                    "value": 5
                                },
                                {
                                    "all": [
                                        {
                                            "path": "height.feet",
                                            "operator": "==",
                                            "value": 5
                                        },
                                        {
                                            "path": "height.inches",
                                            "operator": ">=",
                                            "value": 2
                                        }
                                    ]
                                }
                            ]
                        }
                    ]
                }"#,
                Predicate::Compound(all!(
                    predicate!("age" >= 12),
                    any!(
                        predicate!("height.feet" > 5),
                        all!(
                            predicate!("height.feet" == 5),
                            predicate!("height.inches" >= 2)
                        )
                    )
                ))
            )
        }

        #[test]
        fn test_rule() {
            assert_deserialize!(
                Rule,
                r#"{
                    "name": "rule-1",
                    "message": "Important rule failed",
                    "predicate": {
                        "any": [
                            {
                                "path": "foo",
                                "operator": ">=",
                                "value": 12
                            }
                        ]
                    }
                }"#,
                rule!(
                    "rule-1",
                    "Important rule failed",
                    any!(predicate!("foo" >= 12))
                )
            );
        }
    }
}
