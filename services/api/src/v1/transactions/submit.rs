use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    errors::{AppError, AppResult},
    extractors::DatabaseConnection,
    lib::AppState,
};

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
    DatabaseConnection(mut _db_conn): DatabaseConnection,
    Json(request): Json<SubmitTransactionRequest>,
) -> AppResult<Json<SubmitTransactionResponse>> {
    // TODO Step 1: INPUT VALIDATION
    // Example validation:
    // if request.account_id.is_empty() || request.account_id.len() > 255 {
    //     return Err(AppError::bad_request("Invalid account_id: must be 1-255 characters"));
    // }
    // if request.transaction_data.is_null() {
    //     return Err(AppError::bad_request("transaction_data cannot be null"));
    // }
    // Validate JSON size, priority range, etc.
    
    // TODO Step 2: RATE LIMITING  
    // Example rate limiting:
    // let rate_limiter = RateLimiter::new(&state.redis_pool);
    // let rate_limit_result = rate_limiter.check_rate_limit(&request.account_id, 100).await?;
    // if !rate_limit_result.allowed {
    //     return Err(AppError::too_many_requests("Rate limit exceeded"));
    // }
    // Don't forget to add rate limit headers to response!
    
    // TODO Step 3: DATABASE PERSISTENCE
    // Example database insertion:
    // let new_transaction = NewTransactionQueue {
    //     id: Uuid::new_v4(),
    //     account_id: request.account_id.clone(),
    //     transaction_data: request.transaction_data.clone(),
    //     priority: request.priority.unwrap_or(0),
    //     status: "pending".to_string(),
    //     created_at: chrono::Utc::now().naive_utc(),
    //     // ... other fields
    // };
    // let transaction = diesel::insert_into(transaction_queue::table)
    //     .values(&new_transaction)
    //     .get_result::<TransactionQueue>(&mut db_conn)
    //     .await?;
    
    // TODO Step 4: QUEUE MANAGEMENT
    // Example queue operations:
    // let queue_manager = QueueManager::new(&state.redis_pool);
    // queue_manager.add_to_queue(&transaction.id, request.priority.unwrap_or(0)).await?;
    // let queue_position = queue_manager.get_queue_position(&transaction.id).await?;
    
    // TODO Step 5: RESPONSE CALCULATION
    // let estimated_time = std::cmp::min(queue_position * 30, 3600);
    
    // TODO Step 6: Add rate limit headers to response
    // You'll need to modify the return type to include headers
    
    // Placeholder response
    Ok(Json(SubmitTransactionResponse {
        transaction_id: Uuid::new_v4(),
        queue_position: 1,
        estimated_processing_time_seconds: 30,
        status: "pending".to_string(),
    }))
}