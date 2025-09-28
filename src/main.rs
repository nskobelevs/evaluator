use actix_web::{App, HttpResponse, HttpServer, Responder, delete, get, post, put, web};

#[get("/rules")]
async fn get_all_rules_handler() -> impl Responder {
    HttpResponse::Ok().body("todo")
}

#[get("/rules/{id}")]
async fn get_rule_handler(id: web::Path<String>) -> impl Responder {
    HttpResponse::Ok().body("todo")
}

#[post("/rules")]
async fn create_rule_handler() -> impl Responder {
    HttpResponse::Ok().body("todo")
}

#[delete("/rules/{id}")]
async fn delete_rule_handler(id: web::Path<String>) -> impl Responder {
    HttpResponse::Ok().body("todo")
}

#[put("/rules/{id}")]
async fn update_rule_handler(id: web::Path<String>) -> impl Responder {
    HttpResponse::Ok().body("todo")
}

#[post("/evaluate")]
async fn evaluate_rules_handler() -> impl Responder {
    HttpResponse::Ok().body("todo")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(get_all_rules_handler)
            .service(get_rule_handler)
            .service(create_rule_handler)
            .service(delete_rule_handler)
            .service(update_rule_handler)
            .service(evaluate_rules_handler)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
