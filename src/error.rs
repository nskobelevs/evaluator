use crate::pretty_json::PrettyJson;
use crate::repository::{
    CreateRuleError, DeleteRuleError, EvaluateRuleError, GetAllRulesError, GetRuleError,
    UpdateRuleError,
};
use actix_web::{
    HttpResponse, HttpResponseBuilder, ResponseError, body::BoxBody, http::StatusCode,
};
use serde::{Deserialize, Serialize};

macro_rules! impl_response_error {
    ($($error:tt {
        $(
            $variant:pat => $status_code:expr
        ),+
    }),+) => {
        $(
            impl ResponseError for $error {
            fn status_code(&self) -> StatusCode {
                match self {
                    $(
                        $variant => $status_code
                    ),+
                }
            }

            fn error_response(&self) -> HttpResponse<BoxBody> {
                HttpResponseBuilder::new(self.status_code()).json_pretty(ApiError::from(self))
            }
        }
        )+
    };
}

impl_response_error!(
    GetAllRulesError {
        GetAllRulesError::Unknown => StatusCode::INTERNAL_SERVER_ERROR
    },
    GetRuleError {
        GetRuleError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        GetRuleError::NoSuchRule(_) => StatusCode::NOT_FOUND
    },
    CreateRuleError {
        CreateRuleError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        CreateRuleError::Duplicate(_) => StatusCode::BAD_REQUEST
    },
    DeleteRuleError {
        DeleteRuleError::Unknown => StatusCode::INTERNAL_SERVER_ERROR
    },
    UpdateRuleError {
        UpdateRuleError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        UpdateRuleError::NoSuchRule(_) => StatusCode::NOT_FOUND
    },
    EvaluateRuleError {
        EvaluateRuleError::NoSuchRule(_) => StatusCode::NOT_FOUND,
        EvaluateRuleError::EvaluationError(_, _) => StatusCode::BAD_REQUEST,
        EvaluateRuleError::Unknown => StatusCode::INTERNAL_SERVER_ERROR
    }
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: InnerError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerError {
    pub message: String,
}

impl<E> From<E> for ApiError
where
    E: std::error::Error,
{
    fn from(error: E) -> Self {
        Self {
            error: InnerError {
                message: error.to_string(),
            },
        }
    }
}
