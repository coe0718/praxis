//! Graceful shutdown — handle SIGTERM/SIGINT with proper cleanup.
//!
//! Coordinates shutdown across all components with configurable grace period.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Shared shutdown flag.
#[derive(Clone)]
pub struct ShutdownFlag(Arc<AtomicBool>);

impl ShutdownFlag {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Check if shutdown has been requested.
    pub fn is_shutting_down(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// Request shutdown.
    pub fn request_shutdown(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl Default for ShutdownFlag {
    fn default() -> Self {
        Self::new()
    }
}

/// Callback for cleanup during shutdown.
pub trait ShutdownHook: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self) -> Result<(), String>;
}

/// Boxed shutdown hook.
pub type BoxedHook = Box<dyn ShutdownHook>;

/// Graceful shutdown coordinator.
pub struct GracefulShutdown {
    flag: ShutdownFlag,
    hooks: Vec<BoxedHook>,
    grace_period: Duration,
}

impl GracefulShutdown {
    pub fn new(grace_period: Duration) -> Self {
        Self {
            flag: ShutdownFlag::new(),
            hooks: Vec::new(),
            grace_period,
        }
    }

    /// Get the shutdown flag.
    pub fn flag(&self) -> &ShutdownFlag {
        &self.flag
    }

    /// Register a cleanup hook.
    pub fn register(&mut self, hook: BoxedHook) {
        self.hooks.push(hook);
    }

    /// Register a simple closure as a hook.
    pub fn register_closure<F>(&mut self, name: &str, f: F)
    where
        F: Fn() -> Result<(), String> + Send + Sync + 'static,
    {
        struct ClosureHook<F> {
            name: String,
            f: F,
        }
        impl<F: Fn() -> Result<(), String> + Send + Sync> ShutdownHook for ClosureHook<F> {
            fn name(&self) -> &str {
                &self.name
            }
            fn run(&self) -> Result<(), String> {
                (self.f)()
            }
        }
        self.register(Box::new(ClosureHook { name: name.to_string(), f }));
    }

    /// Execute all hooks in reverse order.
    pub fn run_hooks(&self) -> Vec<(String, Result<(), String>)> {
        let mut results = Vec::new();
        for hook in self.hooks.iter().rev() {
            log::info!("shutdown: running hook '{}'", hook.name());
            match hook.run() {
                Ok(()) => {
                    log::info!("shutdown: hook '{}' completed", hook.name());
                }
                Err(e) => {
                    log::error!("shutdown: hook '{}' failed: {e}", hook.name());
                    results.push((hook.name().to_string(), Err(e)));
                }
            }
        }
        results
    }

    /// Get grace period.
    pub fn grace_period(&self) -> Duration {
        self.grace_period
    }

    /// Wait for shutdown signal.
    /// NOTE: Signal handling is platform-specific. On most platforms this is a no-op.
    /// Use with tokio::signal::ctrl_c() in async contexts, or hook into your init system.
    pub fn wait_for_signal(&self) {
        // Platform-agnostic stub: override this method in your binary
        // to integrate with your specific signal handling (tokio, ctrlc crate, etc.)
        log::info!("shutdown: waiting for shutdown signal (not implemented in library)");
        // In practice, you'd integrate with:
        // - tokio: tokio::signal::ctrl_c().await
        // - ctrlc crate: ctrlc::set_handler(...)
        // - systemd: sd_notify(...)
        // For now, this is a placeholder that does nothing.
    }
}

impl Default for GracefulShutdown {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

/// Simple empty shutdown hook for no-op.
pub struct NoOpHook;

impl ShutdownHook for NoOpHook {
    fn name(&self) -> &str {
        "no-op"
    }
    fn run(&self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_shutdown_flag() {
        let flag = ShutdownFlag::new();
        assert!(!flag.is_shutting_down());

        flag.request_shutdown();
        assert!(flag.is_shutting_down());
    }

    #[test]
    fn test_shutdown_flag_clone() {
        let flag = ShutdownFlag::new();
        let clone = flag.clone();

        flag.request_shutdown();
        assert!(clone.is_shutting_down());
    }

    #[test]
    fn test_graceful_shutdown_hooks() {
        let mut shutdown = GracefulShutdown::new(Duration::from_secs(10));

        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();

        shutdown.register_closure("test_hook", move || {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        });

        shutdown.run_hooks();
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_hook_order_reversed() {
        let mut shutdown = GracefulShutdown::new(Duration::from_secs(10));

        let order = std::sync::Arc::new(Mutex::new(Vec::new()));
        let order_clone = order.clone();

        shutdown.register_closure("first", move || {
            order_clone.lock().unwrap().push("first");
            Ok(())
        });

        let order2 = order.clone();
        shutdown.register_closure("second", move || {
            order2.lock().unwrap().push("second");
            Ok(())
        });

        shutdown.run_hooks();

        let result = order.lock().unwrap();
        // Hooks run in reverse order
        assert_eq!(*result, vec!["second", "first"]);
    }

    #[test]
    fn test_hook_failure_recorded() {
        let mut shutdown = GracefulShutdown::new(Duration::from_secs(10));

        shutdown.register_closure("failing", || Err("test error".to_string()));

        let results = shutdown.run_hooks();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "failing");
        assert!(results[0].1.is_err());
    }

    #[test]
    fn test_grace_period_default() {
        let shutdown = GracefulShutdown::default();
        assert_eq!(shutdown.grace_period(), Duration::from_secs(30));
    }
}
