//! Asynchronous data preloader library
//!
//! This library provides functionality for asynchronously loading and caching data.
//! You can perform other tasks while the data is loading, and retrieve the result immediately once loading is complete.
//!
//! # Key Features
//!
//! - **Asynchronous Data Loading**: Asynchronous data loading using Future
//! - **Caching**: Once loaded data is cached in memory for reuse
//! - **Thread Safety**: Can be safely used across multiple threads
//! - **State Management**: Clear state-based behavior (Idle, Start, Loading, Loaded)
//!
//! # Usage Example
//!
//! ```rust
//! use preloader::Preloader;
//! use tokio;
//!
//! #[tokio::main]
//! async fn main() {
//!     let preloader = Preloader::new();
//!     
//!     // Start asynchronous task
//!     preloader.load(async {
//!         // Simulate time-consuming task
//!         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//!         "loaded data".to_string()
//!     }).await;
//!     
//!     // Get data (blocking)
//!     match preloader.get().await {
//!         Ok(data) => println!("Data: {}", data),
//!         Err(e) => println!("Error: {}", e),
//!     }
//!     
//!     // Get data (non-blocking)
//!     match preloader.try_get() {
//!         Ok(data) => println!("Immediate data: {}", data),
//!         Err(e) => println!("Not ready yet: {}", e),
//!     }
//! }
//! ```
//!
//! # Module Structure
//!
//! - [`Preloader`]: Main preloader struct

mod preloader;

pub use preloader::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_new_preloader() {
        let preloader = Preloader::<String>::new();
        assert!(matches!(
            preloader.try_get(),
            Err(PreloaderError::NotLoaded)
        ));
    }

    #[tokio::test]
    async fn test_load_and_get() {
        let preloader = Preloader::new();

        // Start loading
        preloader
            .load(async {
                sleep(Duration::from_millis(10)).await;
                "test data".to_string()
            })
            .await;

        // Get data
        let result = preloader.get().await;
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), "test data");
    }

    #[tokio::test]
    async fn test_try_get_before_load() {
        let preloader = Preloader::<String>::new();

        // Try before loading
        let result = preloader.try_get();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PreloaderError::NotLoaded));
    }

    #[tokio::test]
    async fn test_try_get_while_loading() {
        let preloader = Preloader::new();

        // Start loading (long task)
        preloader
            .load(async {
                sleep(Duration::from_millis(100)).await;
                "slow data".to_string()
            })
            .await;

        // Try while loading
        let result = preloader.try_get();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PreloaderError::Loading));
    }

    #[tokio::test]
    async fn test_try_get_after_load() {
        let preloader = Preloader::new();

        // Start loading
        preloader
            .load(async {
                sleep(Duration::from_millis(10)).await;
                42
            })
            .await;

        // Wait for completion
        preloader.get().await.unwrap();

        // Get immediately after completion
        let result = preloader.try_get();
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_multiple_load_calls() {
        let preloader = Preloader::new();

        // First load
        preloader
            .load(async {
                sleep(Duration::from_millis(10)).await;
                "first".to_string()
            })
            .await;

        // Second load (should be ignored)
        preloader
            .load(async {
                sleep(Duration::from_millis(10)).await;
                "second".to_string()
            })
            .await;

        // Check result
        let result = preloader.get().await;
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), "first");
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let preloader = Arc::new(Preloader::new());

        // Start loading
        preloader
            .load(async {
                sleep(Duration::from_millis(50)).await;
                "concurrent access test".to_string()
            })
            .await;

        // Concurrent access from multiple tasks
        let mut handles = vec![];
        for i in 0..5 {
            let preloader = Arc::clone(&preloader);
            handles.push(tokio::spawn(async move {
                let result = preloader.get().await;
                (i, result.map(|s| s.to_string()))
            }));
        }

        // Collect all results
        let results = futures::future::join_all(handles).await;

        for result in results {
            let (i, data_result) = result.unwrap();
            assert!(data_result.is_ok(), "Task {} failed", i);
            assert_eq!(data_result.unwrap(), "concurrent access test");
        }
    }

    #[tokio::test]
    async fn test_error_handling() {
        let preloader = Preloader::new();

        // Future that causes an error
        preloader
            .load(async {
                sleep(Duration::from_millis(10)).await;
                panic!("intentional panic");
            })
            .await;

        // Check error handling
        let result = preloader.get().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_different_data_types() {
        // String type
        let string_preloader = Preloader::new();
        string_preloader.load(async { "string".to_string() }).await;
        let result = string_preloader.get().await;
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), "string");

        // i32 type
        let int_preloader = Preloader::new();
        int_preloader.load(async { 123 }).await;
        let result = int_preloader.get().await;
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), 123);

        // Vec type
        let vec_preloader = Preloader::new();
        vec_preloader.load(async { vec![1, 2, 3] }).await;
        let result = vec_preloader.get().await;
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_state_transitions() {
        let preloader = Preloader::new();

        // Initial state: Idle
        assert!(matches!(
            preloader.try_get(),
            Err(PreloaderError::NotLoaded)
        ));

        // Start loading: Start -> Loading
        preloader
            .load(async {
                sleep(Duration::from_millis(50)).await;
                "state test".to_string()
            })
            .await;

        // Loading: Loading
        assert!(matches!(preloader.try_get(), Err(PreloaderError::Loading)));

        // After completion: Loaded
        preloader.get().await.unwrap();
        let result = preloader.try_get();
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), "state test");
    }

    #[tokio::test]
    async fn test_reuse_after_load() {
        let preloader = Preloader::new();

        // First load
        preloader.load(async { "reuse test".to_string() }).await;
        let result1 = preloader.get().await;
        assert!(result1.is_ok());
        assert_eq!(*result1.unwrap(), "reuse test");

        // Second access (using cached value)
        let result2 = preloader.get().await;
        assert!(result2.is_ok());
        assert_eq!(*result2.unwrap(), "reuse test");

        // Also accessible via try_get
        let result3 = preloader.try_get();
        assert!(result3.is_ok());
        assert_eq!(*result3.unwrap(), "reuse test");
    }

    #[tokio::test]
    async fn test_immediate_load() {
        let preloader = Preloader::new();

        // Future that completes immediately
        preloader.load(async { "immediate data".to_string() }).await;

        // Get value first with get()
        let _ = preloader.get().await;
        // After that, try_get should always return Ok
        let result = preloader.try_get();
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), "immediate data");
    }

    #[tokio::test]
    async fn test_multiple_concurrent_loads() {
        let preloader = Arc::new(Preloader::new());

        // Multiple load calls (only the first should execute)
        let mut handles = vec![];
        for i in 0..3 {
            let preloader = Arc::clone(&preloader);
            let i = i; // Move i into closure
            handles.push(tokio::spawn(async move {
                preloader
                    .load(async move {
                        sleep(Duration::from_millis(50)).await;
                        format!("data {}", i)
                    })
                    .await;
                i
            }));
        }

        // Wait for all load calls to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Check result (only first data should be loaded)
        let result = preloader.get().await;
        assert!(result.is_ok());
        // Result from the first task that started
        let data = result.unwrap();
        assert!(data.starts_with("data "));
    }

    #[tokio::test]
    async fn test_take_after_load() {
        let preloader = Preloader::new();

        // Start loading
        preloader
            .load(async {
                sleep(Duration::from_millis(10)).await;
                "take test data".to_string()
            })
            .await;

        // Take data, consuming the preloader
        let result = preloader.take().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "take test data");
        
        // Note: preloader is consumed and cannot be used after take()
    }

    #[tokio::test]
    async fn test_take_before_load() {
        let preloader = Preloader::<String>::new();

        // Try to take before loading, consuming the preloader
        let result = preloader.take().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PreloaderError::NotLoaded));
        
        // Note: preloader is consumed and cannot be used after take()
    }

    #[tokio::test]
    async fn test_take_while_loading() {
        let preloader = Preloader::new();

        // Start loading (long task)
        preloader
            .load(async {
                sleep(Duration::from_millis(100)).await;
                "slow data for take".to_string()
            })
            .await;

        // Take data, consuming the preloader
        let result = preloader.take().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "slow data for take");
        
        // Note: preloader is consumed and cannot be used after take()
    }

    #[tokio::test]
    async fn test_is_loaded() {
        let preloader = Preloader::new();
        
        // Initially not loaded
        assert!(!preloader.is_loaded());
        
        // Start loading
        preloader
            .load(async {
                sleep(Duration::from_millis(10)).await;
                "loaded data".to_string()
            })
            .await;
        
        // Still not loaded immediately after starting
        assert!(!preloader.is_loaded());
        
        // Wait for completion
        preloader.get().await.unwrap();
        
        // Now it should be loaded
        assert!(preloader.is_loaded());
    }
}
