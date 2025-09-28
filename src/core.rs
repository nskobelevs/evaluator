pub mod eval;
pub mod rule;

#[macro_export]
macro_rules! rule {
    ($id:literal, $message:literal, $predicate:expr) => {
        $crate::core::rule::Rule {
            id: String::from($id),
            message: String::from($message),
            predicate: $crate::core::rule::Predicate::from($predicate),
        }
    };
}

#[macro_export]
macro_rules! predicate {
            ($path:literal $operator:tt $value:expr) => {
                    $crate::core::rule::RawPredicate {
                    path: $path.to_owned(),
                    operator: predicate!(operator $operator),
                    value: serde_json::Value::from($value)
                }
            };
            (operator ==) => {$crate::core::rule::Operator::Equal};
            (operator >) => {$crate::core::rule::Operator::Greater};
            (operator <) => {$crate::core::rule::Operator::Less};
            (operator >=) => {$crate::core::rule::Operator::GreaterEqual};
            (operator <=) => {$crate::core::rule::Operator::LessEqual};
            (operator !=) => {$crate::core::rule::Operator::NotEqual};
            (operator contains) => {$crate::core::rule::Operator::Contains};
        }

#[macro_export]
macro_rules! any { ($($predicate:expr),*) => {$crate::core::rule::CompoundPredicate::Any(vec![$($crate::core::rule::Predicate::from($predicate),)*])}; }

#[macro_export]
macro_rules! all { ($($predicate:expr),*) => {$crate::core::rule::CompoundPredicate::All(vec![$($crate::core::rule::Predicate::from($predicate),)*])}; }

#[macro_export]
macro_rules! none { ($($predicate:expr),*) => {$crate::core::rule::CompoundPredicate::None(vec![$($crate::core::rule::Predicate::from($predicate),)*])}; }

#[macro_export]
macro_rules! not {
    ($predicate:expr) => {
        $crate::core::rule::CompoundPredicate::Not(Box::new($crate::core::rule::Predicate::from(
            $predicate,
        )))
    };
}
