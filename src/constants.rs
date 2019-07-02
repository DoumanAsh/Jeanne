use core::time::Duration;

pub const ADMIN_CHECK_FAIL: &str = "You're unathorized to access the command.";

pub const CONFIG_UPDATE_INTERVAL: Duration = Duration::from_secs(15 * 60);
