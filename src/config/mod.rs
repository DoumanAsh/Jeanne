use std::path::{Path, PathBuf};
use std::io::{self};
use std::fs;
use std::env;

use serde::Serialize;
use serde::de::{DeserializeOwned};

pub mod discord;
pub use discord::{DISCORD_TOKEN, Discord};

pub const CMD_PREFIX: &str = "~";

lazy_static::lazy_static! {
    pub static ref DISCORD: Discord = match Discord::new() {
        Ok(discord) => discord,
        Err(error) => {
            rogu::error!("Unable to load discord config: {}", error);
            unreachable!()
        }
    };
}

pub fn init() {
    rogu::set_level(rogu::Level::TRACE);

    lazy_static::initialize(&DISCORD);
}

#[inline(always)]
pub fn load_from_file<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let file = fs::File::open(&path).map_err(|error| io::Error::new(io::ErrorKind::Other, format!("{}: {}", path.display(), error)))?;
    bincode::deserialize_from(file).map_err(|error| io::Error::new(io::ErrorKind::Other, format!("Invalid config: {}", error)))
}

#[inline(always)]
pub fn save_to_file<T: Serialize>(value: &T, path: &Path) -> io::Result<()> {
    let file = fs::File::create(&path).map_err(|error| io::Error::new(io::ErrorKind::Other, format!("{}: {}", path.display(), error)))?;
    bincode::serialize_into(file, value).map_err(|error| io::Error::new(io::ErrorKind::Other, format!("Invalid config: {}", error)))
}

pub trait FileSystemLoad: Serialize + DeserializeOwned + Default {
    const NAME: &'static str;

    fn path() -> PathBuf {
        match env::current_exe() {
            Ok(mut result) => {
                result.set_file_name(Self::NAME);
                result
            },
            Err(_) => unreachable!(),
        }
    }

    fn existing_path() -> io::Result<PathBuf> {
        let path = Self::path();

        match path.exists() {
            true => Ok(path),
            false => Err(io::Error::new(io::ErrorKind::Other, "Unable to find configuration file"))
        }
    }

    fn load() -> io::Result<Self> {
        Self::existing_path().and_then(|path| load_from_file(&path))
                             .or_else(|_| Ok(Self::default()))
    }

    fn save(&self) -> io::Result<()> {
        save_to_file(&self, &Self::path())
    }
}
