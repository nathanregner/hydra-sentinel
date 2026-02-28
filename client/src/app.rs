use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tokio::sync::{oneshot, watch};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

use crate::ConnectionState;

#[derive(Debug)]
pub enum UserEvent {
    MenuEvent(MenuEvent),
    StateChange,
    Exit,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
struct State {
    enabled: bool,
    connection_state: ConnectionState,
}

pub fn run(
    mut shutdown_rx: oneshot::Receiver<()>,
    connection_state: watch::Receiver<ConnectionState>,
    enabled: watch::Sender<bool>,
) -> anyhow::Result<()> {
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Accessory);
    }

    tokio::task::spawn({
        let mut enabled = enabled.subscribe();
        let mut connection_state = connection_state.clone();
        let proxy = event_loop.create_proxy();
        async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        let _ = proxy.send_event(UserEvent::Exit);
                        break;
                    }
                    _ = connection_state.changed() => {},
                    _ = enabled.changed() => {},
                };
                if let Err(err) = proxy.send_event(UserEvent::StateChange) {
                    tracing::error!("Failed to send event: {err}");
                }
            }
        }
    });

    let color: &Icon = Box::leak(Box::new(
        load_icon(include_bytes!("../assets/logo-color.png")).expect("valid icon"),
    ));
    let gray: &Icon = Box::leak(Box::new(
        load_icon(include_bytes!("../assets/logo-gray.png")).expect("valid icon"),
    ));
    let white: &Icon = Box::leak(Box::new(
        load_icon(include_bytes!("../assets/logo-white.png")).expect("valid icon"),
    ));

    let tray_menu = Menu::new();
    let toggle = MenuItem::new("", true, None);
    tray_menu.append_items(&[&toggle])?;

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        if let Err(err) = proxy.send_event(UserEvent::MenuEvent(event)) {
            tracing::error!("Failed to proxy MenuEvent: {}", err);
        }
    }));

    let mut update = {
        let mut current_state = None;
        let enabled = enabled.clone();
        let toggle = toggle.clone();
        move |tray_icon: &mut TrayIcon| {
            let state = State {
                enabled: *enabled.borrow(),
                connection_state: *connection_state.borrow(),
            };
            if Some(state) != current_state {
                current_state = Some(state);
                toggle.set_text(if state.enabled { "Disable" } else { "Enable" });

                #[cfg(target_os = "linux")]
                tray_icon.set_title(Some(match state.connection_state {
                    ConnectionState::Connected { keep_awake: true } => "Keepawake requested",
                    ConnectionState::Connected { keep_awake: false } => "Connected",
                    ConnectionState::Disconnected => "Disconnected",
                }));

                #[cfg(not(target_os = "linux"))]
                if let Err(err) = tray_icon.set_tooltip(Some(match state.connection_state {
                    ConnectionState::Connected { keep_awake: true } => "Keepawake requested",
                    ConnectionState::Connected { keep_awake: false } => "Connected",
                    ConnectionState::Disconnected => "Disconnected",
                })) {
                    tracing::warn!("Failed to update tooltip: {err}")
                }

                if let Err(err) = tray_icon.set_icon(Some(
                    match (state.connection_state, state.enabled) {
                        (ConnectionState::Connected { keep_awake: true }, true) => color,
                        (ConnectionState::Connected { keep_awake: false }, true) => white,
                        _ => gray,
                    }
                    .clone(),
                )) {
                    tracing::warn!("Failed to update icon: {err}")
                }
            }
        }
    };

    let mut tray_icon = None;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
                tray_icon = Some({
                    let mut tray_icon = TrayIconBuilder::new()
                        .with_menu(Box::new(tray_menu.clone()))
                        .build()
                        .unwrap();
                    update(&mut tray_icon);
                    tray_icon
                });
            }
            Event::UserEvent(UserEvent::MenuEvent(event)) if event.id() == toggle.id() => {
                enabled.send_modify(|enabled| {
                    *enabled = !*enabled;
                    if *enabled {
                        tracing::debug!("Enabled");
                    } else {
                        tracing::debug!("Disabled");
                    }
                });
            }
            Event::UserEvent(UserEvent::StateChange) => {
                if let Some(tray_icon) = &mut tray_icon {
                    update(tray_icon);
                }
            }
            Event::UserEvent(UserEvent::Exit) | Event::LoopDestroyed => {
                tracing::debug!("Exiting");
                tray_icon.take();
                *control_flow = ControlFlow::ExitWithCode(0);
            }
            _ => {
                tracing::trace!("Ignored event {event:?}")
            }
        }
    });
}

fn load_icon(bytes: &[u8]) -> anyhow::Result<tray_icon::Icon> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(bytes)?.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    Ok(tray_icon::Icon::from_rgba(
        icon_rgba,
        icon_width,
        icon_height,
    )?)
}
