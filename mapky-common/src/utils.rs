use tokio::sync::watch::Receiver;

/// Creates a watch channel that sends `true` on Ctrl-C for shutdown signalling.
pub fn create_shutdown_rx() -> Receiver<bool> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = shutdown_tx.send(true);
    });
    shutdown_rx
}
