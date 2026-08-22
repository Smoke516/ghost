use crate::models::{AuthStrength, HealthStatus, ServerConnection};
use crate::ssh::{ConnectionMode, ConnectionTestResult, SSHManager};
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;

/// Most simultaneous health probes. Each is a TCP connect with a timeout, so
/// they're cheap, but an unbounded fan-out over a large config would open
/// hundreds of sockets at once and trip per-process fd limits.
const MAX_CONCURRENT_CHECKS: usize = 16;

/// Health monitoring system that runs background checks.
///
/// The server list lives behind a shared lock rather than being moved into the
/// background task, so servers added or removed while Ghost is running are
/// picked up on the next cycle. (Previously the task captured a snapshot taken
/// at startup and never saw another server again.)
pub struct HealthMonitor {
    ssh_manager: Arc<RwLock<SSHManager>>,
    servers: Arc<RwLock<Vec<ServerConnection>>>,
    tx: mpsc::UnboundedSender<HealthUpdate>,
    rx: mpsc::UnboundedReceiver<HealthUpdate>,
    check_interval: Duration,
    running: Arc<RwLock<bool>>,
}

/// Health update message
#[derive(Debug, Clone)]
pub struct HealthUpdate {
    pub server_id: String,
    pub result: ConnectionTestResult,
}

impl HealthMonitor {
    pub fn new(check_interval_seconds: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            ssh_manager: Arc::new(RwLock::new(SSHManager::new())),
            servers: Arc::new(RwLock::new(Vec::new())),
            tx,
            rx,
            check_interval: Duration::from_secs(check_interval_seconds),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Replace the set of servers being monitored. Called whenever the app's
    /// connection list changes (add / edit / delete / import).
    pub async fn set_servers(&self, servers: Vec<ServerConnection>) {
        *self.servers.write().await = servers;
    }

    /// Start the health monitoring background task.
    pub async fn start(&self) -> tokio::task::JoinHandle<()> {
        *self.running.write().await = true;
        let ssh_manager = self.ssh_manager.clone();
        let servers = self.servers.clone();
        let tx = self.tx.clone();
        let check_interval = self.check_interval;
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval_timer = interval(check_interval);
            // The first tick fires immediately; skip it so startup isn't a
            // thundering herd of connects before the UI has even drawn.
            interval_timer.tick().await;

            while *running.read().await {
                interval_timer.tick().await;

                // Re-read the list every cycle so edits take effect.
                let snapshot = servers.read().await.clone();
                if snapshot.is_empty() {
                    continue;
                }

                if run_checks(&ssh_manager, snapshot, &tx).await.is_err() {
                    // Receiver dropped — the app is shutting down.
                    break;
                }
            }
        })
    }

    /// Kick off an immediate check of every server without blocking the caller.
    ///
    /// Results arrive on the same channel the periodic task uses, so the UI
    /// keeps rendering and updates land frame by frame. Returns the number of
    /// servers being checked so the caller can track progress.
    pub async fn refresh_now(&self) -> usize {
        let snapshot = self.servers.read().await.clone();
        let count = snapshot.len();
        if count == 0 {
            return 0;
        }

        let ssh_manager = self.ssh_manager.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let _ = run_checks(&ssh_manager, snapshot, &tx).await;
        });

        count
    }

    /// Stop the health monitoring
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }

    /// Drain any pending health updates (non-blocking).
    pub fn try_recv_update(&mut self) -> Option<HealthUpdate> {
        self.rx.try_recv().ok()
    }

    /// Connect to server with specific connection mode.
    /// Returns the PID of the spawned terminal process.
    pub async fn connect_to_server_with_mode(
        &self,
        server: &ServerConnection,
        mode: ConnectionMode,
    ) -> Result<u32, String> {
        let mut ssh_manager = self.ssh_manager.write().await;
        ssh_manager
            .connect_with_mode(server, mode)
            .await
            .map_err(|e| format!("Connection failed: {}", e))
    }
}

/// Probe every server with bounded concurrency, streaming each result out as
/// soon as it lands. Errors only when the receiving end has hung up.
async fn run_checks(
    ssh_manager: &Arc<RwLock<SSHManager>>,
    servers: Vec<ServerConnection>,
    tx: &mpsc::UnboundedSender<HealthUpdate>,
) -> Result<(), ()> {
    let mut pending = FuturesUnordered::new();
    let mut queue = servers.into_iter();

    // Prime the pool.
    for server in queue.by_ref().take(MAX_CONCURRENT_CHECKS) {
        pending.push(check_one(ssh_manager.clone(), server));
    }

    while let Some(update) = pending.next().await {
        if tx.send(update).is_err() {
            return Err(());
        }
        // Backfill so exactly MAX_CONCURRENT_CHECKS stay in flight.
        if let Some(server) = queue.next() {
            pending.push(check_one(ssh_manager.clone(), server));
        }
    }

    Ok(())
}

async fn check_one(ssh_manager: Arc<RwLock<SSHManager>>, server: ServerConnection) -> HealthUpdate {
    let result = {
        let manager = ssh_manager.read().await;
        manager.quick_health_check(&server).await
    }
    .unwrap_or_else(|e| ConnectionTestResult {
        status: HealthStatus::Unknown,
        auth_strength: AuthStrength::Unknown,
        latency: None,
        error_message: Some(format!("Health check error: {}", e)),
    });

    HealthUpdate {
        server_id: server.id.clone(),
        result,
    }
}
