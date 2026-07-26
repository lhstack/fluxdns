//! DNS Query API module
//!
//! Implements REST API endpoint for DNS query tool.
//!
//! # Requirements
//!
//! - 4.8: Provide domain query functionality interface
//! - 4.9: Return DNS resolution results
//! - 4.10: Display record value, TTL, response time, cache hit info
//! - 4.11: Support all DNS record types

use std::sync::Arc;

use std::str::FromStr;

use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::application::web::ApiError;
use crate::business::dns_query_business::DnsQueryBusiness;
use crate::dns::RecordType;
use crate::infrastructure::common::AppError;

/// Application state for DNS query API
#[derive(Clone)]
pub struct DnsQueryState {
    pub business: Arc<DnsQueryBusiness>,
}

/// Valid record types
const VALID_RECORD_TYPES: &[&str] = &["A", "AAAA", "CNAME", "MX", "TXT", "PTR", "NS", "SOA", "SRV"];

/// DNS query request
#[derive(Debug, Clone, Deserialize)]
pub struct DnsQueryRequest {
    pub domain: String,
    pub record_type: String,
}

/// DNS record result
#[derive(Debug, Clone, Serialize)]
pub struct DnsRecordResult {
    pub name: String,
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
}

/// DNS query response
#[derive(Debug, Clone, Serialize)]
pub struct DnsQueryResponse {
    pub domain: String,
    pub record_type: String,
    pub records: Vec<DnsRecordResult>,
    pub response_time_ms: u64,
    pub cache_hit: bool,
    pub upstream_used: Option<String>,
    pub rewrite_applied: bool,
    pub response_code: String,
}

/// Validation error details
#[derive(Debug, Serialize)]
pub struct ValidationErrors {
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Serialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl DnsQueryRequest {
    /// Validate the query request
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if self.domain.is_empty() {
            errors.push(ValidationError {
                field: "domain".to_string(),
                message: "Domain cannot be empty".to_string(),
            });
        } else if self.domain.len() > 255 {
            errors.push(ValidationError {
                field: "domain".to_string(),
                message: "Domain cannot exceed 255 characters".to_string(),
            });
        }

        let upper = self.record_type.to_uppercase();
        if !VALID_RECORD_TYPES.contains(&upper.as_str()) {
            errors.push(ValidationError {
                field: "record_type".to_string(),
                message: format!(
                    "Invalid record type. Must be one of: {}",
                    VALID_RECORD_TYPES.join(", ")
                ),
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { errors })
        }
    }

    /// Get the record type enum
    pub fn get_record_type(&self) -> Option<RecordType> {
        RecordType::from_str(&self.record_type).ok()
    }
}

/// Perform DNS query
///
/// POST /api/dns/query
pub async fn dns_query(
    State(state): State<DnsQueryState>,
    Json(request): Json<DnsQueryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    request.validate().map_err(ApiError::validation)?;

    let record_type = request
        .get_record_type()
        .ok_or_else(|| AppError::Validation("Invalid record type".to_string()))?;

    let result = state.business.resolve(&request.domain, record_type).await?;

    let records: Vec<DnsRecordResult> = result
        .response
        .answers
        .iter()
        .map(|r| DnsRecordResult {
            name: r.name.clone(),
            record_type: r.record_type.to_string(),
            value: r.value.clone(),
            ttl: r.ttl,
        })
        .collect();

    Ok(Json(DnsQueryResponse {
        domain: request.domain,
        record_type: request.record_type.to_uppercase(),
        records,
        response_time_ms: result.metadata.response_time_ms,
        cache_hit: result.metadata.cache_hit,
        upstream_used: result.metadata.upstream_used,
        rewrite_applied: result.metadata.rewrite_applied,
        response_code: result.response.response_code.to_string(),
    }))
}

/// Build the DNS query API router
pub fn dns_query_router(state: DnsQueryState) -> axum::Router {
    use axum::routing::post;

    axum::Router::new()
        .route("/query", post(dns_query))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_query_request_validation_valid() {
        let request = DnsQueryRequest {
            domain: "example.com".to_string(),
            record_type: "A".to_string(),
        };
        assert!(request.validate().is_ok());

        let request = DnsQueryRequest {
            domain: "example.com".to_string(),
            record_type: "aaaa".to_string(),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_dns_query_request_validation_invalid_domain() {
        let request = DnsQueryRequest {
            domain: "".to_string(),
            record_type: "A".to_string(),
        };
        assert!(request.validate().is_err());

        let request = DnsQueryRequest {
            domain: "a".repeat(256),
            record_type: "A".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_dns_query_request_validation_invalid_record_type() {
        let request = DnsQueryRequest {
            domain: "example.com".to_string(),
            record_type: "INVALID".to_string(),
        };
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_dns_query_request_get_record_type() {
        let request = DnsQueryRequest {
            domain: "example.com".to_string(),
            record_type: "A".to_string(),
        };
        assert_eq!(request.get_record_type(), Some(RecordType::A));

        let request = DnsQueryRequest {
            domain: "example.com".to_string(),
            record_type: "aaaa".to_string(),
        };
        assert_eq!(request.get_record_type(), Some(RecordType::AAAA));

        let request = DnsQueryRequest {
            domain: "example.com".to_string(),
            record_type: "INVALID".to_string(),
        };
        assert_eq!(request.get_record_type(), None);
    }
}
