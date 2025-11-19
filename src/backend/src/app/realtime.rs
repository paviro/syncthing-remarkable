use appload_client::BackendReplier;
use tokio::task::JoinHandle;

use super::event_stream;
use super::protocol::SYSTEMD_MONITOR_INTERVAL_SECS;
use super::Backend;

impl Backend {
    pub fn ensure_realtime_updates(&mut self, functionality: &BackendReplier<Self>) {
        if !task_is_running(&self.realtime_task) {
            let config = self.config.clone();
            let replier = functionality.clone();
            self.realtime_task = Some(tokio::spawn(async move {
                event_stream::drive_syncthing_stream(replier, config).await;
            }));
        }

        if !task_is_running(&self.systemd_monitor_task) {
            let config = self.config.clone();
            let replier = functionality.clone();
            self.systemd_monitor_task = Some(tokio::spawn(async move {
                crate::systemd::monitor_service(
                    config,
                    SYSTEMD_MONITOR_INTERVAL_SECS,
                    move || {
                        let replier = replier.clone();
                        tokio::spawn(async move {
                            let mut backend = replier.backend.lock().await;
                            backend.send_status(&replier, "systemd-monitor").await;
                        });
                    },
                )
                .await;
            }));
        }
    }
}

fn task_is_running(handle: &Option<JoinHandle<()>>) -> bool {
    handle
        .as_ref()
        .map(|handle| !handle.is_finished())
        .unwrap_or(false)
}

