//! Asynchronous data preloader module
//!
//! This module provides the `Preloader` struct for asynchronously loading and caching data.
//! You can perform other tasks while the data is loading, and retrieve the result immediately once loading is complete.

use std::{cell::UnsafeCell, future::Future, sync::atomic::Ordering};

use atomic_enum::atomic_enum;
use tokio::sync::{
    oneshot::{self, Receiver},
    Mutex,
};

// preloader error define
#[derive(Debug, thiserror::Error)]
pub enum PreloaderError {
    #[error("Preloader is not loaded")]
    NotLoaded,
    #[error("Preloader is loading")]
    Loading,
}

type Result<T> = std::result::Result<T, PreloaderError>;

/// Enum representing the current state of the preloader
#[atomic_enum]
enum PreloaderState {
    /// Initial state - loading has not started yet
    Idle,
    /// Start state - loading process has started
    Start,
    /// Loading state - data is being loaded asynchronously
    Loading,
    /// Loaded state - data has been successfully loaded and is available
    Loaded,
}

/// Asynchronous data preloader
///
/// `Preloader` is a struct for asynchronously loading and caching data.
/// Once data loading is complete, the result is returned immediately, and the original future is executed only once even if called multiple times.
///
/// # Example
///
/// ```rust
/// use preloader::Preloader;
/// let preloader: Preloader<String> = Preloader::new();
/// ```
///
/// # Thread Safety
///
/// `Preloader` implements `Send` and `Sync`, so it can be safely used across multiple threads.
///
/// # Generic Type
///
/// - `T`: The type of data to load. Must satisfy `Send + 'static`.
pub struct Preloader<T: Send + 'static> {
    /// Current state of the preloader
    state: AtomicPreloaderState,
    /// Handle for the asynchronous task
    handle: Mutex<Option<Receiver<T>>>,
    /// Cell storing the loaded data
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Send + 'static> Send for Preloader<T> {}
unsafe impl<T: Send + 'static> Sync for Preloader<T> {}

impl<T: Send + 'static> Preloader<T> {
    /// Creates a new `Preloader` instance.
    ///
    /// # Returns
    ///
    /// A new `Preloader` instance in the initial `Idle` state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use preloader::Preloader;
    /// let preloader: Preloader<String> = Preloader::new();
    /// ```
    pub fn new() -> Self {
        Self {
            state: AtomicPreloaderState::new(PreloaderState::Idle),
            handle: Mutex::new(None),
            value: UnsafeCell::new(None),
        }
    }

    /// Starts an asynchronous task to load data.
    ///
    /// This method can only be called in the `Idle` state. If loading is already in progress or completed,
    /// it does nothing and returns immediately.
    ///
    /// # Parameters
    ///
    /// - `future`: The asynchronous task to execute. Must implement `Future<Output = T> + Send + 'static`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use preloader::Preloader;
    /// use tokio;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let preloader = Preloader::new();
    ///     preloader.load(async {
    ///         // Simulate a time-consuming task
    ///         tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    ///         42
    ///     }).await;
    /// }
    /// ```
    pub async fn load(&self, future: impl Future<Output = T> + Send + 'static) {
        let Ok(PreloaderState::Idle) = self.state.compare_exchange(
            PreloaderState::Idle,
            PreloaderState::Start,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) else {
            return;
        };

        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let value = future.await;
            _ = tx.send(value);
        });

        self.set_handle(rx).await;
    }

    /// Retrieves the loaded data.
    ///
    /// Returns an error if the data is not yet loaded.
    /// If the data is still loading, waits until loading is complete.
    ///
    /// # Returns
    ///
    /// - `Ok(&T)`: If the data was successfully loaded
    /// - `Err(String)`: If the data is not loaded or an error occurred during loading
    ///
    /// # Example
    ///
    /// ```rust
    /// use preloader::Preloader;
    /// use tokio;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let preloader = Preloader::new();
    ///     // Start loading first
    ///     preloader.load(async { "data".to_string() }).await;
    ///     // Retrieve data
    ///     match preloader.get().await {
    ///         Ok(data) => println!("Loaded data: {}", data),
    ///         Err(e) => println!("Error: {}", e),
    ///     }
    /// }
    /// ```
    pub async fn get(&self) -> Result<&T> {
        match self.state.load(Ordering::Relaxed) {
            PreloaderState::Idle | PreloaderState::Start => {
                return Err(PreloaderError::NotLoaded);
            }
            PreloaderState::Loading => {
                let mut handle = self.handle.lock().await;
                if let Some(handle) = handle.take() {
                    let value = handle.await.map_err(|_| PreloaderError::Loading)?;
                    self.set_value(value);
                    return Ok(self.get_value());
                } else {
                    // If handle is already None, just return the value
                    return Ok(self.get_value());
                }
            }
            PreloaderState::Loaded => {
                return Ok(self.get_value());
            }
        }
    }

    /// Attempts to retrieve the loaded data immediately.
    ///
    /// Unlike `get()`, this method does not block. If the data is not yet loaded or is still loading, returns an error immediately.
    ///
    /// # Returns
    ///
    /// - `Ok(&T)`: If the data was successfully loaded
    /// - `Err(String)`: If the data is not loaded or is still loading
    ///
    /// # Example
    ///
    /// ```rust
    /// use preloader::Preloader;
    /// use tokio;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let preloader = Preloader::new();
    ///     // Try before loading
    ///     match preloader.try_get() {
    ///         Ok(data) => println!("Data: {}", data),
    ///         Err(e) => println!("Not loaded yet: {}", e),
    ///     }
    ///     // Start loading
    ///     preloader.load(async { "data".to_string() }).await;
    ///     // Try while loading
    ///     match preloader.try_get() {
    ///         Ok(data) => println!("Data: {}", data),
    ///         Err(e) => println!("Still loading: {}", e),
    ///     }
    /// }
    /// ```
    pub fn try_get(&self) -> Result<&T> {
        match self.state.load(Ordering::Relaxed) {
            PreloaderState::Idle | PreloaderState::Start => {
                return Err(PreloaderError::NotLoaded);
            }
            PreloaderState::Loading => {
                let mut handle = self
                    .handle
                    .try_lock()
                    .map_err(|_| PreloaderError::Loading)?;

                if let Some(handle) = handle.as_mut() {
                    let value = handle.try_recv().map_err(|_| PreloaderError::Loading)?;
                    self.set_value(value);
                }
                return Ok(self.get_value());
            }
            PreloaderState::Loaded => {
                return Ok(self.get_value());
            }
        }
    }

    /// Sets the handle for the asynchronous task and changes the state to `Loading`.
    ///
    /// # Parameters
    ///
    /// - `handle`: Receiver for the asynchronous task
    #[inline]
    async fn set_handle(&self, handle: Receiver<T>) {
        *self.handle.lock().await = Some(handle);
        self.state.store(PreloaderState::Loading, Ordering::Release);
    }

    /// Safely retrieves the stored value.
    ///
    /// # Returns
    ///
    /// Reference to the stored value
    ///
    /// # Safety
    ///
    /// This method should only be called in the `Loaded` state, and the value is guaranteed to exist.
    #[inline]
    fn get_value(&self) -> &T {
        unsafe { &*self.value.get() }.as_ref().unwrap()
    }

    /// Stores the value and changes the state to `Loaded`.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to store
    #[inline]
    fn set_value(&self, value: T) {
        unsafe { *self.value.get() = Some(value) };
        // Set handle to None to prevent duplicate receiving
        if let Ok(mut handle) = self.handle.try_lock() {
            *handle = None;
        }
        self.state.store(PreloaderState::Loaded, Ordering::Release);
    }
}
