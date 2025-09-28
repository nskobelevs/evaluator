use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Rule {
    pub(crate) name: String,
    pub(crate) predicate: Predicate,
    pub(crate) message: String,
}

impl Rule {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn predicate(&self) -> &Predicate {
        &self.predicate
    }

    pub fn message(&self) -> &str {
        &self.message
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{all, any, predicate, rule};

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
