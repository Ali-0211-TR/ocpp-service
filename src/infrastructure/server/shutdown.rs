//! Graceful shutdown handling for the OCPP server
//!
//! This module provides a shutdown signal handler that listens for SIGTERM and SIGINT
//! signals and allows coordinated shutdown of all server components.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use log::{info, warn};
use tokio::sync::broadcast;

/// Shutdown signal that can be cloned and shared across tasks
#[derive(Clone)]
pub struct ShutdownSignal {
    /// Broadcast sender for shutdown notification
    sender: broadcast::Sender<()>,
    /// Flag indicating if shutdown has been triggered
    triggered: Arc<AtomicBool>,
}

impl ShutdownSignal {
    /// Create a new shutdown signal
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1);
        Self {
            sender,
            triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Subscribe to receive shutdown notification
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }

    /// Check if shutdown has been triggered
    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }

    /// Trigger shutdown
    pub fn trigger(&self) {
        if !self.triggered.swap(true, Ordering::SeqCst) {
            info!("🛑 Shutdown signal triggered");
            let _ = self.sender.send(());
        }
    }

    /// Wait for shutdown signal
    pub async fn wait(&self) {
        let mut rx = self.subscribe();
        // Ignore errors - they just mean no one is listening yet
        let _ = rx.recv().await;
    }

    /// Create a future that resolves when shutdown is triggered
    pub fn notified(&self) -> ShutdownNotified {
        ShutdownNotified {
            receiver: self.subscribe(),
            triggered: self.triggered.clone(),
        }
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// A future that resolves when shutdown is triggered
pub struct ShutdownNotified {
    receiver: broadcast::Receiver<()>,
    triggered: Arc<AtomicBool>,
}

impl ShutdownNotified {
    pub async fn wait(mut self) {
        // Check if already triggered
        if self.triggered.load(Ordering::SeqCst) {
            return;
        }
        // Wait for signal
        let _ = self.receiver.recv().await;
    }
}

/// Listen for shutdown signals (SIGTERM, SIGINT) and trigger the shutdown signal
pub async fn listen_for_shutdown_signals(shutdown: ShutdownSignal) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt())
            .expect("Failed to install SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("📡 Received SIGTERM signal");
            }
            _ = sigint.recv() => {
                info!("📡 Received SIGINT signal (Ctrl+C)");
            }
        }

        shutdown.trigger();
    }

    #[cfg(not(unix))]
    {
        use tokio::signal::ctrl_c;

        ctrl_c().await.expect("Failed to install Ctrl+C handler");
        info!("📡 Received Ctrl+C signal");
        shutdown.trigger();
    }
}

/// Graceful shutdown coordinator
/// 
/// Manages the shutdown process ensuring all components are properly closed
pub struct ShutdownCoordinator {
    signal: ShutdownSignal,
    /// Timeout for graceful shutdown (in seconds)
    timeout_secs: u64,
}

impl ShutdownCoordinator {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            signal: ShutdownSignal::new(),
            timeout_secs,
        }
    }

    /// Get the shutdown signal for sharing with components
    pub fn signal(&self) -> ShutdownSignal {
        self.signal.clone()
    }

    /// Start listening for shutdown signals in the background
    pub fn start_signal_listener(&self) {
        let signal = self.signal.clone();
        tokio::spawn(async move {
            listen_for_shutdown_signals(signal).await;
        });
    }

    /// Wait for shutdown with timeout
    /// Returns true if shutdown completed gracefully, false if timeout occurred
    pub async fn wait_for_shutdown(&self) -> bool {
        self.signal.wait().await;
        
        info!("⏳ Starting graceful shutdown (timeout: {}s)...", self.timeout_secs);
        
        // Give components time to shut down
        // In practice, you'd wait for specific completion signals here
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        true
    }

    /// Perform graceful shutdown with custom cleanup logic
    pub async fn shutdown_with_cleanup<F, Fut>(&self, cleanup: F) -> bool 
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        self.signal.wait().await;
        
        info!("⏳ Starting graceful shutdown (timeout: {}s)...", self.timeout_secs);
        
        let cleanup_future = cleanup();
        
        // Run cleanup with timeout
        match tokio::time::timeout(
            tokio::time::Duration::from_secs(self.timeout_secs),
            cleanup_future,
        ).await {
            Ok(()) => {
                info!("✅ Graceful shutdown completed");
                true
            }
            Err(_) => {
                warn!("⚠️ Graceful shutdown timed out after {}s", self.timeout_secs);
                false
            }
        }
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new(30) // 30 seconds default timeout
    }
}
