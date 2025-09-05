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
- **🛡️ Memory Safe**: Uses Rust's type system for compile-time safety
- **⚡ Performance**: Unsafe unchecked methods for zero-cost abstractions
- **🔄 Consumption**: Take ownership of loaded data with `take()`

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
preloader = "0.1.3"
tokio = { version = "1.0", features = ["full"] }
thiserror = "1.0"
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
- `load(future: impl Future<Output = T> + Send + 'static) -> ()` - Start loading data asynchronously
- `get() -> Result<&T, PreloaderError>` - Get data (blocks until ready)
- `try_get() -> Result<&T, PreloaderError>` - Try to get data (non-blocking)
- `take(self) -> Result<T, PreloaderError>` - Take ownership of data, consuming the preloader (blocks until ready)
- `is_loaded() -> bool` - Check if data is loaded and ready for immediate access
- `get_unchecked() -> &T` - Get data without checks (unsafe, panics if not ready)
- `try_get_unchecked() -> &T` - Try to get data without checks (unsafe, panics if not ready)

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum PreloaderError {
    #[error("Preloader is not loaded")]
    NotLoaded,
    #[error("Preloader is loading")]
    Loading,
}
```

### Type Aliases

```rust
type Result<T> = std::result::Result<T, PreloaderError>;
```

## Performance Characteristics

- **Memory Overhead**: Minimal - only stores the loaded data and state
- **Concurrency**: Excellent - supports unlimited concurrent readers
- **Latency**: Near-zero for cached data access
- **Thread Safety**: Full `Send + Sync` implementation
- **Atomic Operations**: Uses atomic state transitions for optimal performance
- **Zero-Cost Abstractions**: Unsafe unchecked methods for maximum performance

## Use Cases

- **Configuration Loading**: Load app configuration once, access everywhere
- **Database Connections**: Preload connection pools
- **File Caching**: Cache frequently accessed files
- **API Response Caching**: Cache external API responses
- **Resource Initialization**: Initialize heavy resources on startup
- **Lazy Loading**: Load expensive resources only when first accessed
- **High-Performance Systems**: Use unchecked methods in performance-critical paths
- **Data Processing Pipelines**: Use `take()` to consume data for transformation pipelines

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
        let content = tokio::fs::read_to_string("config.json").await.unwrap();
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

### Data Consumption Pattern

```rust
use preloader::Preloader;
use tokio;

#[tokio::main]
async fn main() {
    let preloader = Preloader::new();
    
    // Start loading data asynchronously
    preloader.load(async {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        "data to consume".to_string()
    }).await;
    
    // Take ownership of the data, consuming the preloader itself
    match preloader.take().await {
        Ok(data) => {
            println!("Consumed data: {}", data);
            // Do something with owned data
            let modified = data + " - modified";
            println!("Modified: {}", modified);
        },
        Err(e) => println!("Error: {}", e),
    }
    
    // Note: preloader is consumed and cannot be used after take()
    // The following code would not compile:
    // let result = preloader.try_get(); // Error: use of moved value
}
```

### High-Performance Access Pattern

```rust
use preloader::Preloader;
use std::sync::Arc;

struct AppState {
    config: Arc<Preloader<Config>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            config: Arc::new(Preloader::new()),
        }
    }
    
    // Safe method for general use
    async fn get_config(&self) -> Result<&Config, PreloaderError> {
        self.config.get().await
    }
    
    // High-performance method for hot paths
    fn get_config_fast(&self) -> &Config {
        // Only use when you're certain the config is loaded
        unsafe { self.config.get_unchecked() }
    }
}
```

### Lazy Resource Loading

```rust
use preloader::Preloader;
use std::sync::Arc;

struct AppState {
    config: Arc<Preloader<Config>>,
    cache: Arc<Preloader<Cache>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            config: Arc::new(Preloader::new()),
            cache: Arc::new(Preloader::new()),
        }
    }
    
    async fn get_config(&self) -> &Config {
        // Load config only when first accessed
        if self.config.try_get().is_err() {
            self.config.load(async {
                // Load configuration logic
                Config::load_from_env().await
            }).await;
        }
        self.config.get().await.unwrap()
    }
}
```

## State Transitions

The preloader follows a clear state machine:

1. **Idle** → **Start**: When `load()` is first called
2. **Start** → **Loading**: When the future is spawned
3. **Loading** → **Loaded**: When the future completes successfully
4. **Idle/Start** → **Idle**: When `load()` is called again (ignored)

## Thread Safety

The `Preloader` is designed for concurrent access:

- **Multiple Readers**: Unlimited concurrent `get()` and `try_get()` calls
- **Single Writer**: Only one `load()` call is processed
- **Atomic State**: State transitions are atomic and lock-free
- **Memory Ordering**: Uses appropriate memory ordering for performance

## Safety Considerations

### Safe Methods
- `get()` - Always safe, blocks until data is ready
- `try_get()` - Always safe, returns error if not ready

### Unsafe Methods
- `get_unchecked()` - **Unsafe**: Panics if data is not loaded
- `try_get_unchecked()` - **Unsafe**: Panics if data is not loaded

**Use unsafe methods only when you are absolutely certain the data is loaded and ready.**

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

### Development Setup

```bash
git clone https://github.com/yourusername/preloader.git
cd preloader
cargo test
cargo doc --open
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_concurrent_access
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- Uses [atomic-enum](https://docs.rs/atomic-enum/) for atomic state management
- Uses [thiserror](https://docs.rs/thiserror/) for error handling
- Inspired by modern caching patterns and concurrent programming best practices

---

**Made with ❤️ in Rust** 