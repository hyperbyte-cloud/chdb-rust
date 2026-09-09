//! Process-wide engine lifecycle.
//!
//! chDB installs signal handlers and starts threads for the whole process, not
//! for a connection. Both facts are invisible from the connection API and both
//! matter to a host that has its own signal handling or its own teardown
//! sequence, so the controls for them live here.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::bindings;
use crate::error::{Error, Result};
use crate::registry;

/// Choose whether chDB installs process-wide signal handlers.
///
/// May be called at any time, not just before the first connection. chDB
/// re-installs its deadly-signal handlers at the start of every query
/// (`setupCommonDeadlySignalHandlers`, idempotent), consulting a process-wide
/// flag each time. Passing `false` here sets that flag *and* immediately
/// resets any handlers already installed (chDB's own
/// `chdb_set_signal_handlers_enabled` calls `chdb_reset_signal_handlers`
/// internally when disabling), so a host with an existing connection can
/// still opt out and have it stick — the next query will not re-install them.
///
/// Calling this before the first connection is still the cleanest way to
/// ensure the handlers are never installed at all, since it removes the brief
/// window where a query could run before the preference is set. But it is no
/// longer required: this call is infallible and works after connections are
/// already open.
///
/// # Examples
///
/// ```no_run
/// chdb_rust::runtime::signal_handlers(false);
/// let conn = chdb_rust::connection::Connection::open_in_memory()?;
/// # Ok::<(), chdb_rust::error::Error>(())
/// ```
pub fn signal_handlers(enabled: bool) {
    // Wraps chdb_set_signal_handlers_enabled. Sets the process-wide disable
    // flag consulted on every query's handler setup; disabling also resets
    // any handlers already installed, as an immediate effect of this call.
    unsafe { bindings::chdb_set_signal_handlers_enabled(i32::from(enabled)) };
}

/// Restore every signal handler chDB installed to `SIG_DFL`.
///
/// This is temporary on its own: it restores the disposition but does not set
/// the disable flag, so the very next query will re-install the handlers
/// (they are re-installed at the start of every query, not once at engine
/// start). For a durable opt-out that survives subsequent queries, use
/// [`signal_handlers`]`(false)` instead, which sets the flag as well as
/// resetting.
pub fn reset_signal_handlers() {
    // Wraps chdb_reset_signal_handlers. Safe at any time; resets disposition
    // only, does not touch the disable flag.
    unsafe { bindings::chdb_reset_signal_handlers() };
}

/// Set once a shutdown has actually succeeded. Only then is the library closed
/// for the process — a refused shutdown must leave it usable.
static SHUT_DOWN: AtomicBool = AtomicBool::new(false);

/// Stop the engine, joining every thread chDB started.
///
/// Call this before a host teardown sequence of its own — global destructors, a
/// finalizing language runtime, a sanitizer exit handler — which would
/// otherwise race the engine's still-running threads. A process that simply
/// exits does not need it.
///
/// This is one-way. Once it succeeds, no further connection can be opened in
/// this process. Calling it again is harmless.
///
/// # Errors
///
/// Returns [`Error::ConnectionsStillOpen`] if any connection is open, and
/// [`Error::Unknown`] if the engine could not stop every thread.
///
/// # Examples
///
/// ```no_run
/// let conn = chdb_rust::connection::Connection::open_in_memory()?;
/// drop(conn);
/// chdb_rust::runtime::shutdown()?;
/// # Ok::<(), chdb_rust::error::Error>(())
/// ```
pub fn shutdown() -> Result<()> {
    if SHUT_DOWN.load(Ordering::SeqCst) {
        return Ok(());
    }

    let count = registry::refs();
    if count != 0 {
        return Err(Error::ConnectionsStillOpen { count });
    }

    // Wraps chdb_shutdown. Refuses while the engine holds a connection, which
    // the check above should already have caught; a refusal here means some
    // other crate in this process holds one.
    let state = unsafe { bindings::chdb_shutdown() };
    if state != bindings::chdb_state_CHDBSuccess {
        return Err(Error::Unknown);
    }

    SHUT_DOWN.store(true, Ordering::SeqCst);
    Ok(())
}

/// Whether a new connection may still be opened.
pub(crate) fn ensure_running() -> Result<()> {
    if SHUT_DOWN.load(Ordering::SeqCst) {
        return Err(Error::EngineShutDown);
    }
    Ok(())
}
