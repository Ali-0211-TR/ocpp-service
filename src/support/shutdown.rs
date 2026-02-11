//! Graceful shutdown handling
//!
//! Provides shutdown signal coordination for all server components.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{info, warn};

/// Shutdown signal that can be cloned and shared across tasks
#[derive(Clone)]
pub struct ShutdownSignal {
    sender: broadcast::Sender<()>,
    triggered: Arc<AtomicBool>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1);
        Self {
            sender,
            triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }

    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::SeqCst)
    }

    pub fn trigger(&self) {
        if !self.triggered.swap(true, Ordering::SeqCst) {
            info!("🛑 Shutdown signal triggered");
            let _ = self.sender.send(());
        }
    }

    pub async fn wait(&self) {
        let mut rx = self.subscribe();
        let _ = rx.recv().await;
    }

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
        if self.triggered.load(Ordering::SeqCst) {
            return;
        }
        let _ = self.receiver.recv().await;
    }
}

/// Listen for OS shutdown signals (SIGTERM, SIGINT)
pub async fn listen_for_shutdown_signals(shutdown: ShutdownSignal) {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");

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
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        info!("📡 Received Ctrl+C signal");
        shutdown.trigger();
    }
}

/// Graceful shutdown coordinator
pub struct ShutdownCoordinator {
    signal: ShutdownSignal,
    timeout_secs: u64,
}

impl ShutdownCoordinator {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            signal: ShutdownSignal::new(),
            timeout_secs,
        }
    }

    pub fn signal(&self) -> ShutdownSignal {
        self.signal.clone()
    }

    pub fn start_signal_listener(&self) {
        let signal = self.signal.clone();
        tokio::spawn(async move {
            listen_for_shutdown_signals(signal).await;
        });
    }

    pub async fn wait_for_shutdown(&self) -> bool {
        self.signal.wait().await;
        info!(
            "⏳ Starting graceful shutdown (timeout: {}s)...",
            self.timeout_secs
        );
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        true
    }

    pub async fn shutdown_with_cleanup<F, Fut>(&self, cleanup: F) -> bool
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        self.signal.wait().await;
        info!(
            "⏳ Starting graceful shutdown (timeout: {}s)...",
            self.timeout_secs
        );

        match tokio::time::timeout(
            tokio::time::Duration::from_secs(self.timeout_secs),
            cleanup(),
        )
        .await
        {
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
        Self::new(30)
    }
}
