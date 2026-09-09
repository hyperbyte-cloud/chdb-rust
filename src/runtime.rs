//! Process-wide engine lifecycle.
//!
//! chDB installs signal handlers and starts threads for the whole process, not
//! for a connection. Both facts are invisible from the connection API and both
//! matter to a host that has its own signal handling or its own teardown
//! sequence, so the controls for them live here.
//!
//! The ordering rules the C API states in prose are enforced: a preference that
//! only takes effect before startup is refused after it.

use crate::bindings;
use crate::error::{Error, Result};
use crate::registry;

/// Choose whether chDB installs process-wide signal handlers.
///
/// chDB installs handlers for the whole process when the engine starts. A host
/// that manages its own signals — a daemon with a crash reporter, say — will
/// want them off.
///
/// Must be called before the first connection is opened.
///
/// # Errors
///
/// Returns [`Error::EngineAlreadyStarted`] if any connection is already open,
/// because the setting would have no effect.
///
/// # Examples
///
/// ```no_run
/// chdb_rust::runtime::signal_handlers(false)?;
/// let conn = chdb_rust::connection::Connection::open_in_memory()?;
/// # Ok::<(), chdb_rust::error::Error>(())
/// ```
pub fn signal_handlers(enabled: bool) -> Result<()> {
    if registry::refs() > 0 {
        return Err(Error::EngineAlreadyStarted);
    }

    // Wraps chdb_set_signal_handlers_enabled. Takes effect at engine startup.
    unsafe { bindings::chdb_set_signal_handlers_enabled(i32::from(enabled)) };
    Ok(())
}

/// Restore every signal handler chDB installed to `SIG_DFL`.
///
/// For the case [`signal_handlers`] cannot help with: the engine is already
/// running and the host wants its signals back.
pub fn reset_signal_handlers() {
    // Wraps chdb_reset_signal_handlers. Safe at any time.
    unsafe { bindings::chdb_reset_signal_handlers() };
}
