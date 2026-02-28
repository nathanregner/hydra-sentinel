use std::sync::Arc;
use tokio::sync::Notify;

/// Spawns a platform-specific wake-from-sleep monitor.
///
/// Returns an `Arc<Notify>` that is signalled each time the system wakes from sleep.
/// The caller can `select!` on `notify.notified()` alongside other futures.
pub fn spawn() -> Arc<Notify> {
    let notify = Arc::new(Notify::new());
    start(notify.clone());
    notify
}

// ---- macOS: IOKit power notifications on a dedicated OS thread ----
// macOS implementation adapted from:
// https://github.com/gamepoet/naptime/blob/main/src/macos.rs

#[cfg(target_os = "macos")]
fn start(notify: Arc<Notify>) {
    std::thread::spawn(move || {
        use apple_sys::IOKit::{
            io_connect_t, io_object_t, io_service_t, kCFRunLoopCommonModes, CFRunLoopAddSource,
            CFRunLoopGetCurrent, CFRunLoopRemoveSource, CFRunLoopRun, IOAllowPowerChange,
            IODeregisterForSystemPower, IOMessageCanSystemSleep, IOMessageSystemHasPoweredOn,
            IOMessageSystemWillNotSleep, IOMessageSystemWillPowerOn, IOMessageSystemWillSleep,
            IONotificationPortDestroy, IONotificationPortGetRunLoopSource, IONotificationPortRef,
            IORegisterForSystemPower, IOServiceClose,
        };
        use std::ffi::c_void;

        struct Ctx {
            notify: Arc<Notify>,
            root_port: io_connect_t,
        }

        #[allow(non_upper_case_globals)]
        unsafe extern "C" fn power_callback(
            refcon: *mut c_void,
            _service: io_service_t,
            message_type: u32,
            message_arg: *mut c_void,
        ) {
            let ctx = &*(refcon as *const Ctx);
            match message_type {
                IOMessageCanSystemSleep => {
                    tracing::trace!("kIOMessageCanSystemSleep — allowing");
                    IOAllowPowerChange(ctx.root_port, message_arg as usize as isize);
                }
                IOMessageSystemWillSleep => {
                    tracing::trace!("kIOMessageSystemWillSleep — allowing");
                    IOAllowPowerChange(ctx.root_port, message_arg as usize as isize);
                }
                IOMessageSystemWillNotSleep => {
                    tracing::trace!("kIOMessageSystemWillNotSleep");
                }
                IOMessageSystemWillPowerOn => {
                    tracing::trace!("kIOMessageSystemWillPowerOn");
                }
                IOMessageSystemHasPoweredOn => {
                    tracing::trace!("kIOMessageSystemHasPoweredOn");
                    ctx.notify.notify_one();
                }
                other => {
                    tracing::debug!(message_type = other, "unknown power message");
                }
            }
        }

        let mut notify_port: IONotificationPortRef = std::ptr::null_mut();
        let mut notifier: io_object_t = 0;

        // Allocate the context before calling IORegisterForSystemPower so we have a stable
        // address to pass as refcon. root_port is filled in after registration.
        let ctx = Box::new(Ctx {
            notify,
            root_port: 0,
        });
        let ctx_ptr = Box::into_raw(ctx);

        let root_port = unsafe {
            IORegisterForSystemPower(
                ctx_ptr.cast::<c_void>(),
                &mut notify_port,
                Some(power_callback),
                &mut notifier,
            )
        };

        if root_port == 0 || notify_port.is_null() {
            tracing::error!("IORegisterForSystemPower failed; wake-from-sleep detection disabled");
            drop(unsafe { Box::from_raw(ctx_ptr) });
            return;
        }

        // The callback is only ever invoked from within CFRunLoopRun (below), so
        // writing root_port here is race-free.
        unsafe { (*ctx_ptr).root_port = root_port };

        let source = unsafe { IONotificationPortGetRunLoopSource(notify_port) };
        let run_loop = unsafe { CFRunLoopGetCurrent() };
        unsafe { CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes) };

        // Blocks this dedicated thread until the process exits.
        unsafe { CFRunLoopRun() };

        unsafe {
            CFRunLoopRemoveSource(run_loop, source, kCFRunLoopCommonModes);
            IODeregisterForSystemPower(&mut notifier);
            IOServiceClose(root_port);
            IONotificationPortDestroy(notify_port);
            drop(Box::from_raw(ctx_ptr));
        }
    });
}

// ---- Linux: logind D-Bus PrepareForSleep signal ----

#[cfg(target_os = "linux")]
fn start(notify: Arc<Notify>) {
    tokio::spawn(async move {
        if let Err(err) = run_linux(notify).await {
            tracing::error!(?err, "Wake monitor exited with error");
        }
    });
}

#[cfg(target_os = "linux")]
async fn run_linux(notify: Arc<Notify>) -> anyhow::Result<()> {
    use futures_util::StreamExt;
    use zbus::message::Type as MessageType;
    use zbus::{MatchRule, MessageStream};

    let conn = zbus::Connection::system().await?;
    let rule = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender("org.freedesktop.login1")?
        .interface("org.freedesktop.login1.Manager")?
        .member("PrepareForSleep")?
        .build();
    let mut stream = MessageStream::for_match_rule(rule, &conn, None).await?;
    while let Some(Ok(msg)) = stream.next().await {
        // PrepareForSleep(false) means the system just woke up.
        // PrepareForSleep(true) means the system is about to sleep.
        if let Ok((false,)) = msg.body().deserialize::<(bool,)>() {
            notify.notify_one();
        }
    }
    Ok(())
}

// ---- Unsupported platforms: no-op ----

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn start(_notify: Arc<Notify>) {}
