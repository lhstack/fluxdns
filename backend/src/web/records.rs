//! DNS Records API module
//!
//! Implements REST API endpoints for DNS record management.
//!
//! # Requirements
//!
//! - 4.3: Provide DNS record management functionality

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::db::{CreateDnsRecord, Database, DnsRecord, UpdateDnsRecord};
use crate::web::ApiError;

/// Application state for DNS records API
#[derive(Clone)]
pub struct RecordsState {
    pub db: Arc<Database>,
}

/// Supported DNS record types
const VALID_RECORD_TYPES: &[&str] = &["A", "AAAA", "CNAME", "MX", "TXT", "PTR", "NS", "SOA", "SRV"];

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

/// Create DNS record request with validation
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRecordRequest {
    pub name: String,
    pub record_type: String,
    pub value: String,
    #[serde(default = "default_ttl")]
    pub ttl: i32,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_ttl() -> i32 {
    300
}

fn default_enabled() -> bool {
    true
}

/// Update DNS record request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRecordRequest {
    pub name: Option<String>,
    pub record_type: Option<String>,
    pub value: Option<String>,
    pub ttl: Option<i32>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}

/// API response wrapper for single record
#[derive(Debug, Serialize)]
pub struct RecordResponse {
    pub data: DnsRecord,
}

/// API response wrapper for multiple records
#[derive(Debug, Serialize)]
pub struct RecordsListResponse {
    pub data: Vec<DnsRecord>,
    pub total: usize,
}

/// Validate a DNS record name
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if name.len() > 255 {
        return Err("Name cannot exceed 255 characters".to_string());
    }
    // Basic DNS name validation - allow alphanumeric, hyphens, dots, and wildcards
    let valid_chars = name.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '*'
    });
    if !valid_chars {
        return Err("Name contains invalid characters".to_string());
    }
    Ok(())
}

/// Validate a DNS record type
fn validate_record_type(record_type: &str) -> Result<(), String> {
    let upper = record_type.to_uppercase();
    if !VALID_RECORD_TYPES.contains(&upper.as_str()) {
        return Err(format!(
            "Invalid record type. Must be one of: {}",
            VALID_RECORD_TYPES.join(", ")
        ));
    }
    Ok(())
}

/// Validate a DNS record value based on record type
fn validate_value(value: &str, record_type: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("Value cannot be empty".to_string());
    }
    
    match record_type.to_uppercase().as_str() {
        "A" => {
            // Validate IPv4 address
            if value.parse::<std::net::Ipv4Addr>().is_err() {
                return Err("Invalid IPv4 address for A record".to_string());
            }
        }
        "AAAA" => {
            // Validate IPv6 address
            if value.parse::<std::net::Ipv6Addr>().is_err() {
                return Err("Invalid IPv6 address for AAAA record".to_string());
            }
        }
        "MX" => {
            // MX records should have priority and hostname
            // Format: "10 mail.example.com" or just "mail.example.com"
            if value.is_empty() {
                return Err("MX record value cannot be empty".to_string());
            }
        }
        "TXT" => {
            // TXT records can contain any text
            if value.len() > 65535 {
                return Err("TXT record value too long".to_string());
            }
        }
        _ => {
            // For other types, just ensure non-empty
            if value.is_empty() {
                return Err("Value cannot be empty".to_string());
            }
        }
    }
    Ok(())
}

/// Validate TTL value
#[allow(unused_comparisons)]
fn validate_ttl(ttl: i32) -> Result<(), String> {
    if ttl < 0 {
        return Err("TTL cannot be negative".to_string());
    }
    if ttl > 2147483647 {
        return Err("TTL value too large".to_string());
    }
    Ok(())
}

/// Validate priority value
fn validate_priority(priority: i32) -> Result<(), String> {
    if priority < 0 {
        return Err("Priority cannot be negative".to_string());
    }
    Ok(())
}

impl CreateRecordRequest {
    /// Validate the create request
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if let Err(e) = validate_name(&self.name) {
            errors.push(ValidationError {
                field: "name".to_string(),
                message: e,
            });
        }

        if let Err(e) = validate_record_type(&self.record_type) {
            errors.push(ValidationError {
                field: "record_type".to_string(),
                message: e,
            });
        }

        // Only validate value against record type if record type is valid
        if validate_record_type(&self.record_type).is_ok() {
            if let Err(e) = validate_value(&self.value, &self.record_type) {
                errors.push(ValidationError {
                    field: "value".to_string(),
                    message: e,
                });
            }
        } else if self.value.is_empty() {
            errors.push(ValidationError {
                field: "value".to_string(),
                message: "Value cannot be empty".to_string(),
            });
        }

        if let Err(e) = validate_ttl(self.ttl) {
            errors.push(ValidationError {
                field: "ttl".to_string(),
                message: e,
            });
        }

        if let Err(e) = validate_priority(self.priority) {
            errors.push(ValidationError {
                field: "priority".to_string(),
                message: e,
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { errors })
        }
    }

    /// Convert to CreateDnsRecord with normalized record type
    pub fn into_create_dns_record(self) -> CreateDnsRecord {
        CreateDnsRecord {
            name: self.name,
            record_type: self.record_type.to_uppercase(),
            value: self.value,
            ttl: self.ttl,
            priority: self.priority,
            enabled: self.enabled,
        }
    }
}

impl UpdateRecordRequest {
    /// Validate the update request
    pub fn validate(&self, existing_record_type: &str) -> Result<(), ValidationErrors> {
        let mut errors = Vec::new();

        if let Some(ref name) = self.name {
            if let Err(e) = validate_name(name) {
                errors.push(ValidationError {
                    field: "name".to_string(),
                    message: e,
                });
            }
        }

        if let Some(ref record_type) = self.record_type {
            if let Err(e) = validate_record_type(record_type) {
                errors.push(ValidationError {
                    field: "record_type".to_string(),
                    message: e,
                });
            }
        }

        // Validate value against the record type (new or existing)
        if let Some(ref value) = self.value {
            let record_type = self.record_type.as_deref().unwrap_or(existing_record_type);
            if validate_record_type(record_type).is_ok() {
                if let Err(e) = validate_value(value, record_type) {
                    errors.push(ValidationError {
                        field: "value".to_string(),
                        message: e,
                    });
                }
            }
        }

        if let Some(ttl) = self.ttl {
            if let Err(e) = validate_ttl(ttl) {
                errors.push(ValidationError {
                    field: "ttl".to_string(),
                    message: e,
                });
            }
        }

        if let Some(priority) = self.priority {
            if let Err(e) = validate_priority(priority) {
                errors.push(ValidationError {
                    field: "priority".to_string(),
                    message: e,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { errors })
        }
    }

    /// Convert to UpdateDnsRecord with normalized record type
    pub fn into_update_dns_record(self) -> UpdateDnsRecord {
        UpdateDnsRecord {
            name: self.name,
            record_type: self.record_type.map(|t| t.to_uppercase()),
            value: self.value,
            ttl: self.ttl,
            priority: self.priority,
            enabled: self.enabled,
        }
    }
}


/// List all DNS records
///
/// GET /api/records
pub async fn list_records(
    State(state): State<RecordsState>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.dns_records();
    
    let records = repo.list().await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to list records: {}", e),
        details: None,
    })?;

    Ok(Json(RecordsListResponse {
        total: records.len(),
        data: records,
    }))
}

/// Get a DNS record by ID
///
/// GET /api/records/:id
pub async fn get_record(
    State(state): State<RecordsState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.dns_records();
    
    let record = repo.get_by_id(id).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to get record: {}", e),
        details: None,
    })?;

    match record {
        Some(r) => Ok(Json(RecordResponse { data: r })),
        None => Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Record with id {} not found", id),
            details: None,
        }),
    }
}

/// Create a new DNS record
///
/// POST /api/records
pub async fn create_record(
    State(state): State<RecordsState>,
    Json(request): Json<CreateRecordRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate request
    if let Err(validation_errors) = request.validate() {
        return Err(ApiError {
            code: "BAD_REQUEST".to_string(),
            message: "Validation failed".to_string(),
            details: Some(serde_json::to_value(validation_errors).unwrap()),
        });
    }

    let repo = state.db.dns_records();
    let create_record = request.into_create_dns_record();
    
    let record = repo.create(create_record).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to create record: {}", e),
        details: None,
    })?;

    Ok((StatusCode::CREATED, Json(RecordResponse { data: record })))
}

/// Update a DNS record
///
/// PUT /api/records/:id
pub async fn update_record(
    State(state): State<RecordsState>,
    Path(id): Path<i64>,
    Json(request): Json<UpdateRecordRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.dns_records();
    
    // First check if record exists
    let existing = repo.get_by_id(id).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to get record: {}", e),
        details: None,
    })?;

    let existing = existing.ok_or_else(|| ApiError {
        code: "NOT_FOUND".to_string(),
        message: format!("Record with id {} not found", id),
        details: None,
    })?;

    // Validate request against existing record type
    if let Err(validation_errors) = request.validate(&existing.record_type) {
        return Err(ApiError {
            code: "BAD_REQUEST".to_string(),
            message: "Validation failed".to_string(),
            details: Some(serde_json::to_value(validation_errors).unwrap()),
        });
    }

    let update_record = request.into_update_dns_record();
    
    let record = repo.update(id, update_record).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to update record: {}", e),
        details: None,
    })?;

    match record {
        Some(r) => Ok(Json(RecordResponse { data: r })),
        None => Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Record with id {} not found", id),
            details: None,
        }),
    }
}

/// Delete a DNS record
///
/// DELETE /api/records/:id
pub async fn delete_record(
    State(state): State<RecordsState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state.db.dns_records();
    
    let deleted = repo.delete(id).await.map_err(|e| ApiError {
        code: "INTERNAL_ERROR".to_string(),
        message: format!("Failed to delete record: {}", e),
        details: None,
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Record with id {} not found", id),
            details: None,
        })
    }
}

/// Build the records API router
pub fn records_router(state: RecordsState) -> axum::Router {
    use axum::routing::get;
    
    axum::Router::new()
        .route("/", get(list_records).post(create_record))
        .route("/:id", get(get_record).put(update_record).delete(delete_record))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("example.com").is_ok());
        assert!(validate_name("sub.example.com").is_ok());
        assert!(validate_name("*.example.com").is_ok());
        assert!(validate_name("test-domain.com").is_ok());
        assert!(validate_name("_dmarc.example.com").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(validate_name("").is_err());
        assert!(validate_name(&"a".repeat(256)).is_err());
        assert!(validate_name("example.com!").is_err());
    }

    #[test]
    fn test_validate_record_type_valid() {
        assert!(validate_record_type("A").is_ok());
        assert!(validate_record_type("a").is_ok());
        assert!(validate_record_type("AAAA").is_ok());
        assert!(validate_record_type("CNAME").is_ok());
        assert!(validate_record_type("MX").is_ok());
        assert!(validate_record_type("TXT").is_ok());
        assert!(validate_record_type("PTR").is_ok());
        assert!(validate_record_type("NS").is_ok());
        assert!(validate_record_type("SOA").is_ok());
        assert!(validate_record_type("SRV").is_ok());
    }

    #[test]
    fn test_validate_record_type_invalid() {
        assert!(validate_record_type("INVALID").is_err());
        assert!(validate_record_type("").is_err());
    }

    #[test]
    fn test_validate_value_a_record() {
        assert!(validate_value("192.168.1.1", "A").is_ok());
        assert!(validate_value("10.0.0.1", "A").is_ok());
        assert!(validate_value("invalid", "A").is_err());
        assert!(validate_value("", "A").is_err());
    }

    #[test]
    fn test_validate_value_aaaa_record() {
        assert!(validate_value("::1", "AAAA").is_ok());
        assert!(validate_value("2001:db8::1", "AAAA").is_ok());
        assert!(validate_value("invalid", "AAAA").is_err());
    }

    #[test]
    fn test_validate_ttl() {
        assert!(validate_ttl(0).is_ok());
        assert!(validate_ttl(300).is_ok());
        assert!(validate_ttl(86400).is_ok());
        assert!(validate_ttl(-1).is_err());
    }

    #[test]
    fn test_validate_priority() {
        assert!(validate_priority(0).is_ok());
        assert!(validate_priority(10).is_ok());
        assert!(validate_priority(-1).is_err());
    }

    #[test]
    fn test_create_request_validation() {
        let valid_request = CreateRecordRequest {
            name: "example.com".to_string(),
            record_type: "A".to_string(),
            value: "192.168.1.1".to_string(),
            ttl: 300,
            priority: 0,
            enabled: true,
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = CreateRecordRequest {
            name: "".to_string(),
            record_type: "INVALID".to_string(),
            value: "".to_string(),
            ttl: -1,
            priority: -1,
            enabled: true,
        };
        let result = invalid_request.validate();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.errors.len() >= 3);
    }

    #[test]
    fn test_create_request_into_create_dns_record() {
        let request = CreateRecordRequest {
            name: "example.com".to_string(),
            record_type: "a".to_string(), // lowercase
            value: "192.168.1.1".to_string(),
            ttl: 300,
            priority: 0,
            enabled: true,
        };
        let create_record = request.into_create_dns_record();
        assert_eq!(create_record.record_type, "A"); // Should be uppercase
    }
}
