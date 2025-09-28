use actix_web::{HttpResponse, HttpResponseBuilder, error::JsonPayloadError, http::header, mime};
use serde::Serialize;

pub trait PrettyJson {
    fn json_pretty(&mut self, value: impl Serialize) -> HttpResponse;
}

impl PrettyJson for HttpResponseBuilder {
    fn json_pretty(&mut self, value: impl Serialize) -> HttpResponse {
        match serde_json::to_string_pretty(&value) {
            Ok(body) => {
                self.insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON));
                self.body(body)
            }
            Err(err) => HttpResponse::from_error(JsonPayloadError::Serialize(err)),
        }
    }
}
