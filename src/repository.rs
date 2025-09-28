use crate::core::{eval::EvaluationError, rule::Rule};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, RwLock},
};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Evaluation {
    pub result: EvaluationResult,
    pub reasons: Vec<EvaluationReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvaluationReason {
    pub rule: String,
    pub requirement: String,
    pub evaluation: EvaluationResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum EvaluationResult {
    Pass,
    Fail,
}

#[derive(Debug, Error, PartialEq, Eq, Hash)]
pub enum CreateRuleError {
    #[error("a rule with id {0} already exists")]
    Duplicate(String),
    #[error("an unknown error occured")]
    Unknown,
}

#[derive(Debug, Error, PartialEq, Eq, Hash)]
pub enum DeleteRuleError {
    #[error("an unknown error occured")]
    Unknown,
}

#[derive(Debug, Error, PartialEq, Eq, Hash)]
pub enum GetRuleError {
    #[error("a rule with id {0} does not exist")]
    NoSuchRule(String),
    #[error("an unknown error occured")]
    Unknown,
}

#[derive(Debug, Error, PartialEq, Eq, Hash)]
pub enum UpdateRuleError {
    #[error("a rule with id {0} does not exist")]
    NoSuchRule(String),
    #[error("an unknown error occured")]
    Unknown,
}

#[derive(Debug, Error, PartialEq, Eq, Hash)]
pub enum GetAllRulesError {
    #[error("an unknown error occured")]
    Unknown,
}

#[derive(Debug, Error, PartialEq, Eq, Hash)]
pub enum EvaluateRuleError {
    #[error("a rule with id {0} does not exist")]
    NoSuchRule(String),
    #[error("failed to evaluate rule {0}: {1}")]
    EvaluationError(String, EvaluationError),
    #[error("an unknown error occured")]
    Unknown,
}

pub trait RuleRepository: Clone + Send + Sync + 'static {
    fn get_all(&self) -> impl Future<Output = Result<Vec<Rule>, GetAllRulesError>> + Send;

    #[allow(clippy::ptr_arg)]
    fn get(&self, id: &String) -> impl Future<Output = Result<Rule, GetRuleError>> + Send;

    fn create(&self, rule: Rule) -> impl Future<Output = Result<(), CreateRuleError>> + Send;

    #[allow(clippy::ptr_arg)]
    fn delete(
        &self,
        id: &String,
    ) -> impl Future<Output = Result<Option<Rule>, DeleteRuleError>> + Send;

    fn update(
        &self,
        id: String,
        new_rule: Rule,
    ) -> impl Future<Output = Result<Option<Rule>, UpdateRuleError>> + Send;

    fn evaluate(
        &self,
        ids: &[String],
        input: serde_json::Value,
    ) -> impl Future<Output = Result<Evaluation, EvaluateRuleError>> + Send;
}

#[derive(Debug, Clone)]
pub struct InMemRuleRepository {
    rules: Arc<RwLock<HashMap<String, Rule>>>,
}

impl InMemRuleRepository {
    pub fn new(rules: &[Rule]) -> Self {
        Self {
            rules: Arc::new(RwLock::new(
                rules
                    .iter()
                    .cloned()
                    .map(|rule| (rule.id.clone(), rule))
                    .collect(),
            )),
        }
    }

    pub fn empty() -> Self {
        Self {
            rules: Arc::default(),
        }
    }
}

impl RuleRepository for InMemRuleRepository {
    async fn get_all(&self) -> Result<Vec<Rule>, GetAllRulesError> {
        let rules = self.rules.read().map_err(|_| GetAllRulesError::Unknown)?;

        Ok(rules.values().cloned().collect())
    }

    async fn get(&self, id: &String) -> Result<Rule, GetRuleError> {
        let rules = self.rules.read().map_err(|_| GetRuleError::Unknown)?;

        if let Some(rule) = rules.get(id) {
            Ok(rule.clone())
        } else {
            Err(GetRuleError::NoSuchRule(id.clone()))
        }
    }

    async fn create(&self, rule: Rule) -> Result<(), CreateRuleError> {
        let mut rules = self.rules.write().map_err(|_| CreateRuleError::Unknown)?;

        let id = rule.id().to_owned();

        #[allow(clippy::map_entry)]
        if rules.contains_key(&id) {
            Err(CreateRuleError::Duplicate(id.clone()))
        } else {
            rules.insert(id, rule);

            Ok(())
        }
    }

    async fn delete(&self, id: &String) -> Result<Option<Rule>, DeleteRuleError> {
        let mut rules = self.rules.write().map_err(|_| DeleteRuleError::Unknown)?;

        Ok(rules.remove(id))
    }

    async fn update(&self, id: String, new_rule: Rule) -> Result<Option<Rule>, UpdateRuleError> {
        let mut rules = self.rules.write().map_err(|_| UpdateRuleError::Unknown)?;

        if !rules.contains_key(&id) {
            return Err(UpdateRuleError::NoSuchRule(id.clone()));
        }

        let old_rule = rules.remove(&id);

        rules.insert(new_rule.id.clone(), new_rule);

        Ok(old_rule)
    }

    async fn evaluate(
        &self,
        ids: &[String],
        input: serde_json::Value,
    ) -> Result<Evaluation, EvaluateRuleError> {
        let rules = self.rules.read().map_err(|_| EvaluateRuleError::Unknown)?;

        let mut reasons = Vec::with_capacity(ids.len());

        let mut is_pass = true;

        for id in ids {
            let Some(rule) = rules.get(id) else {
                return Err(EvaluateRuleError::NoSuchRule(id.clone()));
            };

            let evaluation = rule
                .evaluate(&input)
                .map_err(|err| EvaluateRuleError::EvaluationError(id.clone(), err))?;

            if evaluation {
                reasons.push(EvaluationReason {
                    rule: id.clone(),
                    evaluation: EvaluationResult::Pass,
                    requirement: rule.message.clone(),
                });
            } else {
                reasons.push(EvaluationReason {
                    rule: id.clone(),
                    evaluation: EvaluationResult::Fail,
                    requirement: rule.message.clone(),
                });
            }

            is_pass &= evaluation == true;
        }

        Ok(Evaluation {
            result: if is_pass {
                EvaluationResult::Pass
            } else {
                EvaluationResult::Fail
            },
            reasons,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{predicate, rule};

    mod in_mem_rule_repository {
        use super::*;

        macro_rules! assert_repository_size {
            ($db:expr, $expected:literal) => {{
                let rules = $db.get_all().await.expect("get_all failed unexpectedly");
                assert_eq!(rules.len(), $expected);
            }};
        }

        macro_rules! assert_repository_contains {
            ($db:expr, $rule:expr) => {{
                let rules = $db.get_all().await.expect("get_all failed unexpectedly");

                assert!(rules.contains(&$rule));

                let fetched_rule = $db.get(&$rule.id).await.expect("get failed unexpectedly");
                assert_eq!(fetched_rule, $rule);
            }};
        }

        macro_rules! assert_repository_does_not_contain {
            ($db:expr, $rule:expr) => {{
                let rules = $db.get_all().await.expect("get_all failed unexpectedly");

                assert!(!rules.contains(&$rule));

                let fetched_rule = $db.get(&$rule.id).await;

                match fetched_rule {
                    err @ Err(_) => {
                        assert_eq!(err, Err(GetRuleError::NoSuchRule($rule.id.clone())));
                    }
                    Ok(fetched_rule) => {
                        assert_ne!(fetched_rule, $rule);
                    }
                }
            }};
        }

        #[tokio::test]
        async fn test_create_rule() {
            let db = InMemRuleRepository::empty();
            assert_repository_size!(db, 0);

            let rule = rule!("rule-1", "important rule failed", predicate!("foo" == 10));

            db.create(rule.clone())
                .await
                .expect("rule creation should not fail");

            assert_repository_size!(db, 1);
            assert_repository_contains!(db, rule);
        }

        #[tokio::test]
        async fn test_delete_rule() {
            let db = InMemRuleRepository::empty();
            let rule = rule!("rule-1", "important rule failed", predicate!("foo" == 10));

            db.create(rule.clone())
                .await
                .expect("rule creation should not fail");

            assert_repository_size!(db, 1);
            assert_repository_contains!(db, rule);

            db.delete(&rule.id).await.expect("delete should not fail");

            assert_repository_size!(db, 0);
            assert_repository_does_not_contain!(db, rule);
        }

        #[tokio::test]
        async fn test_create_duplicate_err() {
            let db = InMemRuleRepository::empty();
            assert_repository_size!(db, 0);

            let rule = rule!("rule-1", "important rule failed", predicate!("foo" == 10));

            db.create(rule.clone())
                .await
                .expect("rule creation should not fail");

            let creation_result = db.create(rule.clone()).await;

            assert!(matches!(
                creation_result,
                Err(CreateRuleError::Duplicate(_))
            ))
        }

        #[tokio::test]
        async fn test_delete_idempotent() {
            let db = InMemRuleRepository::empty();
            let rule = rule!("rule-1", "important rule failed", predicate!("foo" == 10));

            db.create(rule.clone())
                .await
                .expect("rule creation should not fail");

            assert_repository_size!(db, 1);
            assert_repository_contains!(db, rule);

            db.delete(&rule.id).await.expect("delete should not fail");

            assert_repository_size!(db, 0);
            assert_repository_does_not_contain!(db, rule);

            db.delete(&rule.id)
                .await
                .expect("delete of non existing rule should not fail");

            assert_repository_size!(db, 0);
            assert_repository_does_not_contain!(db, rule);
        }

        #[tokio::test]
        async fn test_update() {
            let db = InMemRuleRepository::empty();
            let rule = rule!("rule-1", "important rule failed", predicate!("foo" == 10));

            db.create(rule.clone())
                .await
                .expect("rule creation should not fail");

            let updated_rule = rule!("rule-2", "updated message", predicate!("foo" == 10));

            assert_repository_contains!(db, rule);
            assert_repository_does_not_contain!(db, updated_rule);

            db.update(rule.id.clone(), updated_rule.clone())
                .await
                .expect("update should not fail");

            assert_repository_contains!(db, updated_rule);
            assert_repository_does_not_contain!(db, rule);
        }

        #[tokio::test]
        async fn test_update_err() {
            let db = InMemRuleRepository::empty();
            let rule = rule!("rule-1", "important rule failed", predicate!("foo" == 10));

            db.create(rule.clone())
                .await
                .expect("rule creation should not fail");

            let updated_rule = rule!("rule-2", "updated message", predicate!("foo" == 10));

            let update_result = db.update("rule-3".to_owned(), updated_rule.clone()).await;

            assert!(matches!(update_result, Err(UpdateRuleError::NoSuchRule(_))));
        }
    }
}
