use thiserror::Error;

use crate::core::rule::{CompoundPredicate, Operator, Predicate, RawPredicate, Rule};

type JsonValue = serde_json::Value;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum EvaluationError {
    #[error("cannot read field `{field}` of type {kind}")]
    NotAnObject { field: String, kind: &'static str },
    #[error("cannot compare {lhs} with {rhs} using operator {operator:?}")]
    TypeMismatch {
        lhs: &'static str,
        rhs: &'static str,
        operator: Operator,
    },
}

impl EvaluationError {
    fn not_an_object(field: String, value: &JsonValue) -> Self {
        Self::NotAnObject {
            field,
            kind: json_type(value),
        }
    }

    fn type_mismatch(lhs: &JsonValue, rhs: &JsonValue, operator: Operator) -> Self {
        Self::TypeMismatch {
            lhs: json_type(lhs),
            rhs: json_type(rhs),
            operator,
        }
    }
}

impl Rule {
    pub fn evaluate(&self, input: &JsonValue) -> Result<bool, EvaluationError> {
        self.predicate.evaluate(input)
    }
}

impl Predicate {
    pub fn evaluate(&self, input: &JsonValue) -> Result<bool, EvaluationError> {
        match self {
            Predicate::Raw(predicate) => predicate.evaluate(input),
            Predicate::Compound(predicate) => predicate.evaluate(input),
        }
    }
}

fn json_type(value: &JsonValue) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn follow_path<'a>(path: &str, input: &'a JsonValue) -> Result<&'a JsonValue, EvaluationError> {
    let mut head = input;

    for field in path.split(".") {
        if !head.is_object() {
            return Err(EvaluationError::not_an_object(field.to_owned(), head));
        }

        head = &head[field];
    }

    Ok(head)
}

impl RawPredicate {
    pub fn evaluate(&self, input: &JsonValue) -> Result<bool, EvaluationError> {
        let data = follow_path(&self.path, input)?;

        match self.operator {
            Operator::Equal => Ok(data == &self.value),
            Operator::NotEqual => Ok(data != &self.value),
            Operator::Greater | Operator::Less | Operator::GreaterEqual | Operator::LessEqual => {
                let (Some(lhs), Some(rhs)) = (data.as_f64(), self.value.as_f64()) else {
                    return Err(EvaluationError::type_mismatch(
                        data,
                        &self.value,
                        self.operator,
                    ));
                };

                Ok(match self.operator {
                    Operator::Greater => lhs > rhs,
                    Operator::Less => lhs < rhs,
                    Operator::GreaterEqual => lhs >= rhs,
                    Operator::LessEqual => lhs <= rhs,
                    other => unreachable!("got unexpected non-mathematical operator {other:?}"),
                })
            }
            Operator::Contains => {
                let Some(lhs) = data.as_array() else {
                    return Err(EvaluationError::type_mismatch(
                        data,
                        &self.value,
                        self.operator,
                    ));
                };

                Ok(lhs.contains(&self.value))
            }
        }
    }
}

impl CompoundPredicate {
    pub fn evaluate(&self, input: &JsonValue) -> Result<bool, EvaluationError> {
        match self {
            CompoundPredicate::Not(predicate) => predicate.evaluate(input).map(|b| !b),
            CompoundPredicate::Any(predicates) => {
                for predicate in predicates {
                    if predicate.evaluate(input)? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            CompoundPredicate::All(predicates) => {
                for predicate in predicates {
                    if !predicate.evaluate(input)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            CompoundPredicate::None(predicates) => {
                for predicate in predicates {
                    if predicate.evaluate(input)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{all, any, none, not, predicate, rule};
    use serde_json::json;

    macro_rules! not_an_object_err {
        ($field:expr, $kind:expr) => {
            Err(EvaluationError::NotAnObject {
                field: $field.into(),
                kind: $kind,
            })
        };
    }

    #[test]
    fn test_follow_path() {
        assert_eq!(follow_path("foo", &json!({"foo": 10})), Ok(&json!(10)));
        assert_eq!(
            follow_path("foo.bar", &json!({"foo": {"bar": 123}})),
            Ok(&json!(123))
        );
        assert_eq!(
            follow_path("a.b.c", &json!({"a": {"b": {"c": [1,2,3]}}})),
            Ok(&json!([1, 2, 3]))
        );
        assert_eq!(
            follow_path("a.b", &json!({"a": {"b": {"c": [1, 2, 3]}}})),
            Ok(&json!({"c": [1, 2, 3]}))
        );

        assert_eq!(
            follow_path("b.a", &json!({"b": [1,2,3]})),
            not_an_object_err!("a", "array")
        );
    }

    mod evaluate {
        use super::*;

        mod operators {
            use super::*;

            macro_rules! test_op {
                ($operator:tt, $expected:expr, $value:expr, $($input:tt)*) => {
                    assert_eq!(
                        predicate!("field" $operator $value).evaluate(&json!({
                            "field": $($input)*
                        })),
                        $expected
                    );
                };
            }

            macro_rules! type_err {
                ($lhs:literal,$rhs:literal, $operator:expr) => {
                    Err(EvaluationError::TypeMismatch {
                        lhs: $lhs,
                        rhs: $rhs,
                        operator: $operator,
                    })
                };
            }

            mod equal {
                use super::*;

                macro_rules! test_eq_op {
                    ($expected:expr, $value:expr, $($input:tt)*) => {
                        test_op!(==, $expected, $value, $($input)*);
                        test_op!(!=, $expected.map(|b| !b), $value, $($input)*);

                    };
                }

                #[test]
                fn test_null() {
                    test_eq_op!(Ok(true), (), null);

                    test_eq_op!(Ok(false), (), true);
                    test_eq_op!(Ok(false), (), false);

                    test_eq_op!(Ok(false), (), "test");
                    test_eq_op!(Ok(false), (), "");
                    test_eq_op!(Ok(false), (), "null");

                    test_eq_op!(Ok(false), (), []);
                    test_eq_op!(Ok(false), (), [null]);

                    test_eq_op!(Ok(false), (), {});
                    test_eq_op!(Ok(false), (), {"foo": null});
                }

                #[test]
                fn test_bool() {
                    test_eq_op!(Ok(true), true, true);
                    test_eq_op!(Ok(true), false, false);
                    test_eq_op!(Ok(false), true, false);
                    test_eq_op!(Ok(false), false, true);

                    test_eq_op!(Ok(false), true, "true");
                    test_eq_op!(Ok(false), true, 1);
                    test_eq_op!(Ok(false), true, [true]);
                    test_eq_op!(Ok(false), true, {});
                }

                #[test]
                fn test_number() {
                    test_eq_op!(Ok(true), 0, 0);
                    test_eq_op!(Ok(true), 3.987, 3.987);
                    test_eq_op!(Ok(true), 10, 10);

                    test_eq_op!(Ok(false), 0, 1);
                    test_eq_op!(Ok(false), 0.001, 0.01);
                    test_eq_op!(Ok(false), 0, "0");
                    test_eq_op!(Ok(false), 0, []);
                    test_eq_op!(Ok(false), 0, [0]);
                    test_eq_op!(Ok(false), 0, null);

                    test_eq_op!(Ok(false), 0, {});
                }

                #[test]
                fn test_string() {
                    test_eq_op!(Ok(true), "", "");
                    test_eq_op!(Ok(true), "foo", "foo");

                    test_eq_op!(Ok(false), "Foo", "foo");
                    test_eq_op!(Ok(false), "123", 123);
                    test_eq_op!(Ok(false), "null", null);
                    test_eq_op!(Ok(false), "foo", ["foo"]);
                    test_eq_op!(Ok(false), "{}", {});
                }

                #[test]
                fn test_array() {
                    test_eq_op!(Ok(true), Vec::<u8>::new(), []);
                    test_eq_op!(Ok(true), [1, 2, 3], [1, 2, 3]);
                    test_eq_op!(
                        Ok(true),
                        [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
                        [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
                    );
                    test_eq_op!(Ok(true), ["foo"], ["foo"]);
                    test_eq_op!(Ok(true), [json!({"foo": "bar"})], [{"foo": "bar"}]);

                    test_eq_op!(Ok(false), Vec::<u8>::new(), null);
                    test_eq_op!(Ok(false), Vec::<u8>::new(), 0);
                    test_eq_op!(Ok(false), Vec::<u8>::new(), {});
                    test_eq_op!(Ok(false), Vec::<u8>::new(), "{}");
                    test_eq_op!(Ok(false), Vec::<u8>::new(), true);
                    test_eq_op!(Ok(false), Vec::<u8>::new(), false);
                }

                #[test]
                fn test_object() {
                    test_eq_op!(Ok(true), json!({}), {});
                    test_eq_op!(
                        Ok(true),
                        json!({"foo": {"bar": [1], "baz": null, "buzz": 123, "bizz": true, "faz": "fizz"}}),
                        {"foo": {"bar": [1], "baz": null, "buzz": 123, "bizz": true, "faz": "fizz"}}
                    );

                    test_eq_op!(Ok(false), json!({}), {"foo": null});
                    test_eq_op!(Ok(false), json!({"foo": null}), {});
                    test_eq_op!(Ok(false), json!({}), null);
                    test_eq_op!(Ok(false), json!({}), 0);
                    test_eq_op!(Ok(false), json!({}), true);
                    test_eq_op!(Ok(false), json!({}), false);
                    test_eq_op!(Ok(false), json!({}), []);
                    test_eq_op!(Ok(false), json!({}), [{}]);
                    test_eq_op!(Ok(false), json!({"foo": 10, "baz": {"baz": 12}}), {"foo": 10, "baz": {"baz": 13}});
                }
            }

            mod math {
                use super::*;

                #[test]
                fn test_type_mismatch_error() {
                    test_op!(>=, type_err!("number", "null", Operator::GreaterEqual), (), 10);
                    test_op!(<=, type_err!("null", "number", Operator::LessEqual), 10, null);

                    test_op!(>, type_err!("number", "boolean", Operator::Greater), true, 10);
                    test_op!(<, type_err!("boolean", "number", Operator::Less), 10, false);

                    test_op!(>, type_err!("number", "string", Operator::Greater), "10", 10);
                    test_op!(<, type_err!("string", "number", Operator::Less), 10, "string");

                    test_op!(<=, type_err!("number", "array", Operator::LessEqual), vec![10], 10);
                    test_op!(>=, type_err!("array", "number", Operator::GreaterEqual), 10, [10]);

                    test_op!(<=, type_err!("number", "object", Operator::LessEqual), json!({}), 10);
                    test_op!(>=, type_err!("object", "number", Operator::GreaterEqual), 10, {"foo": "bar"});
                }

                #[test]
                fn test_greater() {
                    test_op!(>, Ok(true), 10, 15);
                    test_op!(>, Ok(true), 0, 15);
                    test_op!(>, Ok(true), -60, 15);
                    test_op!(>, Ok(true), 14, 15);

                    test_op!(>, Ok(false), 15, 15);
                    test_op!(>, Ok(false), 20, 15);
                    test_op!(>, Ok(false), 100, 15);
                }

                #[test]
                fn test_greater_equal() {
                    test_op!(>=, Ok(true), 10, 15);
                    test_op!(>=, Ok(true), 0, 15);
                    test_op!(>=, Ok(true), -60, 15);
                    test_op!(>=, Ok(true), 14, 15);
                    test_op!(>=, Ok(true), 15, 15);
                    test_op!(>=, Ok(true), 0, 0);

                    test_op!(>=, Ok(false), 20, 15);
                    test_op!(>=, Ok(false), 100, 15);
                }

                #[test]
                fn test_less() {
                    test_op!(<, Ok(false), 10, 15);
                    test_op!(<, Ok(false), 0, 15);
                    test_op!(<, Ok(false), -60, 15);
                    test_op!(<, Ok(false), 14, 15);
                    test_op!(<, Ok(false), 15, 15);

                    test_op!(<, Ok(true), 20, 15);
                    test_op!(<, Ok(true), 100, 15);
                }

                #[test]
                fn test_less_equal() {
                    test_op!(<=, Ok(false), 10, 15);
                    test_op!(<=, Ok(false), 0, 15);
                    test_op!(<=, Ok(false), -60, 15);
                    test_op!(<=, Ok(false), 14, 15);

                    test_op!(<=, Ok(true), 15, 15);
                    test_op!(<=, Ok(true), 20, 15);
                    test_op!(<=, Ok(true), 100, 15);
                }
            }

            mod contains {
                use super::*;

                #[test]
                fn test_contains() {
                    test_op!(contains, Ok(false), 10, []);
                    test_op!(contains, Ok(true), 10, [10]);
                    test_op!(contains, Ok(true), 10, [0, 1, 2, 10, 3]);

                    test_op!(contains, Ok(true), [1, 2, 3], [[1, 2, 3], [3, 2, 1]]);

                    test_op!(contains, Ok(false), true, [false, false]);
                    test_op!(contains, Ok(true), true, [false, true]);

                    test_op!(contains, Ok(false), json!({"foo": {"bar": 10}}), [{"foo": {"bar": 12}}, {"foo": {"bar": 25}}]);
                    test_op!(contains, Ok(true), json!({"foo": {"bar": 10}}), [{"foo": {"bar": 12}}, {"foo": {"bar": 10}}]);
                }

                #[test]
                fn test_contains_type_err() {
                    test_op!(
                        contains,
                        type_err!("object", "number", Operator::Contains),
                        10,
                        {}
                    );

                    test_op!(
                        contains,
                        type_err!("number", "number", Operator::Contains),
                        10,
                        10
                    );

                    test_op!(
                        contains,
                        type_err!("string", "string", Operator::Contains),
                        "Hello",
                        "Hello World"
                    );
                }
            }
        }

        mod rule {
            use super::*;

            macro_rules! assert_rule_eval {
                ($rule:expr, $input:expr, $expected:expr) => {
                    assert_eq!($rule.evaluate(&$input), $expected)
                };
            }

            #[test]
            fn test_nested_rule() {
                let rule = rule!(
                    "id",
                    "rule failed",
                    all!(
                        predicate!("age" >= 12),
                        any!(
                            predicate!("height.feet" > 5),
                            all!(
                                predicate!("height.feet" == 5),
                                predicate!("height.inches" >= 2)
                            )
                        )
                    )
                );

                let create_input = |age: u32, feet: u32, inches: u32| {
                    json!({
                        "age": age,
                        "height": {
                            "feet": feet,
                            "inches": inches
                        }
                    })
                };

                assert_rule_eval!(rule, create_input(12, 5, 2), Ok(true));
                assert_rule_eval!(rule, create_input(12, 5, 3), Ok(true));
                assert_rule_eval!(rule, create_input(12, 5, 12), Ok(true));
                assert_rule_eval!(rule, create_input(12, 6, 0), Ok(true));
                assert_rule_eval!(rule, create_input(12, 6, 11), Ok(true));

                assert_rule_eval!(rule, create_input(12, 5, 1), Ok(false));
                assert_rule_eval!(rule, create_input(12, 5, 0), Ok(false));
                assert_rule_eval!(rule, create_input(12, 3, 1), Ok(false));
            }

            #[test]
            fn test_simple_rule() {
                assert_rule_eval!(
                    rule!("id", "rule failed", predicate!("foo" == 10)),
                    json!({"foo": 10}),
                    Ok(true)
                );

                assert_rule_eval!(
                    rule!("id", "rule failed", predicate!("foo" < 25)),
                    json!({"foo": 30}),
                    Ok(false)
                );

                assert_rule_eval!(
                    rule!("id", "rule failed", predicate!("foo" contains "bar")),
                    json!({"foo": ["bar"]}),
                    Ok(true)
                );
            }

            #[test]
            fn test_all_rule() {
                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        all!(predicate!("fizz" == 3), predicate!("buzz" == 5))
                    ),
                    json!({"fizz": 3, "buzz": 5}),
                    Ok(true)
                );

                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        all!(predicate!("fizz" == 3), predicate!("buzz" == 5))
                    ),
                    json!({"fizz": 3, "buzz": 5}),
                    Ok(true)
                );

                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        all!(predicate!("fizz" == 3), predicate!("buzz" == 5))
                    ),
                    json!({"fizz": 5, "buzz": 3}),
                    Ok(false)
                );

                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        all!(predicate!("fizz" == 3), predicate!("buzz" == 5))
                    ),
                    json!({"fizz": 4, "buzz": 5}),
                    Ok(false)
                );

                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        all!(predicate!("fizz" == 3), predicate!("buzz" == 5))
                    ),
                    json!({"fizz": 3, "buzz": 6}),
                    Ok(false)
                );
            }

            #[test]
            fn test_any_rule() {
                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        any!(predicate!("color" == "red"), predicate!("color" == "blue"))
                    ),
                    json!({"color": "red"}),
                    Ok(true)
                );

                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        any!(predicate!("color" == "red"), predicate!("color" == "blue"))
                    ),
                    json!({"color": "blue"}),
                    Ok(true)
                );

                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        any!(predicate!("color" == "red"), predicate!("color" == "blue"))
                    ),
                    json!({"color": ""}),
                    Ok(false)
                );

                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        any!(predicate!("color" == "red"), predicate!("color" == "blue"))
                    ),
                    json!({"color": "green"}),
                    Ok(false)
                );
            }

            #[test]
            fn test_none_rule() {
                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        none!(predicate!("color" == "red"), predicate!("color" == "blue"))
                    ),
                    json!({"color": "green"}),
                    Ok(true)
                );

                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        none!(predicate!("color" == "red"), predicate!("color" == "blue"))
                    ),
                    json!({"color": "red"}),
                    Ok(false)
                );

                assert_rule_eval!(
                    rule!(
                        "id",
                        "rule failed",
                        none!(predicate!("color" == "red"), predicate!("color" == "blue"))
                    ),
                    json!({"color": "blue"}),
                    Ok(false)
                );
            }

            #[test]
            fn test_not() {
                assert_rule_eval!(
                    rule!("id", "rule failed", not!(predicate!("foo" == 10))),
                    json!({"foo": 10}),
                    Ok(false)
                );

                assert_rule_eval!(
                    rule!("id", "rule failed", not!(predicate!("foo" == 10))),
                    json!({"foo": 15}),
                    Ok(true)
                );
            }
        }
    }
}
