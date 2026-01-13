//! Authentication service module
//!
//! Implements JWT-based authentication for the web management interface.
//!
//! # Requirements
//!
//! - 5.1: Require user login to access management interface
//! - 5.2: Generate authentication token for correct credentials
//! - 5.3: Reject access for incorrect credentials
//! - 5.4: Read username/password from environment variables
//! - 5.5: Fall back to config file if env vars not set
//! - 5.6: Environment variables take priority over config file

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

use crate::config::ConfigManager;
use crate::error::AppError;

/// JWT secret key - in production, this should be loaded from configuration
const JWT_SECRET: &str = "dns-proxy-service-secret-key-change-in-production";

/// Token expiration time in hours
const TOKEN_EXPIRATION_HOURS: i64 = 24;

/// Login request payload
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response payload
#[derive(Debug, Clone, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: i64,
    pub token_type: String,
}

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (username)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at time (Unix timestamp)
    pub iat: i64,
}

/// Authentication service
///
/// Handles user authentication and JWT token management.
/// Reads credentials from ConfigManager which handles the priority
/// of environment variables over config file (Requirements 5.4, 5.5, 5.6).
#[derive(Clone)]
pub struct AuthService {
    config: Arc<ConfigManager>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    /// Create a new AuthService instance
    pub fn new(config: Arc<ConfigManager>) -> Self {
        Self {
            config,
            encoding_key: EncodingKey::from_secret(JWT_SECRET.as_bytes()),
            decoding_key: DecodingKey::from_secret(JWT_SECRET.as_bytes()),
        }
    }

    /// Create AuthService with a custom secret key (for testing)
    #[allow(dead_code)]
    pub fn with_secret(config: Arc<ConfigManager>, secret: &str) -> Self {
        Self {
            config,
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    /// Authenticate user and generate JWT token
    ///
    /// # Requirements
    /// - 5.2: Generate token for correct credentials
    /// - 5.3: Reject access for incorrect credentials
    pub fn login(&self, request: &LoginRequest) -> Result<LoginResponse, AppError> {
        let app_config = self.config.get();

        // Validate credentials
        if request.username != app_config.admin_username
            || request.password != app_config.admin_password
        {
            return Err(AppError::Auth("Invalid username or password".to_string()));
        }

        // Generate JWT token
        let now = Utc::now();
        let expires_at = now + Duration::hours(TOKEN_EXPIRATION_HOURS);

        let claims = Claims {
            sub: request.username.clone(),
            exp: expires_at.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::Auth(format!("Failed to generate token: {}", e)))?;

        Ok(LoginResponse {
            token,
            expires_at: expires_at.timestamp(),
            token_type: "Bearer".to_string(),
        })
    }

    /// Verify JWT token and extract claims
    ///
    /// Returns the claims if the token is valid, or an error if invalid/expired.
    pub fn verify_token(&self, token: &str) -> Result<Claims, AppError> {
        let validation = Validation::default();

        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &validation)
            .map_err(|e| AppError::Auth(format!("Invalid token: {}", e)))?;

        Ok(token_data.claims)
    }

    /// Extract token from Authorization header
    ///
    /// Expects format: "Bearer <token>"
    pub fn extract_token_from_header(auth_header: &str) -> Option<&str> {
        if auth_header.starts_with("Bearer ") {
            Some(&auth_header[7..])
        } else {
            None
        }
    }
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "FORBIDDEN" => StatusCode::FORBIDDEN,
            "BAD_REQUEST" => StatusCode::BAD_REQUEST,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(self);
        (status, body).into_response()
    }
}

/// Application state for auth middleware
#[derive(Clone)]
pub struct AuthState {
    pub auth_service: AuthService,
}

/// Login handler for the /api/auth/login endpoint
pub async fn login_handler(
    State(state): State<AuthState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    state
        .auth_service
        .login(&request)
        .map(Json)
        .map_err(|e| ApiError {
            code: "UNAUTHORIZED".to_string(),
            message: e.to_string(),
            details: None,
        })
}

/// Authentication middleware
///
/// Validates JWT token from Authorization header.
/// Returns 401 Unauthorized if token is missing or invalid.
///
/// # Requirements
/// - 5.1: Require user login to access management interface
pub async fn auth_middleware(
    State(state): State<AuthState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // Skip auth for login endpoint
    if request.uri().path() == "/api/auth/login" {
        return Ok(next.run(request).await);
    }

    // Extract Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) => AuthService::extract_token_from_header(header),
        None => None,
    };

    let token = token.ok_or_else(|| ApiError {
        code: "UNAUTHORIZED".to_string(),
        message: "Missing or invalid Authorization header".to_string(),
        details: None,
    })?;

    // Verify token
    state.auth_service.verify_token(token).map_err(|e| ApiError {
        code: "UNAUTHORIZED".to_string(),
        message: e.to_string(),
        details: None,
    })?;

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PartialConfig;
    use proptest::prelude::*;

    fn create_test_config() -> Arc<ConfigManager> {
        let config = PartialConfig {
            admin_username: Some("testuser".to_string()),
            admin_password: Some("testpass".to_string()),
            ..Default::default()
        };
        Arc::new(ConfigManager::from_configs(Some(config), None))
    }

    fn create_config_with_credentials(username: String, password: String) -> Arc<ConfigManager> {
        let config = PartialConfig {
            admin_username: Some(username),
            admin_password: Some(password),
            ..Default::default()
        };
        Arc::new(ConfigManager::from_configs(Some(config), None))
    }

    // Feature: dns-proxy-service, Property 8: 认证令牌有效性
    // *For any* 正确的用户名和密码组合，Auth_Service 生成的令牌应能通过验证；
    // *For any* 错误的凭据，应拒绝生成令牌。
    // **Validates: Requirements 5.2, 5.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 8a: For any valid credentials, the generated token should be verifiable
        /// and contain the correct username in claims
        #[test]
        fn test_valid_credentials_produce_verifiable_token(
            username in "[a-zA-Z][a-zA-Z0-9_]{2,20}",
            password in "[a-zA-Z0-9!@#$%^&*]{4,30}",
        ) {
            let config = create_config_with_credentials(username.clone(), password.clone());
            let auth_service = AuthService::new(config);

            let request = LoginRequest {
                username: username.clone(),
                password: password.clone(),
            };

            // Property: Valid credentials should always produce a token
            let login_result = auth_service.login(&request);
            prop_assert!(login_result.is_ok(),
                "Login with valid credentials should succeed for user '{}'", username);

            let login_response = login_result.unwrap();

            // Property: Generated token should not be empty
            prop_assert!(!login_response.token.is_empty(),
                "Generated token should not be empty");

            // Property: Token type should be Bearer
            prop_assert_eq!(login_response.token_type, "Bearer",
                "Token type should be 'Bearer'");

            // Property: Generated token should be verifiable
            let verify_result = auth_service.verify_token(&login_response.token);
            prop_assert!(verify_result.is_ok(),
                "Token generated for '{}' should be verifiable", username);

            // Property: Verified claims should contain the correct username
            let claims = verify_result.unwrap();
            prop_assert_eq!(claims.sub, username,
                "Token claims should contain the correct username");

            // Property: Token expiration should be in the future
            prop_assert!(claims.exp > Utc::now().timestamp(),
                "Token expiration should be in the future");

            // Property: Token issued-at should be in the past or present
            prop_assert!(claims.iat <= Utc::now().timestamp(),
                "Token issued-at should be in the past or present");
        }

        /// Property 8b: For any incorrect username (with correct password), login should fail
        #[test]
        fn test_wrong_username_rejected(
            correct_username in "[a-zA-Z][a-zA-Z0-9_]{2,20}",
            wrong_username in "[a-zA-Z][a-zA-Z0-9_]{2,20}",
            password in "[a-zA-Z0-9!@#$%^&*]{4,30}",
        ) {
            // Skip if usernames happen to be the same
            prop_assume!(correct_username != wrong_username);

            let config = create_config_with_credentials(correct_username.clone(), password.clone());
            let auth_service = AuthService::new(config);

            let request = LoginRequest {
                username: wrong_username.clone(),
                password: password.clone(),
            };

            // Property: Wrong username should always be rejected
            let result = auth_service.login(&request);
            prop_assert!(result.is_err(),
                "Login with wrong username '{}' (expected '{}') should fail",
                wrong_username, correct_username);

            // Property: Error should be an Auth error
            match result {
                Err(AppError::Auth(msg)) => {
                    prop_assert!(msg.contains("Invalid"),
                        "Error message should indicate invalid credentials");
                }
                _ => prop_assert!(false, "Expected Auth error for wrong username"),
            }
        }

        /// Property 8c: For any incorrect password (with correct username), login should fail
        #[test]
        fn test_wrong_password_rejected(
            username in "[a-zA-Z][a-zA-Z0-9_]{2,20}",
            correct_password in "[a-zA-Z0-9!@#$%^&*]{4,30}",
            wrong_password in "[a-zA-Z0-9!@#$%^&*]{4,30}",
        ) {
            // Skip if passwords happen to be the same
            prop_assume!(correct_password != wrong_password);

            let config = create_config_with_credentials(username.clone(), correct_password.clone());
            let auth_service = AuthService::new(config);

            let request = LoginRequest {
                username: username.clone(),
                password: wrong_password.clone(),
            };

            // Property: Wrong password should always be rejected
            let result = auth_service.login(&request);
            prop_assert!(result.is_err(),
                "Login with wrong password for user '{}' should fail", username);

            // Property: Error should be an Auth error
            match result {
                Err(AppError::Auth(msg)) => {
                    prop_assert!(msg.contains("Invalid"),
                        "Error message should indicate invalid credentials");
                }
                _ => prop_assert!(false, "Expected Auth error for wrong password"),
            }
        }

        /// Property 8d: Tokens generated with different secrets should not be cross-verifiable
        #[test]
        fn test_tokens_not_cross_verifiable_with_different_secrets(
            username in "[a-zA-Z][a-zA-Z0-9_]{2,20}",
            password in "[a-zA-Z0-9!@#$%^&*]{4,30}",
            secret1 in "[a-zA-Z0-9]{16,32}",
            secret2 in "[a-zA-Z0-9]{16,32}",
        ) {
            // Skip if secrets happen to be the same
            prop_assume!(secret1 != secret2);

            let config = create_config_with_credentials(username.clone(), password.clone());
            let auth_service1 = AuthService::with_secret(config.clone(), &secret1);
            let auth_service2 = AuthService::with_secret(config, &secret2);

            let request = LoginRequest {
                username: username.clone(),
                password: password.clone(),
            };

            // Generate token with service1
            let token1 = auth_service1.login(&request).unwrap().token;

            // Property: Token from service1 should NOT verify with service2
            let cross_verify_result = auth_service2.verify_token(&token1);
            prop_assert!(cross_verify_result.is_err(),
                "Token generated with secret1 should not verify with secret2");
        }

        /// Property 8e: Token verification should fail for tampered tokens
        #[test]
        fn test_tampered_token_rejected(
            username in "[a-zA-Z][a-zA-Z0-9_]{2,20}",
            password in "[a-zA-Z0-9!@#$%^&*]{4,30}",
            tamper_char in "[a-zA-Z0-9]",
        ) {
            let config = create_config_with_credentials(username.clone(), password.clone());
            let auth_service = AuthService::new(config);

            let request = LoginRequest {
                username: username.clone(),
                password: password.clone(),
            };

            let token = auth_service.login(&request).unwrap().token;

            // Tamper with the token by appending a character
            let tampered_token = format!("{}{}", token, tamper_char);

            // Property: Tampered token should not verify
            let result = auth_service.verify_token(&tampered_token);
            prop_assert!(result.is_err(),
                "Tampered token should not verify");
        }
    }

    #[test]
    fn test_login_success() {
        let config = create_test_config();
        let auth_service = AuthService::new(config);

        let request = LoginRequest {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let response = auth_service.login(&request).unwrap();
        assert!(!response.token.is_empty());
        assert_eq!(response.token_type, "Bearer");
        assert!(response.expires_at > Utc::now().timestamp());
    }

    #[test]
    fn test_login_invalid_username() {
        let config = create_test_config();
        let auth_service = AuthService::new(config);

        let request = LoginRequest {
            username: "wronguser".to_string(),
            password: "testpass".to_string(),
        };

        let result = auth_service.login(&request);
        assert!(result.is_err());
        match result {
            Err(AppError::Auth(msg)) => {
                assert!(msg.contains("Invalid username or password"));
            }
            _ => panic!("Expected Auth error"),
        }
    }

    #[test]
    fn test_login_invalid_password() {
        let config = create_test_config();
        let auth_service = AuthService::new(config);

        let request = LoginRequest {
            username: "testuser".to_string(),
            password: "wrongpass".to_string(),
        };

        let result = auth_service.login(&request);
        assert!(result.is_err());
        match result {
            Err(AppError::Auth(msg)) => {
                assert!(msg.contains("Invalid username or password"));
            }
            _ => panic!("Expected Auth error"),
        }
    }

    #[test]
    fn test_verify_valid_token() {
        let config = create_test_config();
        let auth_service = AuthService::new(config);

        let request = LoginRequest {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let login_response = auth_service.login(&request).unwrap();
        let claims = auth_service.verify_token(&login_response.token).unwrap();

        assert_eq!(claims.sub, "testuser");
        assert!(claims.exp > Utc::now().timestamp());
    }

    #[test]
    fn test_verify_invalid_token() {
        let config = create_test_config();
        let auth_service = AuthService::new(config);

        let result = auth_service.verify_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_tampered_token() {
        let config = create_test_config();
        let auth_service = AuthService::new(config);

        let request = LoginRequest {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let login_response = auth_service.login(&request).unwrap();
        // Tamper with the token
        let tampered_token = format!("{}x", login_response.token);

        let result = auth_service.verify_token(&tampered_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_token_from_header() {
        let token = AuthService::extract_token_from_header("Bearer abc123");
        assert_eq!(token, Some("abc123"));

        let token = AuthService::extract_token_from_header("abc123");
        assert_eq!(token, None);

        let token = AuthService::extract_token_from_header("Basic abc123");
        assert_eq!(token, None);
    }

    #[test]
    fn test_token_contains_correct_claims() {
        let config = create_test_config();
        let auth_service = AuthService::new(config);

        let request = LoginRequest {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let login_response = auth_service.login(&request).unwrap();
        let claims = auth_service.verify_token(&login_response.token).unwrap();

        // Verify claims structure
        assert_eq!(claims.sub, "testuser");
        assert!(claims.iat <= Utc::now().timestamp());
        assert!(claims.exp > claims.iat);
        // Token should expire in approximately 24 hours
        let expected_duration = Duration::hours(TOKEN_EXPIRATION_HOURS).num_seconds();
        let actual_duration = claims.exp - claims.iat;
        assert!((actual_duration - expected_duration).abs() < 5); // Allow 5 seconds tolerance
    }

    #[test]
    fn test_different_secrets_produce_different_tokens() {
        let config = create_test_config();
        let auth_service1 = AuthService::with_secret(config.clone(), "secret1");
        let auth_service2 = AuthService::with_secret(config, "secret2");

        let request = LoginRequest {
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let token1 = auth_service1.login(&request).unwrap().token;
        let token2 = auth_service2.login(&request).unwrap().token;

        // Tokens should be different due to different secrets
        assert_ne!(token1, token2);

        // Token from service1 should not verify with service2
        assert!(auth_service2.verify_token(&token1).is_err());
        assert!(auth_service1.verify_token(&token2).is_err());
    }

    #[test]
    fn test_credentials_from_config() {
        // Test that credentials are read from config
        let config = PartialConfig {
            admin_username: Some("customuser".to_string()),
            admin_password: Some("custompass".to_string()),
            ..Default::default()
        };
        let config_manager = Arc::new(ConfigManager::from_configs(Some(config), None));
        let auth_service = AuthService::new(config_manager);

        // Should succeed with custom credentials
        let request = LoginRequest {
            username: "customuser".to_string(),
            password: "custompass".to_string(),
        };
        assert!(auth_service.login(&request).is_ok());

        // Should fail with default credentials
        let request = LoginRequest {
            username: "admin".to_string(),
            password: "admin".to_string(),
        };
        assert!(auth_service.login(&request).is_err());
    }
}
