use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{bounded, Sender};
use discord_rich_presence::activity::{Activity, ActivityType, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

pub const APP_ID: &str = "1539058341353889883";

const MIN_INTERVAL: Duration = Duration::from_secs(15);
const RECONNECT_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Status {
    pub details: String,
    pub state: String,
    pub started_at: Option<i64>,
    pub active: bool,
}

pub struct Presence {
    tx: Sender<Status>,
}

impl Presence {
    pub fn start() -> Self {
        let (tx, rx) = bounded::<Status>(16);

        std::thread::Builder::new()
            .name("nuru-discord".into())
            .spawn(move || {
                let mut client: Option<DiscordIpcClient> = None;
                let mut wanted = Status::default();
                let mut applied: Option<Status> = None;
                let mut last_apply = Instant::now() - MIN_INTERVAL;
                let mut last_connect_try = Instant::now() - RECONNECT_INTERVAL;

                loop {
                    match rx.recv_timeout(Duration::from_secs(2)) {
                        Ok(next) => {
                            wanted = next;
                            while let Ok(newer) = rx.try_recv() {
                                wanted = newer;
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                    }

                    if client.is_none() && last_connect_try.elapsed() >= RECONNECT_INTERVAL {
                        last_connect_try = Instant::now();
                        let mut c = DiscordIpcClient::new(APP_ID);
                        if c.connect().is_ok() {
                            log::info!("discord rich presence connected");
                            client = Some(c);
                            applied = None;
                        }
                    }

                    let Some(c) = client.as_mut() else { continue };

                    if applied.as_ref() == Some(&wanted) || last_apply.elapsed() < MIN_INTERVAL {
                        continue;
                    }

                    let result = if wanted.active {
                        let mut activity = Activity::new()
                            .activity_type(ActivityType::Listening)
                            .details(&wanted.details)
                            .state(&wanted.state)
                            .assets(Assets::new().large_image("nuru").large_text("Nuru"));
                        if let Some(start) = wanted.started_at {
                            activity = activity.timestamps(Timestamps::new().start(start));
                        }
                        c.set_activity(activity)
                    } else {
                        c.clear_activity()
                    };

                    match result {
                        Ok(_) => {
                            applied = Some(wanted.clone());
                            last_apply = Instant::now();
                        }
                        Err(e) => {
                            log::debug!("discord presence dropped: {e}");
                            let _ = c.close();
                            client = None;
                        }
                    }
                }
            })
            .ok();

        Self { tx }
    }

    pub fn set(&self, status: Status) {
        let _ = self.tx.try_send(status);
    }
}

pub fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
