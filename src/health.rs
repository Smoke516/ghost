use crate::models::{HealthStatus, SecurityStatus, ServerConnection};
use crate::ssh::{ConnectionMode, ConnectionTestResult, SSHManager};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;

/// Health monitoring system that runs background checks
pub struct HealthMonitor {
    ssh_manager: Arc<RwLock<SSHManager>>,
    tx: mpsc::UnboundedSender<HealthUpdate>,
    rx: Arc<RwLock<mpsc::UnboundedReceiver<HealthUpdate>>>,
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
            tx,
            rx: Arc::new(RwLock::new(rx)),
            check_interval: Duration::from_secs(check_interval_seconds),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the health monitoring background task
    pub async fn start(&self, servers: Vec<ServerConnection>) -> tokio::task::JoinHandle<()> {
        *self.running.write().await = true;
        let ssh_manager = self.ssh_manager.clone();
        let tx = self.tx.clone();
        let check_interval = self.check_interval;
        let running = self.running.clone();

        tokio::spawn(async move {
            let mut interval_timer = interval(check_interval);
            
            while *running.read().await {
                interval_timer.tick().await;
                
                // Perform health checks for all servers
                for server in &servers {
                    if !*running.read().await {
                        break;
                    }

                    let ssh_manager = ssh_manager.read().await;
                    let result = ssh_manager.quick_health_check(server).await
                        .unwrap_or_else(|e| ConnectionTestResult {
                            status: HealthStatus::Unknown,
                            security_status: SecurityStatus::Unknown,
                            latency: None,
                            error_message: Some(format!("Health check error: {}", e)),
                        });

                    let update = HealthUpdate {
                        server_id: server.id.clone(),
                        result,
                    };

                    if tx.send(update).is_err() {
                        // Channel closed, stop monitoring
                        break;
                    }
                }
            }
        })
    }

    /// Stop the health monitoring
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }

    /// Get the next health update (non-blocking)
    pub async fn try_recv_update(&self) -> Option<HealthUpdate> {
        let mut rx = self.rx.write().await;
        rx.try_recv().ok()
    }

    /// Perform immediate health check on a single server
    pub async fn check_server_now(&self, server: &ServerConnection) -> ConnectionTestResult {
        let ssh_manager = self.ssh_manager.read().await;
        ssh_manager.quick_health_check(server).await
            .unwrap_or_else(|e| ConnectionTestResult {
                status: HealthStatus::Unknown,
                security_status: SecurityStatus::Unknown,
                latency: None,
                error_message: Some(format!("Immediate check error: {}", e)),
            })
    }


    /// Connect to server interactively
    /// Returns the PID of the spawned terminal process
    pub async fn connect_to_server(&self, server: &ServerConnection) -> Result<u32, String> {
        self.connect_to_server_with_mode(server, ConnectionMode::Auto).await
    }
    
    /// Connect to server with specific connection mode
    /// Returns the PID of the spawned terminal process
    pub async fn connect_to_server_with_mode(&self, server: &ServerConnection, mode: ConnectionMode) -> Result<u32, String> {
        let mut ssh_manager = self.ssh_manager.write().await;
        ssh_manager.connect_with_mode(server, mode).await
            .map_err(|e| format!("Connection failed: {}", e))
    }

}

