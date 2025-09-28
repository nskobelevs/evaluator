use std::fs;

use actix_web::{
    App, HttpResponse, HttpServer, Responder, dev,
    web::{self},
};
use evaluator::{
    core::rule::Rule,
    pretty_json::PrettyJson,
    repository::{InMemRuleRepository, RuleRepository},
};
use serde::Deserialize;
use serde_json::Value;

async fn get_all_rules_handler<RR: RuleRepository>(
    state: web::Data<AppState<RR>>,
) -> Result<impl Responder, actix_web::Error> {
    let rules = state.rule_repository.get_all().await?;

    Ok(HttpResponse::Ok().json_pretty(rules))
}

async fn get_rule_handler<RR: RuleRepository>(
    state: web::Data<AppState<RR>>,
    id: web::Path<String>,
) -> Result<impl Responder, actix_web::Error> {
    let rule = state.rule_repository.get(&id).await?;

    Ok(HttpResponse::Ok().json_pretty(rule))
}

async fn create_rule_handler<RR: RuleRepository>(
    state: web::Data<AppState<RR>>,
    rule: web::Json<Rule>,
) -> Result<impl Responder, actix_web::Error> {
    state.rule_repository.create(rule.into_inner()).await?;

    Ok(HttpResponse::Created())
}

async fn delete_rule_handler<RR: RuleRepository>(
    state: web::Data<AppState<RR>>,
    id: web::Path<String>,
) -> Result<impl Responder, actix_web::Error> {
    state.rule_repository.delete(&id.into_inner()).await?;

    Ok(HttpResponse::Ok())
}

async fn update_rule_handler<RR: RuleRepository>(
    state: web::Data<AppState<RR>>,
    id: web::Path<String>,
    rule: web::Json<Rule>,
) -> Result<impl Responder, actix_web::Error> {
    state
        .rule_repository
        .update(id.into_inner(), rule.into_inner())
        .await?;

    Ok(HttpResponse::Ok())
}

#[derive(Debug, Deserialize)]
pub struct EvaluateParams {
    rules: Option<String>,
}

async fn evaluate_rules_handler<RR: RuleRepository>(
    state: web::Data<AppState<RR>>,
    ids: web::Query<EvaluateParams>,
    input: web::Json<Value>,
) -> Result<impl Responder, actix_web::Error> {
    let rules = ids
        .into_inner()
        .rules
        .map(|r| r.split(",").map(String::from).collect::<Vec<_>>())
        .unwrap_or_default();

    let result = state
        .rule_repository
        .evaluate(&rules, input.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json_pretty(result))
}

#[derive(Debug, Clone)]
struct AppState<RR: RuleRepository> {
    rule_repository: RR,
}

fn configure_app<RR: RuleRepository>(cfg: &mut web::ServiceConfig) {
    cfg.route("/rules", web::get().to(get_all_rules_handler::<RR>))
        .route("/rules/{id}", web::get().to(get_rule_handler::<RR>))
        .route("/rules", web::post().to(create_rule_handler::<RR>))
        .route("/rules/{id}", web::put().to(update_rule_handler::<RR>))
        .route("/rules/{id}", web::delete().to(delete_rule_handler::<RR>))
        .route("/evaluate", web::post().to(evaluate_rules_handler::<RR>));
}

fn create_server<RR: RuleRepository>(rule_repository: RR) -> Result<dev::Server, std::io::Error> {
    Ok(HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                rule_repository: rule_repository.clone(),
            }))
            .configure(configure_app::<RR>)
    })
    .bind(("0.0.0.0", 8080))?
    .run())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let starting_rules: Vec<Rule> = serde_json::from_str(
        fs::read_to_string("rules.json")
            .expect("failed to read rules.json")
            .as_str(),
    )
    .expect("failed to parse rules from rules.json");

    create_server(InMemRuleRepository::new(&starting_rules))?.await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::{App, test, web};
    use evaluator::repository::{Evaluation, EvaluationReason, EvaluationResult};
    use evaluator::{predicate, rule};
    use serde_json::json;

    macro_rules! create_test_app {
        () => {
            test::init_service(
                App::new()
                    .app_data(web::Data::new(AppState {
                        rule_repository: InMemRuleRepository::empty(),
                    }))
                    .configure(configure_app::<InMemRuleRepository>),
            )
            .await
        };
    }

    macro_rules! get_rules {
        ($app:expr) => {{
            let req = test::TestRequest::get().uri("/rules").to_request();
            let resp: Vec<Rule> = test::call_and_read_body_json(&$app, req).await;

            resp
        }};
    }

    macro_rules! create_rule {
        ($app:expr, $rule:expr) => {{
            let req = test::TestRequest::post()
                .uri("/rules")
                .set_json(&$rule)
                .to_request();
            let resp = test::call_service(&$app, req).await;

            resp
        }};
    }

    macro_rules! get_rule {
        ($app:expr, $id:expr) => {
            get_rule!(Rule, $app, $id)
        };
        ($kind:tt, $app:expr, $id:expr) => {{
            let req = test::TestRequest::get()
                .uri(&format!("/rules/{}", $id))
                .to_request();
            let resp: $kind = test::call_and_read_body_json(&$app, req).await;

            resp
        }};
    }

    macro_rules! delete_rule {
        ($app:expr, $id:expr) => {{
            let req = test::TestRequest::delete()
                .uri(&format!("/rules/{}", $id))
                .to_request();
            let resp = test::call_service(&$app, req).await;

            resp
        }};
    }

    macro_rules! update_rule {
        ($app:expr, $id:expr, $rule:expr) => {{
            let req = test::TestRequest::put()
                .uri(&format!("/rules/{}", $id))
                .set_json(&$rule)
                .to_request();
            let resp = test::call_service(&$app, req).await;

            resp
        }};
    }

    macro_rules! evaluate {
        ($app:expr, $ids:expr, $input:expr) => {{
            let ids = $ids
                .into_iter()
                .map(|s| String::from(s))
                .collect::<Vec<_>>()
                .join(",");

            let req = test::TestRequest::post()
                .uri(&format!("/evaluate?rules={}", ids))
                .set_json(&$input)
                .to_request();
            let resp: Evaluation = test::call_and_read_body_json(&$app, req).await;

            resp
        }};
    }

    #[actix_web::test]
    async fn test_get_rules_empty() {
        let app = create_test_app!();
        let resp = get_rules!(app);
        assert_eq!(resp.len(), 0);
    }

    #[actix_web::test]
    async fn test_create_rule() {
        let app = create_test_app!();

        let rule = rule!("rule-1", "some message", predicate!("foo" == 10));

        let resp = create_rule!(app, rule);
        assert!(resp.response().status().is_success());

        let resp = get_rule!(app, "rule-1");
        assert_eq!(resp, rule);

        let resp = get_rules!(app);
        assert!(resp.contains(&rule));
    }

    #[actix_web::test]
    async fn test_delete_rule() {
        let app = create_test_app!();

        let rule = rule!("rule-1", "some message", predicate!("foo" == 10));

        let resp = create_rule!(app, rule);
        assert!(resp.response().status().is_success());

        let resp = get_rule!(app, "rule-1");
        assert_eq!(resp, rule);

        let resp = delete_rule!(app, "rule-1");
        assert!(resp.response().status().is_success());

        let resp = delete_rule!(app, "rule-1");
        assert!(
            resp.response().status().is_success(),
            "delete should be idempotent"
        );

        let resp = get_rules!(app);
        assert_eq!(resp.len(), 0);
    }

    #[actix_web::test]
    async fn test_update_rule() {
        let app = create_test_app!();
        let rule = rule!("rule-1", "some message", predicate!("foo" == 10));

        let resp = create_rule!(app, rule);
        assert!(resp.response().status().is_success());

        let updated_rule = rule!("rule-2", "some other message", predicate!("foo" == 12));

        let resp = update_rule!(app, "rule-1", updated_rule);
        assert_eq!(resp.response().status(), StatusCode::OK);

        let resp = get_rule!(app, "rule-2");
        assert_eq!(resp, updated_rule);

        let resp = get_rules!(app);
        assert_eq!(resp.len(), 1);
        assert!(resp.contains(&updated_rule));
        assert!(!resp.contains(&rule));
    }

    #[actix_web::test]
    async fn test_evaluate() {
        let app = create_test_app!();
        let rule1 = rule!("rule-1", "some message", predicate!("foo" == 10));
        let rule2 = rule!("rule-2", "some other message", predicate!("foo" == 14));
        let rule3 = rule!("rule-3", "unused rule", predicate!("foo" < 0));

        create_rule!(app, rule1);
        create_rule!(app, rule2);
        create_rule!(app, rule3);

        let resp = evaluate!(app, ["rule-1", "rule-2"], json!({"foo": 10}));
        assert_eq!(resp.result, EvaluationResult::Fail);
        assert_eq!(resp.reasons.len(), 2);

        assert!(resp.reasons.contains(&EvaluationReason {
            rule: "rule-1".to_owned(),
            requirement: "some message".to_owned(),
            evaluation: EvaluationResult::Pass,
        }));

        assert!(resp.reasons.contains(&EvaluationReason {
            rule: "rule-2".to_owned(),
            requirement: "some other message".to_owned(),
            evaluation: EvaluationResult::Fail,
        }));
    }
}
