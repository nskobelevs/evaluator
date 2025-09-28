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

fn create_server<RR: RuleRepository>(rule_repository: RR) -> Result<dev::Server, std::io::Error> {
    Ok(HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                rule_repository: rule_repository.clone(),
            }))
            .route("/rules", web::get().to(get_all_rules_handler::<RR>))
            .route("/rules/{id}", web::get().to(get_rule_handler::<RR>))
            .route("/rules/{id}", web::post().to(create_rule_handler::<RR>))
            .route("/rules/{id}", web::put().to(update_rule_handler::<RR>))
            .route("/rules/{id}", web::delete().to(delete_rule_handler::<RR>))
            .route("/evaluate", web::post().to(evaluate_rules_handler::<RR>))
    })
    .bind(("127.0.0.1", 8080))?
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
