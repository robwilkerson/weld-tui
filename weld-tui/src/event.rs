use std::time::Duration;

use crossterm::event::{self, Event};

/// Waits for a terminal event, blocking up to `timeout` via the OS kernel
/// (epoll/kqueue). Returns `None` if no event arrives within the timeout.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
