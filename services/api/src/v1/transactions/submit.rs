use crate::{
    errors::{AppError, AppResult},
    extractors::DatabaseConnection,
    lib::AppState,
};
use axum::http::HeaderMap;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use diesel_async::RunQueryDsl;
use postgres_models::models::{NewTransactionQueue, TransactionQueue};
use postgres_models::schema::transaction_queue;
use redis_cache::{QueueManager, RateLimiter, MAX_PRIORITY, MIN_PRIORITY};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SubmitTransactionRequest {
    pub account_id: String,
    pub transaction_data: serde_json::Value,
    pub priority: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SubmitTransactionResponse {
    pub transaction_id: Uuid,
    pub queue_position: i64,
    pub estimated_processing_time_seconds: i64,
    pub status: String,
}

pub struct JsonWithHeaders<T> {
    pub status: StatusCode,
    pub json: T,
    pub headers: HeaderMap,
}

impl<T: Serialize> IntoResponse for JsonWithHeaders<T> {
    fn into_response(self) -> Response {
        let mut resp = (self.status, Json(self.json)).into_response();
        let headers_mut = resp.headers_mut();
        for (key, value) in self.headers.iter() {
            headers_mut.insert(key, value.clone());
        }
        resp
    }
}

impl<T> JsonWithHeaders<T> {
    pub fn new(status: StatusCode, json: T) -> Self {
        Self {
            status,
            json,
            headers: HeaderMap::new(),
        }
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }
}

/// Submit a transaction to the queue
/// 
/// This is the main endpoint that candidates need to implement.
/// It should handle high-performance transaction queuing with proper
/// rate limiting, validation, and queue management.
/// 
/// Expected Performance: <100ms p99 latency, 10k+ concurrent requests
/// 
/// TODO: Implement the following steps in order:
/// 
/// Step 1: INPUT VALIDATION (Security Critical)
/// - Validate account_id: non-empty, reasonable length (< 255 chars)
/// - Validate transaction_data: not null, reasonable size (< 1MB)
/// - Validate priority: if provided, should be reasonable range (-1000 to 1000)
/// - Return 400 Bad Request for invalid input with descriptive errors
/// 
/// Step 2: RATE LIMITING (Performance Critical)
/// - Get rate limiter from state: &state.redis_pool
/// - Use libs/redis_cache/src/rate_limiter.rs::RateLimiter::check_rate_limit()
/// - Check account-specific limits from account_rate_limits table
/// - Return 429 Too Many Requests if exceeded
/// - MUST include rate limit headers in ALL responses:
///   - X-RateLimit-Limit: requests per minute allowed
///   - X-RateLimit-Remaining: requests remaining in current window
///   - X-RateLimit-Reset: timestamp when window resets
/// 
/// Step 3: DATABASE PERSISTENCE (Reliability Critical)
/// - Create NewTransactionQueue using libs/postgres_models/src/models.rs
/// - Generate UUID for transaction_id using Uuid::new_v4()
/// - Set created_at to current UTC timestamp
/// - Set status to "pending"
/// - Insert into transaction_queue table using diesel
/// - Handle database errors gracefully (return 500 Internal Server Error)
/// 
/// Step 4: QUEUE MANAGEMENT (Business Logic Critical)
/// - Use libs/redis_cache/src/queue_manager.rs::QueueManager
/// - Add transaction to Redis queue with priority
/// - Get current queue position considering priority ordering
/// - Higher priority numbers should be processed first
/// - Use Redis sorted sets for efficient priority queue
/// 
/// Step 5: RESPONSE CALCULATION
/// - Calculate estimated_processing_time_seconds:
///   - Base time: 30 seconds per transaction
///   - Multiply by queue position ahead of current transaction
///   - Cap at reasonable maximum (e.g., 3600 seconds)
/// - Return proper JSON response with all fields
/// 
/// Step 6: ERROR HANDLING
/// - All database errors should return 500 with generic message
/// - All Redis errors should return 500 with generic message  
/// - Invalid input should return 400 with specific validation errors
/// - Rate limiting should return 429 with retry information
/// - Log all errors for debugging but don't expose internals to client
/// 
/// PERFORMANCE REQUIREMENTS:
/// - This endpoint MUST handle 10,000+ concurrent requests
/// - p99 latency MUST be under 100ms
/// - Success rate MUST be >99% under normal load
/// - Use connection pooling efficiently (don't hold connections unnecessarily)
/// - Use prepared statements for database operations
/// 
/// SECURITY REQUIREMENTS:
/// - NO authentication required (this is intentional for the exercise)
/// - Validate ALL input thoroughly
/// - Prevent JSON injection attacks
/// - Don't expose internal error details
/// - Log security-relevant events
pub async fn handler(
    State(state): State<AppState>,
    DatabaseConnection(mut db_conn): DatabaseConnection,
    Json(request): Json<SubmitTransactionRequest>,
) -> AppResult<JsonWithHeaders<SubmitTransactionResponse>> {
    // Step 1: INPUT VALIDATION
    if request.account_id.is_empty() || request.account_id.len() > 255 {
        return Err(AppError::bad_request("Invalid account_id: must be 1-255 characters"));
    }
    if request.transaction_data.is_null() {
        return Err(AppError::bad_request("transaction_data cannot be null"));
    };

    // Validate transaction_data
    let transaction_size = serde_json::to_vec(&request.transaction_data)
        .map_err(|_| AppError::bad_request("transaction_data must be valid JSON"))?
        .len();
    if transaction_size == 0 {
        return Err(AppError::bad_request("transaction_data cannot be empty"));
    }
    if transaction_size > 1024 * 1024 {
        return Err(AppError::bad_request("transaction_data too large: must be < 1MB"));
    }

    // Validate priority
    if let Some(priority) = request.priority {
        if priority < MIN_PRIORITY || priority > MAX_PRIORITY {
            return Err(AppError::bad_request("priority must be between -1000 and 1000"));
        }
    }

    // Step 2: RATE LIMITING
    let rate_limiter = RateLimiter::new(state.redis_pool.clone());
    let limit_per_minute = 100;
    let window_in_seconds = 60;

    let rate_limit_result = rate_limiter
        .check_rate_limit(&request.account_id, limit_per_minute, window_in_seconds)
        .await
        .map_err(|e| {
            AppError::internal_server_error("Failed to check rate limit")
        })?;

    let mut header_map = HeaderMap::new();
    header_map.insert("X-RateLimit-Limit", limit_per_minute.into());
    header_map.insert("X-RateLimit-Remaining", rate_limit_result.remaining.into());
    header_map.insert("X-RateLimit-Reset", rate_limit_result.reset_at.into());

    if !rate_limit_result.allowed {
        let err = AppError::too_many_requests("Rate limit exceeded")
            .with_headers(header_map.clone());
        return Err(err);
    }

    // Step 3: DATABASE PERSISTENCE
    let mut new_transaction = NewTransactionQueue::new(
        request.account_id.clone(),
        request.transaction_data.clone(),
    );
    new_transaction.priority = request.priority.unwrap_or(0);
    new_transaction.scheduled_at = Some(chrono::Utc::now());
    new_transaction.status = "pending".to_string();

    let transaction_result = diesel::insert_into(transaction_queue::table)
        .values(&new_transaction)
        .get_result::<TransactionQueue>(&mut db_conn)
        .await;

    let transaction = match transaction_result {
        Ok(tx) => tx,
        Err(e) => {
            let err = Err(AppError::internal_server_error(e.to_string()));
            return err;
        }
    };

    // Step 4: QUEUE MANAGEMENT
    let queue_manager = QueueManager::new(state.redis_pool);
    let queue_name = "tx_queue";
    let tx_data = request.transaction_data.to_string();

    let queue_position = if request.priority.is_some() {
        queue_manager
            .enqueue_with_priority(
                &queue_name,
                &tx_data,
                request.priority.unwrap()
            )
            .await
            .map_err(|err| {
                AppError::internal_server_error(format!("Queue management failed: {:#?}", err))
            })?
    } else {
        queue_manager
            .enqueue(&queue_name, &tx_data)
            .await
            .map_err(|err| {
                AppError::internal_server_error(format!("Queue management failed: {:#?}", err))
            })?
    };

    // Step 5: RESPONSE CALCULATION
    let estimated_processing_time_seconds = std::cmp::min(queue_position * 30, 3600);

    // Placeholder response
    let response_body = SubmitTransactionResponse {
        transaction_id: transaction.id,
        queue_position,
        estimated_processing_time_seconds,
        status: new_transaction.status,
    };

    // Step 6: Add rate limit headers to response
    let response = JsonWithHeaders::new(StatusCode::OK, response_body)
        .with_headers(header_map);
    Ok(response)
}