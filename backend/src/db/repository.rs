//! Database repositories
//!
//! CRUD operations for all database entities.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::models::*;

/// Repository for DNS records
pub struct DnsRecordRepository {
    pool: SqlitePool,
}

impl DnsRecordRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new DNS record
    pub async fn create(&self, record: CreateDnsRecord) -> Result<DnsRecord> {
        let now = Utc::now();
        let result = sqlx::query_as::<_, DnsRecord>(
            r#"
            INSERT INTO dns_records (name, record_type, value, ttl, priority, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&record.name)
        .bind(&record.record_type)
        .bind(&record.value)
        .bind(record.ttl)
        .bind(record.priority)
        .bind(record.enabled)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get a DNS record by ID
    pub async fn get_by_id(&self, id: i64) -> Result<Option<DnsRecord>> {
        let result = sqlx::query_as::<_, DnsRecord>(
            "SELECT * FROM dns_records WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get DNS records by name
    pub async fn get_by_name(&self, name: &str) -> Result<Vec<DnsRecord>> {
        let result = sqlx::query_as::<_, DnsRecord>(
            "SELECT * FROM dns_records WHERE name = ? AND enabled = TRUE",
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }


    /// Get DNS records by name and type
    pub async fn get_by_name_and_type(&self, name: &str, record_type: &str) -> Result<Vec<DnsRecord>> {
        let result = sqlx::query_as::<_, DnsRecord>(
            "SELECT * FROM dns_records WHERE name = ? AND record_type = ? AND enabled = TRUE",
        )
        .bind(name)
        .bind(record_type)
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    /// List all DNS records
    pub async fn list(&self) -> Result<Vec<DnsRecord>> {
        let result = sqlx::query_as::<_, DnsRecord>(
            "SELECT * FROM dns_records ORDER BY name, record_type",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    /// Update a DNS record
    pub async fn update(&self, id: i64, update: UpdateDnsRecord) -> Result<Option<DnsRecord>> {
        let existing = self.get_by_id(id).await?;
        if existing.is_none() {
            return Ok(None);
        }
        let existing = existing.unwrap();

        let name = update.name.unwrap_or(existing.name);
        let record_type = update.record_type.unwrap_or(existing.record_type);
        let value = update.value.unwrap_or(existing.value);
        let ttl = update.ttl.unwrap_or(existing.ttl);
        let priority = update.priority.unwrap_or(existing.priority);
        let enabled = update.enabled.unwrap_or(existing.enabled);

        let result = sqlx::query_as::<_, DnsRecord>(
            r#"
            UPDATE dns_records 
            SET name = ?, record_type = ?, value = ?, ttl = ?, priority = ?, enabled = ?, updated_at = ?
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(&name)
        .bind(&record_type)
        .bind(&value)
        .bind(ttl)
        .bind(priority)
        .bind(enabled)
        .bind(Utc::now())
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Delete a DNS record
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM dns_records WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}


/// Repository for rewrite rules
pub struct RewriteRuleRepository {
    pool: SqlitePool,
}

impl RewriteRuleRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new rewrite rule
    pub async fn create(&self, rule: CreateRewriteRule) -> Result<RewriteRule> {
        let now = Utc::now();
        let result = sqlx::query_as::<_, RewriteRule>(
            r#"
            INSERT INTO rewrite_rules (pattern, match_type, action_type, action_value, priority, enabled, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&rule.pattern)
        .bind(&rule.match_type)
        .bind(&rule.action_type)
        .bind(&rule.action_value)
        .bind(rule.priority)
        .bind(rule.enabled)
        .bind(&rule.description)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get a rewrite rule by ID
    pub async fn get_by_id(&self, id: i64) -> Result<Option<RewriteRule>> {
        let result = sqlx::query_as::<_, RewriteRule>(
            "SELECT * FROM rewrite_rules WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// List all rewrite rules ordered by priority
    pub async fn list(&self) -> Result<Vec<RewriteRule>> {
        let result = sqlx::query_as::<_, RewriteRule>(
            "SELECT * FROM rewrite_rules ORDER BY priority DESC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    /// List enabled rewrite rules ordered by priority
    pub async fn list_enabled(&self) -> Result<Vec<RewriteRule>> {
        let result = sqlx::query_as::<_, RewriteRule>(
            "SELECT * FROM rewrite_rules WHERE enabled = TRUE ORDER BY priority DESC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }


    /// Update a rewrite rule
    pub async fn update(&self, id: i64, update: UpdateRewriteRule) -> Result<Option<RewriteRule>> {
        let existing = self.get_by_id(id).await?;
        if existing.is_none() {
            return Ok(None);
        }
        let existing = existing.unwrap();

        let pattern = update.pattern.unwrap_or(existing.pattern);
        let match_type = update.match_type.unwrap_or(existing.match_type);
        let action_type = update.action_type.unwrap_or(existing.action_type);
        let action_value = update.action_value.or(existing.action_value);
        let priority = update.priority.unwrap_or(existing.priority);
        let enabled = update.enabled.unwrap_or(existing.enabled);
        let description = update.description.or(existing.description);

        let result = sqlx::query_as::<_, RewriteRule>(
            r#"
            UPDATE rewrite_rules 
            SET pattern = ?, match_type = ?, action_type = ?, action_value = ?, priority = ?, enabled = ?, description = ?, updated_at = ?
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(&pattern)
        .bind(&match_type)
        .bind(&action_type)
        .bind(&action_value)
        .bind(priority)
        .bind(enabled)
        .bind(&description)
        .bind(Utc::now())
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Delete a rewrite rule
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM rewrite_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}


/// Repository for upstream servers
pub struct UpstreamServerRepository {
    pool: SqlitePool,
}

impl UpstreamServerRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new upstream server
    pub async fn create(&self, server: CreateUpstreamServer) -> Result<UpstreamServer> {
        let now = Utc::now();
        let result = sqlx::query_as::<_, UpstreamServer>(
            r#"
            INSERT INTO upstream_servers (name, address, protocol, timeout, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&server.name)
        .bind(&server.address)
        .bind(&server.protocol)
        .bind(server.timeout)
        .bind(server.enabled)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get an upstream server by ID
    pub async fn get_by_id(&self, id: i64) -> Result<Option<UpstreamServer>> {
        let result = sqlx::query_as::<_, UpstreamServer>(
            "SELECT * FROM upstream_servers WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// List all upstream servers
    pub async fn list(&self) -> Result<Vec<UpstreamServer>> {
        let result = sqlx::query_as::<_, UpstreamServer>(
            "SELECT * FROM upstream_servers ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }

    /// List upstream servers with pagination
    pub async fn list_paged(&self, page: i64, page_size: i64) -> Result<(Vec<UpstreamServer>, i64)> {
        let offset = (page - 1) * page_size;
        
        let result = sqlx::query_as::<_, UpstreamServer>(
            "SELECT * FROM upstream_servers ORDER BY id LIMIT ? OFFSET ?",
        )
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM upstream_servers")
            .fetch_one(&self.pool)
            .await?;

        Ok((result, total.0))
    }

    /// List enabled upstream servers
    pub async fn list_enabled(&self) -> Result<Vec<UpstreamServer>> {
        let result = sqlx::query_as::<_, UpstreamServer>(
            "SELECT * FROM upstream_servers WHERE enabled = TRUE ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }


    /// Update an upstream server
    pub async fn update(&self, id: i64, update: UpdateUpstreamServer) -> Result<Option<UpstreamServer>> {
        let existing = self.get_by_id(id).await?;
        if existing.is_none() {
            return Ok(None);
        }
        let existing = existing.unwrap();

        let name = update.name.unwrap_or(existing.name);
        let address = update.address.unwrap_or(existing.address);
        let protocol = update.protocol.unwrap_or(existing.protocol);
        let timeout = update.timeout.unwrap_or(existing.timeout);
        let enabled = update.enabled.unwrap_or(existing.enabled);

        let result = sqlx::query_as::<_, UpstreamServer>(
            r#"
            UPDATE upstream_servers 
            SET name = ?, address = ?, protocol = ?, timeout = ?, enabled = ?, updated_at = ?
            WHERE id = ?
            RETURNING *
            "#,
        )
        .bind(&name)
        .bind(&address)
        .bind(&protocol)
        .bind(timeout)
        .bind(enabled)
        .bind(Utc::now())
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Delete an upstream server
    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = sqlx::query("DELETE FROM upstream_servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}


/// Repository for query logs
pub struct QueryLogRepository {
    pool: SqlitePool,
}

impl QueryLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new query log entry
    pub async fn create(&self, log: CreateQueryLog) -> Result<QueryLog> {
        let now = Utc::now();
        let result = sqlx::query_as::<_, QueryLog>(
            r#"
            INSERT INTO query_logs (client_ip, query_name, query_type, response_code, response_time, cache_hit, upstream_used, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&log.client_ip)
        .bind(&log.query_name)
        .bind(&log.query_type)
        .bind(&log.response_code)
        .bind(log.response_time)
        .bind(log.cache_hit)
        .bind(&log.upstream_used)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get a query log by ID
    pub async fn get_by_id(&self, id: i64) -> Result<Option<QueryLog>> {
        let result = sqlx::query_as::<_, QueryLog>(
            "SELECT * FROM query_logs WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// List query logs with pagination and filtering
    pub async fn list(&self, filter: QueryLogFilter) -> Result<PaginatedResult<QueryLog>> {
        let limit = filter.limit.unwrap_or(50).min(1000);
        let offset = filter.offset.unwrap_or(0);

        // For simplicity, we'll use separate queries based on filter combinations
        // This avoids complex dynamic query building with sqlx
        
        let (items, total) = if let Some(ref name) = filter.query_name {
            let pattern = format!("%{}%", name);
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM query_logs WHERE query_name LIKE ?"
            )
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await?;
            
            let items = sqlx::query_as::<_, QueryLog>(
                "SELECT * FROM query_logs WHERE query_name LIKE ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(&pattern)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
            (items, count.0)
        } else if let Some(ref qtype) = filter.query_type {
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM query_logs WHERE query_type = ?"
            )
            .bind(qtype)
            .fetch_one(&self.pool)
            .await?;
            
            let items = sqlx::query_as::<_, QueryLog>(
                "SELECT * FROM query_logs WHERE query_type = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(qtype)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
            (items, count.0)
        } else if let Some(ref ip) = filter.client_ip {
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM query_logs WHERE client_ip = ?"
            )
            .bind(ip)
            .fetch_one(&self.pool)
            .await?;
            
            let items = sqlx::query_as::<_, QueryLog>(
                "SELECT * FROM query_logs WHERE client_ip = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(ip)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
            (items, count.0)
        } else if let Some(cache_hit) = filter.cache_hit {
            let count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM query_logs WHERE cache_hit = ?"
            )
            .bind(cache_hit)
            .fetch_one(&self.pool)
            .await?;
            
            let items = sqlx::query_as::<_, QueryLog>(
                "SELECT * FROM query_logs WHERE cache_hit = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(cache_hit)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
            (items, count.0)
        } else {
            // No filters - return all
            let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM query_logs")
                .fetch_one(&self.pool)
                .await?;
            
            let items = sqlx::query_as::<_, QueryLog>(
                "SELECT * FROM query_logs ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            
            (items, count.0)
        };

        Ok(PaginatedResult {
            items,
            total,
            limit,
            offset,
        })
    }

    /// Delete old query logs (older than specified days)
    pub async fn delete_old(&self, days: i64) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM query_logs WHERE created_at < datetime('now', ? || ' days')",
        )
        .bind(-days)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get query statistics
    pub async fn get_stats(&self) -> Result<QueryStats> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM query_logs")
            .fetch_one(&self.pool)
            .await?;

        let cache_hits: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM query_logs WHERE cache_hit = TRUE")
            .fetch_one(&self.pool)
            .await?;

        let today: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM query_logs WHERE created_at >= date('now')",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(QueryStats {
            total_queries: total.0,
            cache_hits: cache_hits.0,
            queries_today: today.0,
        })
    }
}

/// Query statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStats {
    pub total_queries: i64,
    pub cache_hits: i64,
    pub queries_today: i64,
}


/// Repository for system configuration
pub struct SystemConfigRepository {
    pool: SqlitePool,
}

impl SystemConfigRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get a config value by key
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM system_config WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|r| r.0))
    }

    /// Set a config value (insert or update)
    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO system_config (key, value, updated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a config value
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM system_config WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List all config values
    pub async fn list(&self) -> Result<Vec<SystemConfig>> {
        let result = sqlx::query_as::<_, SystemConfig>(
            "SELECT * FROM system_config ORDER BY key",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::tempdir;

    async fn setup_test_db() -> Database {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        Database::new(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_dns_record_crud() {
        let db = setup_test_db().await;
        let repo = db.dns_records();

        // Create
        let record = repo.create(CreateDnsRecord {
            name: "example.com".to_string(),
            record_type: "A".to_string(),
            value: "192.168.1.1".to_string(),
            ttl: 300,
            priority: 0,
            enabled: true,
        }).await.unwrap();

        assert_eq!(record.name, "example.com");
        assert_eq!(record.record_type, "A");

        // Read
        let fetched = repo.get_by_id(record.id).await.unwrap().unwrap();
        assert_eq!(fetched.value, "192.168.1.1");

        // Update
        let updated = repo.update(record.id, UpdateDnsRecord {
            value: Some("192.168.1.2".to_string()),
            ..Default::default()
        }).await.unwrap().unwrap();
        assert_eq!(updated.value, "192.168.1.2");

        // Delete
        let deleted = repo.delete(record.id).await.unwrap();
        assert!(deleted);

        let not_found = repo.get_by_id(record.id).await.unwrap();
        assert!(not_found.is_none());
    }


    #[tokio::test]
    async fn test_rewrite_rule_crud() {
        let db = setup_test_db().await;
        let repo = db.rewrite_rules();

        // Create
        let rule = repo.create(CreateRewriteRule {
            pattern: "*.ads.example.com".to_string(),
            match_type: "wildcard".to_string(),
            action_type: "block".to_string(),
            action_value: None,
            priority: 10,
            enabled: true,
            description: Some("Block ads".to_string()),
        }).await.unwrap();

        assert_eq!(rule.pattern, "*.ads.example.com");
        assert_eq!(rule.match_type, "wildcard");

        // Read
        let fetched = repo.get_by_id(rule.id).await.unwrap().unwrap();
        assert_eq!(fetched.action_type, "block");

        // Update
        let updated = repo.update(rule.id, UpdateRewriteRule {
            priority: Some(20),
            ..Default::default()
        }).await.unwrap().unwrap();
        assert_eq!(updated.priority, 20);

        // Delete
        let deleted = repo.delete(rule.id).await.unwrap();
        assert!(deleted);
    }

    #[tokio::test]
    async fn test_upstream_server_crud() {
        let db = setup_test_db().await;
        let repo = db.upstream_servers();

        // Create
        let server = repo.create(CreateUpstreamServer {
            name: "Cloudflare".to_string(),
            address: "1.1.1.1:53".to_string(),
            protocol: "udp".to_string(),
            timeout: 5000,
            enabled: true,
        }).await.unwrap();

        assert_eq!(server.name, "Cloudflare");

        // Read
        let fetched = repo.get_by_id(server.id).await.unwrap().unwrap();
        assert_eq!(fetched.address, "1.1.1.1:53");

        // Update
        let updated = repo.update(server.id, UpdateUpstreamServer {
            timeout: Some(3000),
            ..Default::default()
        }).await.unwrap().unwrap();
        assert_eq!(updated.timeout, 3000);

        // Delete
        let deleted = repo.delete(server.id).await.unwrap();
        assert!(deleted);
    }

    #[tokio::test]
    async fn test_query_log_crud() {
        let db = setup_test_db().await;
        let repo = db.query_logs();

        // Create
        let log = repo.create(CreateQueryLog {
            client_ip: "192.168.1.100".to_string(),
            query_name: "example.com".to_string(),
            query_type: "A".to_string(),
            response_code: Some("NOERROR".to_string()),
            response_time: Some(50),
            cache_hit: false,
            upstream_used: Some("Cloudflare".to_string()),
        }).await.unwrap();

        assert_eq!(log.query_name, "example.com");

        // Read
        let fetched = repo.get_by_id(log.id).await.unwrap().unwrap();
        assert_eq!(fetched.client_ip, "192.168.1.100");

        // List with filter
        let result = repo.list(QueryLogFilter {
            query_name: Some("example".to_string()),
            ..Default::default()
        }).await.unwrap();
        assert_eq!(result.items.len(), 1);
    }

    #[tokio::test]
    async fn test_system_config_crud() {
        let db = setup_test_db().await;
        let repo = db.system_config();

        // Set
        repo.set("query_strategy", "concurrent").await.unwrap();

        // Get
        let value = repo.get("query_strategy").await.unwrap().unwrap();
        assert_eq!(value, "concurrent");

        // Update
        repo.set("query_strategy", "round_robin").await.unwrap();
        let updated = repo.get("query_strategy").await.unwrap().unwrap();
        assert_eq!(updated, "round_robin");

        // Delete
        let deleted = repo.delete("query_strategy").await.unwrap();
        assert!(deleted);

        let not_found = repo.get("query_strategy").await.unwrap();
        assert!(not_found.is_none());
    }
}


/// Server listener repository
pub struct ServerListenerRepository {
    pool: SqlitePool,
}

impl ServerListenerRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get all server listeners
    pub async fn list(&self) -> Result<Vec<ServerListener>> {
        let listeners = sqlx::query_as::<_, ServerListener>(
            "SELECT * FROM server_listeners ORDER BY protocol"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(listeners)
    }

    /// Get server listener by protocol
    pub async fn get_by_protocol(&self, protocol: &str) -> Result<Option<ServerListener>> {
        let listener = sqlx::query_as::<_, ServerListener>(
            "SELECT * FROM server_listeners WHERE protocol = ?"
        )
        .bind(protocol)
        .fetch_optional(&self.pool)
        .await?;
        Ok(listener)
    }

    /// Update server listener
    pub async fn update(&self, protocol: &str, update: UpdateServerListener) -> Result<Option<ServerListener>> {
        let existing = self.get_by_protocol(protocol).await?;
        if existing.is_none() {
            return Ok(None);
        }
        let existing = existing.unwrap();

        let enabled = update.enabled.unwrap_or(existing.enabled);
        let bind_address = update.bind_address.unwrap_or(existing.bind_address);
        let port = update.port.unwrap_or(existing.port);
        let tls_cert = update.tls_cert.or(existing.tls_cert);
        let tls_key = update.tls_key.or(existing.tls_key);

        let result = sqlx::query_as::<_, ServerListener>(
            r#"
            UPDATE server_listeners 
            SET enabled = ?, bind_address = ?, port = ?, tls_cert = ?, tls_key = ?, updated_at = CURRENT_TIMESTAMP
            WHERE protocol = ?
            RETURNING *
            "#
        )
        .bind(enabled)
        .bind(bind_address)
        .bind(port)
        .bind(tls_cert)
        .bind(tls_key)
        .bind(protocol)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Get enabled listeners
    pub async fn list_enabled(&self) -> Result<Vec<ServerListener>> {
        let listeners = sqlx::query_as::<_, ServerListener>(
            "SELECT * FROM server_listeners WHERE enabled = TRUE ORDER BY protocol"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(listeners)
    }
}
