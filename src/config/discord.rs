use serde::{Serialize, Deserialize};
use async_timer::timer::{SyncTimer, Timer, SyncPlatform, new_sync_timer};

use std::io;
use std::collections::HashSet;

use super::FileSystemLoad;
use crate::constants::CONFIG_UPDATE_INTERVAL;

pub const DISCORD_TOKEN: &str = env!("JEANNE_DISCORD_TOKEN");

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Channels {
    pub welcome: u64,
    pub naze: HashSet<u64>,
    pub bisokuzenshin: HashSet<u64>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub channels: Channels,
    pub owner: u64,
}

impl FileSystemLoad for DiscordConfig {
    const NAME: &'static str = "jeanne.discord.bincode";
}

pub struct Discord {
    inner: parking_lot::RwLock<(DiscordConfig, SyncPlatform)>,
}

impl Discord {
    pub fn new() -> io::Result<Self> {
        DiscordConfig::load().map(|config| {
            let mut timer = new_sync_timer(CONFIG_UPDATE_INTERVAL);

            timer.init(|state| state.register(on_config_update as fn()));
            timer.cancel();

            Self {
                inner: parking_lot::RwLock::new((config, timer)),
            }
        })
    }

    pub fn save(&self) -> io::Result<()> {
        self.with_read(|config| config.save())
    }

    #[inline]
    pub fn with_read<R, F: FnOnce(&DiscordConfig) -> R>(&self, cb: F) -> R {
        let inner = self.inner.read();

        cb(&inner.0)
    }

    #[inline]
    pub fn with_write<R, F: FnOnce(&mut DiscordConfig) -> R>(&self, cb: F) -> R {
        let mut inner = self.inner.write();

        let res = cb(&mut inner.0);

        match inner.1.tick() {
            false => match inner.1.is_ticking() {
                true => return res,
                false => (),
            },
            true => (),
        }

        inner.1.restart(CONFIG_UPDATE_INTERVAL);
        res
    }
}

fn on_config_update() {
    match crate::config::DISCORD.save() {
        Ok(_) => {
            rogu::info!("Discord config is updated.");
        },
        Err(error) => {
            rogu::error!("Discord unable to save config: {}", error);
            crate::stats::STATS.increment(crate::stats::DiscordBrokenConfigUpdate);
        }
    }
}
