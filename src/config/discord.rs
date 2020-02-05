use serde::{Serialize, Deserialize};
use async_timer::oneshot::{Oneshot, Timer};

use std::io;
use std::collections::HashSet;
use core::ptr;
use core::pin::Pin;
use core::future::Future;
use core::task::{self, Poll};

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
    inner: parking_lot::RwLock<(DiscordConfig, Timer)>,
}

impl Discord {
    pub fn new() -> io::Result<Self> {
        DiscordConfig::load().map(|config| Self {
            inner: parking_lot::RwLock::new((config, Timer::new(CONFIG_UPDATE_INTERVAL))),
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

        let waker = task::RawWaker::new(ptr::null(), &VTABLE);
        let waker = unsafe { task::Waker::from_raw(waker) };
        let mut context = task::Context::from_waker(&waker);

        match Future::poll(Pin::new(&mut inner.1), &mut context) {
            Poll::Pending => match inner.1.is_ticking() {
                true => return res,
                false => (),
            },
            Poll::Ready(_) => ()
        }

        inner.1.restart(CONFIG_UPDATE_INTERVAL, &waker);
        res
    }
}

static VTABLE: task::RawWakerVTable = task::RawWakerVTable::new(vtab::on_clone, vtab::on_wake, vtab::on_wake, vtab::on_drop);

mod vtab {
    use super::*;

    pub unsafe fn on_clone(_data: *const ()) -> task::RawWaker {
        task::RawWaker::new(ptr::null(), &VTABLE)
    }

    pub unsafe fn on_drop(_data: *const ()) {
    }

    pub unsafe fn on_wake(_data: *const ()) {
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
}
