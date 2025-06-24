# Preloader

[![Crates.io](https://img.shields.io/crates/v/preloader)](https://crates.io/crates/preloader)
[![Documentation](https://docs.rs/preloader/badge.svg)](https://docs.rs/preloader)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance asynchronous data preloader library for Rust that provides efficient caching and concurrent access patterns.

## Features

- **🚀 Asynchronous Loading**: Load data asynchronously using Rust's `Future` trait
- **💾 Smart Caching**: Once loaded, data is cached in memory for instant access
- **🔒 Thread Safe**: Safe concurrent access across multiple threads
- **📊 State Management**: Clear state-based behavior (Idle, Start, Loading, Loaded)
- **⚡ Non-blocking**: Optional non-blocking data retrieval with `try_get()`
- **🔄 Idempotent**: Multiple load calls are safely ignored after the first one

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
preloader = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use preloader::Preloader;
use tokio;

#[tokio::main]
async fn main() {
    let preloader = Preloader::new();
    
    // Start loading data asynchronously
    preloader.load(async {
        // Simulate expensive operation (network request, file I/O, etc.)
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        "expensive data".to_string()
    }).await;
    
    // Block until data is ready
    match preloader.get().await {
        Ok(data) => println!("Data loaded: {}", data),
        Err(e) => println!("Error: {}", e),
    }
    
    // Non-blocking access (returns immediately if ready)
    match preloader.try_get() {
        Ok(data) => println!("Immediate access: {}", data),
        Err(e) => println!("Not ready yet: {}", e),
    }
}
```

### Advanced Usage

```rust
use preloader::Preloader;
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() {
    let preloader = Arc::new(Preloader::new());
    
    // Start loading
    preloader.load(async {
        // Simulate database query
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        vec![1, 2, 3, 4, 5]
    }).await;

    // Multiple concurrent consumers
    let mut handles = vec![];
    for i in 0..5 {
        let preloader = Arc::clone(&preloader);
        handles.push(tokio::spawn(async move {
            let data = preloader.get().await.unwrap();
            println!("Consumer {} got: {:?}", i, data);
        }));
    }

    // Wait for all consumers
    for handle in handles {
        handle.await.unwrap();
    }
}
```

## API Reference

### `Preloader<T>`

The main preloader struct that handles asynchronous data loading and caching.

#### Methods

- `new() -> Preloader<T>` - Create a new preloader instance
- `load(future) -> ()` - Start loading data asynchronously
- `get() -> Result<&T, PreloaderError>` - Get data (blocks until ready)
- `try_get() -> Result<&T, PreloaderError>` - Try to get data (non-blocking)

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum PreloaderError {
    #[error("Preloader is not loaded")]
    NotLoaded,
    #[error("Preloader is loading")]
    Loading,
    #[error("Failed to receive data: {0}")]
    ReceiveError(String),
}
```

## Performance Characteristics

- **Memory Overhead**: Minimal - only stores the loaded data and state
- **Concurrency**: Excellent - supports unlimited concurrent readers
- **Latency**: Near-zero for cached data access
- **Thread Safety**: Full `Send + Sync` implementation

## Use Cases

- **Configuration Loading**: Load app configuration once, access everywhere
- **Database Connections**: Preload connection pools
- **File Caching**: Cache frequently accessed files
- **API Response Caching**: Cache external API responses
- **Resource Initialization**: Initialize heavy resources on startup

## Examples

### Configuration Loading

```rust
use preloader::Preloader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    database_url: String,
    api_key: String,
    port: u16,
}

async fn load_config() -> Config {
    let preloader = Preloader::new();
    
    preloader.load(async {
        // Load from file or environment
        tokio::fs::read_to_string("config.json").await.unwrap();
        serde_json::from_str(&content).unwrap()
    }).await;
    
    preloader.get().await.unwrap().clone()
}
```

### Database Connection Pool

```rust
use preloader::Preloader;
use sqlx::PgPool;

async fn create_connection_pool() -> PgPool {
    let preloader = Preloader::new();
    
    preloader.load(async {
        PgPool::connect("postgresql://user:pass@localhost/db").await.unwrap()
    }).await;
    
    preloader.get().await.unwrap().clone()
}
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup

```bash
git clone https://github.com/yourusername/preloader.git
cd preloader
cargo test
cargo doc --open
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- Uses [thiserror](https://docs.rs/thiserror/) for error handling
- Inspired by modern caching patterns and concurrent programming best practices

---

**Made with ❤️ in Rust** 