// Cargo.toml dependencies:
// [dependencies]
// libcosmic = { git = "https://github.com/pop-os/libcosmic" }
// tokio = { version = "1", features = ["full"] }
// wayland-client = "0.31"
// wayland-protocols = { version = "0.31", features = ["client", "unstable"] }
// zbus = { version = "5.13.2", default-features = false, features = ["blocking-api"] }
// notify-rust = "4.10"
// sys-locale = "0.3"
// lazy_static = "1.4"
// IMPORTANTE: Connessioni D-Bus vengono chiuse alla disattivazione e riaperte alla riattivazione

use cosmic::iced::{ Length, Subscription };
use cosmic::widget::{button, icon, container};
use cosmic::{app, Application, Element, Task};

use wayland_client::{
    delegate_noop, Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_client::protocol::{wl_compositor, wl_registry, wl_surface};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    zwp_idle_inhibit_manager_v1, zwp_idle_inhibitor_v1,
};

use zbus::blocking::Connection as DbusConnection;
use zbus::proxy;
use notify_rust::Notification;

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::path::Path;
use std::fs;
use std::collections::HashMap;

// svg icons
const ICON_OFF_LIGHT: &str = r##"<svg height="800px" width="800px" version="1.1" id="_x32_" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" 
	 viewBox="0 0 512 512"  xml:space="preserve">
<g>
	<path fill="#000000" d="M479.336,335.852c20.763-27.978,32.711-64.873,32.664-110.414V103.265
		c-0.01-24.739-20.044-44.763-44.774-44.773c0,0-85.116,0-171.396,0c-86.29,0-173.744,0-178.43,0
		C52.556,58.502,0.009,111.048,0,175.893c-0.038,32.389,13.302,61.228,34.539,81.924c21.226,20.744,50.255,33.65,82.038,35.468
		c2.272,0.122,4.488,0.179,6.646,0.179c4.772,0,9.335-0.341,13.738-0.938c11.125,27.571,28.773,51.799,51.259,70.592l-0.34-0.284
		l0.255,0.217c0.729,0.616,1.648,1.326,2.547,2.036H123.64v40.902c0.01,26.244,21.264,47.508,47.519,47.518h293.19
		c26.244-0.01,47.51-21.264,47.518-47.518v-40.902h-61.246C461.31,356.756,471.014,347.099,479.336,335.852z M482.782,394.173
		v11.816c-0.018,10.178-8.265,18.415-18.433,18.433h-293.19c-10.178-0.018-18.416-8.256-18.434-18.433v-11.816h94.28h147.868
		H482.782z M389.884,365.088H252.137c-11.769-4.299-21.729-9.165-29.332-13.539c-8.058-4.62-13.662-8.805-15.849-10.679l0.161,0.141
		l-0.246-0.198c-21.558-18.027-37.88-42.075-46.676-69.513l-4.355-13.539l-13.634,4.043c-5.454,1.619-11.684,2.575-18.983,2.575
		c-1.628,0-3.294-0.048-5.008-0.142c-24.919-1.412-47.32-11.532-63.34-27.221c-16.019-15.725-25.752-36.706-25.79-61.123
		c0.009-24.437,9.865-46.421,25.866-62.45c16.029-16.01,38.013-25.856,62.45-25.866c9.373,0,349.826,0,349.826,0
		c8.654,0.01,15.67,7.025,15.688,15.688v122.173c-0.048,40.617-10.348,70.62-26.955,93.106
		C439.722,340.434,417,355.564,389.884,365.088z"/>
	<path fill="#000000" d="M122.712,122.154c-29.691,0.01-53.73,24.057-53.74,53.74c0.01,29.691,24.058,53.739,53.74,53.748
		c9.572,0,18.368-2.708,25.838-6.826l6.268-3.455v-86.934l-6.268-3.446C141.08,124.852,132.284,122.144,122.712,122.154z
		 M130.58,204.164c-2.537,0.767-5.15,1.24-7.868,1.24c-16.294-0.028-29.474-13.208-29.502-29.511
		c0.028-16.285,13.218-29.474,29.502-29.502c2.718,0.01,5.33,0.483,7.868,1.24V204.164z"/>
</g>
</svg>"##;

const ICON_ON_LIGHT: &str = r##"<svg height="800px" width="800px" version="1.1" id="_x32_" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" 
	 viewBox="0 0 512 512"  xml:space="preserve">
<g>
	<path fill="#000000" d="M129.18,417.603c0,19.313,15.649,34.962,34.962,34.962H474.99c19.304,0,34.963-15.649,34.963-34.962v-27.946
		H129.18V417.603z"/>
	<path fill="#000000" d="M479.949,59.435H143.092h-11.855c-5.2,0-12.247,0-22.184,0C48.825,59.435,0,108.26,0,168.489
		c0,60.228,48.925,105.641,109.054,109.064c11.634,0.662,21.792-0.542,30.686-3.192c10.229,31.871,29.19,59.826,54.286,80.807
		h255.186C487.568,323.094,512,274.932,512,221.018V91.487C512,73.78,497.646,59.435,479.949,59.435z M131.238,208.791
		c-6.616,3.654-14.094,5.912-22.184,5.912c-25.518,0-46.206-20.688-46.206-46.215c0-25.516,20.688-46.205,46.206-46.205
		c8.09,0,15.568,2.258,22.184,5.902V208.791z"/>
</g>
</svg>"##;

const ICON_OFF_DARK: &str = r##"<svg height="800px" width="800px" version="1.1" id="_x32_" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" 
	 viewBox="0 0 512 512"  xml:space="preserve">
<g>
	<path fill="#ffffff" d="M479.336,335.852c20.763-27.978,32.711-64.873,32.664-110.414V103.265
		c-0.01-24.739-20.044-44.763-44.774-44.773c0,0-85.116,0-171.396,0c-86.29,0-173.744,0-178.43,0
		C52.556,58.502,0.009,111.048,0,175.893c-0.038,32.389,13.302,61.228,34.539,81.924c21.226,20.744,50.255,33.65,82.038,35.468
		c2.272,0.122,4.488,0.179,6.646,0.179c4.772,0,9.335-0.341,13.738-0.938c11.125,27.571,28.773,51.799,51.259,70.592l-0.34-0.284
		l0.255,0.217c0.729,0.616,1.648,1.326,2.547,2.036H123.64v40.902c0.01,26.244,21.264,47.508,47.519,47.518h293.19
		c26.244-0.01,47.51-21.264,47.518-47.518v-40.902h-61.246C461.31,356.756,471.014,347.099,479.336,335.852z M482.782,394.173
		v11.816c-0.018,10.178-8.265,18.415-18.433,18.433h-293.19c-10.178-0.018-18.416-8.256-18.434-18.433v-11.816h94.28h147.868
		H482.782z M389.884,365.088H252.137c-11.769-4.299-21.729-9.165-29.332-13.539c-8.058-4.62-13.662-8.805-15.849-10.679l0.161,0.141
		l-0.246-0.198c-21.558-18.027-37.88-42.075-46.676-69.513l-4.355-13.539l-13.634,4.043c-5.454,1.619-11.684,2.575-18.983,2.575
		c-1.628,0-3.294-0.048-5.008-0.142c-24.919-1.412-47.32-11.532-63.34-27.221c-16.019-15.725-25.752-36.706-25.79-61.123
		c0.009-24.437,9.865-46.421,25.866-62.45c16.029-16.01,38.013-25.856,62.45-25.866c9.373,0,349.826,0,349.826,0
		c8.654,0.01,15.67,7.025,15.688,15.688v122.173c-0.048,40.617-10.348,70.62-26.955,93.106
		C439.722,340.434,417,355.564,389.884,365.088z"/>
	<path fill="#ffffff" d="M122.712,122.154c-29.691,0.01-53.73,24.057-53.74,53.74c0.01,29.691,24.058,53.739,53.74,53.748
		c9.572,0,18.368-2.708,25.838-6.826l6.268-3.455v-86.934l-6.268-3.446C141.08,124.852,132.284,122.144,122.712,122.154z
		 M130.58,204.164c-2.537,0.767-5.15,1.24-7.868,1.24c-16.294-0.028-29.474-13.208-29.502-29.511
		c0.028-16.285,13.218-29.474,29.502-29.502c2.718,0.01,5.33,0.483,7.868,1.24V204.164z"/>
</g>
</svg>"##;

const ICON_ON_DARK: &str = r##"<svg height="800px" width="800px" version="1.1" id="_x32_" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" 
	 viewBox="0 0 512 512"  xml:space="preserve">
<g>
	<path fill="#ffffff" d="M129.18,417.603c0,19.313,15.649,34.962,34.962,34.962H474.99c19.304,0,34.963-15.649,34.963-34.962v-27.946
		H129.18V417.603z"/>
	<path fill="#ffffff" d="M479.949,59.435H143.092h-11.855c-5.2,0-12.247,0-22.184,0C48.825,59.435,0,108.26,0,168.489
		c0,60.228,48.925,105.641,109.054,109.064c11.634,0.662,21.792-0.542,30.686-3.192c10.229,31.871,29.19,59.826,54.286,80.807
		h255.186C487.568,323.094,512,274.932,512,221.018V91.487C512,73.78,497.646,59.435,479.949,59.435z M131.238,208.791
		c-6.616,3.654-14.094,5.912-22.184,5.912c-25.518,0-46.206-20.688-46.206-46.215c0-25.516,20.688-46.205,46.206-46.205
		c8.09,0,15.568,2.258,22.184,5.902V208.791z"/>
</g>
</svg>"##;

// D-Bus Proxy definitions
#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    fn get_display_device(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower"
)]
trait UPowerDevice {
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
}

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    fn inhibit(
        &self,
        what: &str,
        who: &str,
        why: &str,
        mode: &str,
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;
}

#[proxy(assume_defaults = true)]
trait ScreenSaver {
    fn inhibit(&self, application_name: &str, reason_for_inhibit: &str) -> zbus::Result<u32>;
    fn un_inhibit(&self, cookie: u32) -> zbus::Result<()>;
}

// Simple localization system
struct Locale {
    translations: HashMap<&'static str, String>,
}

impl Locale {
    fn new() -> Self {
        let lang = sys_locale::get_locale()
            .unwrap_or_else(|| String::from("en-US"));

        let translations = Self::get_translations(&lang);

        Self { translations }
    }

    fn get(&self, key: &str) -> String {
        self.translations.get(key)
            .cloned()
            .unwrap_or_else(|| String::from(key))
    }

    fn get_translations(lang: &str) -> HashMap<&'static str, String> {
        let lang_prefix = lang.split('-').next().unwrap_or("en");

        match lang_prefix {
            "it" => HashMap::from([
                ("app-name", String::from("CosmicCaffeineLand")),
                ("battery-low-notification", String::from("Caffeine è stato disattivato e non può essere riattivato, livello batteria troppo basso")),
                ("lid-closed-notification", String::from("Caffeine è stato disattivato per aver rilevato la chiusura dello schermo")),
            ]),
            "es" => HashMap::from([
                ("app-name", String::from("CosmicCaffeineLand")),
                ("battery-low-notification", String::from("Caffeine ha sido desactivado y no puede ser reactivado, nivel de batería demasiado bajo")),
                ("lid-closed-notification", String::from("Caffeine ha sido desactivado al detectar el cierre de la pantalla")),
            ]),
            "fr" => HashMap::from([
                ("app-name", String::from("CosmicCaffeineLand")),
                ("battery-low-notification", String::from("Caffeine a été désactivé et ne peut pas être réactivé, niveau de batterie trop faible")),
                ("lid-closed-notification", String::from("Caffeine a été désactivé suite à la détection de la fermeture de l'écran")),
            ]),
            "de" => HashMap::from([
                ("app-name", String::from("CosmicCaffeineLand")),
                ("battery-low-notification", String::from("Caffeine wurde deaktiviert und kann nicht reaktiviert werden, Batteriestand zu niedrig")),
                ("lid-closed-notification", String::from("Caffeine wurde deaktiviert, da das Schließen des Bildschirms erkannt wurde")),
            ]),
            "pt" => HashMap::from([
                ("app-name", String::from("CosmicCaffeineLand")),
                ("battery-low-notification", String::from("Caffeine foi desativado e não pode ser reativado, nível de bateria muito baixo")),
                ("lid-closed-notification", String::from("Caffeine foi desativado ao detectar o fechamento da tela")),
            ]),
            "ru" => HashMap::from([
                ("app-name", String::from("CosmicCaffeineLand")),
                ("battery-low-notification", String::from("Caffeine был отключен и не может быть включен, уровень заряда слишком низкий")),
                ("lid-closed-notification", String::from("Caffeine был отключен из-за обнаружения закрытия экрана")),
            ]),
            "zh" => HashMap::from([
                ("app-name", String::from("CosmicCaffeineLand")),
                ("battery-low-notification", String::from("咖啡因已被禁用且无法重新启用,电池电量过低")),
                ("lid-closed-notification", String::from("检测到屏幕关闭,咖啡因已被禁用")),
            ]),
            "ja" => HashMap::from([
                ("app-name", String::from("CosmicCaffeineLand")),
                ("battery-low-notification", String::from("バッテリー残量が少ないため、カフェインが無効化され、再有効化できません")),
                ("lid-closed-notification", String::from("画面の閉鎖を検出したため、カフェインが無効化されました")),
            ]),
            _ => HashMap::from([
                ("app-name", String::from("CosmicCaffeineLand")),
                ("battery-low-notification", String::from("Caffeine has been disabled and cannot be reactivated, battery level too low")),
                ("lid-closed-notification", String::from("Caffeine has been disabled due to screen closure detection")),
            ]),
        }
    }
}

lazy_static::lazy_static! {
    static ref LOCALE: Locale = Locale::new();
}

// Wayland state management
struct WaylandState {
    compositor: Option<wl_compositor::WlCompositor>,
    inhibit_manager: Option<zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1>,
    surface: Option<wl_surface::WlSurface>,
    inhibitor: Option<zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match &interface[..] {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind::<wl_compositor::WlCompositor, _, _>(
                        name, version.min(6), qh, (),
                    ));
                }
                "zwp_idle_inhibit_manager_v1" => {
                    state.inhibit_manager = Some(
                        registry.bind::<zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, _, _>(
                            name, version.min(1), qh, (),
                        ),
                    );
                }
                _ => {}
            }
        }
    }
}

delegate_noop!(WaylandState: ignore wl_compositor::WlCompositor);
delegate_noop!(WaylandState: ignore wl_surface::WlSurface);
delegate_noop!(WaylandState: ignore zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1);
delegate_noop!(WaylandState: ignore zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1);

// Lid state detection
#[derive(Debug, Clone, PartialEq)]
pub enum LidState {
    Open,
    Closed,
    Unknown,
}

impl LidState {
    fn check() -> Self {
        if let Ok(entries) = fs::read_dir("/proc/acpi/button/lid") {
            for entry in entries.flatten() {
                let state_path = entry.path().join("state");
                if let Ok(content) = fs::read_to_string(&state_path) {
                    if content.contains("closed") {
                        return LidState::Closed;
                    } else if content.contains("open") {
                        return LidState::Open;
                    }
                }
            }
        }

        if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
            for entry in entries.flatten() {
                let lid_path = entry.path().join("device/lid_state");
                if let Ok(content) = fs::read_to_string(&lid_path) {
                    let trimmed = content.trim();
                    if trimmed == "closed" {
                        return LidState::Closed;
                    } else if trimmed == "open" {
                        return LidState::Open;
                    }
                }
            }
        }

        LidState::Unknown
    }

    fn is_supported() -> bool {
        Path::new("/proc/acpi/button/lid").exists() ||
            fs::read_dir("/sys/class/power_supply")
                .ok()
                .and_then(|entries| {
                    entries.flatten().find(|e| {
                        e.path().join("device/lid_state").exists()
                    })
                })
                .is_some()
    }
}

// Battery status
#[derive(Debug, Clone)]
pub struct BatteryStatus {
    percentage: f64,
    is_charging: bool,
}

impl BatteryStatus {
    fn check() -> Option<Self> {
        let conn = DbusConnection::system().ok()?;

        let upower_proxy = UPowerProxyBlocking::new(&conn).ok()?;
        let display_device_path = upower_proxy.get_display_device().ok()?;

        let device_proxy = UPowerDeviceProxyBlocking::builder(&conn)
            .path(display_device_path)
            .ok()?
            .build()
            .ok()?;

        let percentage = device_proxy.percentage().ok()?;
        let state = device_proxy.state().ok()?;

        let is_charging = matches!(state, 1 | 4 | 5);

        Some(BatteryStatus {
            percentage,
            is_charging,
        })
    }

    fn is_critical(&self) -> bool {
        self.percentage < 10.0 && !self.is_charging
    }
}

// Main applet structure
pub struct CaffeineApplet {
    core: app::Core,
    is_active: bool,
    user_enabled: bool,
    battery_status: Option<BatteryStatus>,
    lid_state: LidState,
    lid_supported: bool,
    notification_sent: bool,
    wayland_state: Arc<Mutex<Option<WaylandState>>>,
    event_queue: Arc<Mutex<Option<EventQueue<WaylandState>>>>,
    // Connessioni D-Bus che vengono chiuse/riaperte ad ogni ciclo
    session_conn: Arc<Mutex<Option<DbusConnection>>>,
    system_conn: Arc<Mutex<Option<DbusConnection>>>,
    // Inhibitors
    idle_fd: Arc<Mutex<Option<zbus::zvariant::OwnedFd>>>,
    sleep_fd: Arc<Mutex<Option<zbus::zvariant::OwnedFd>>>,
    lid_fd: Arc<Mutex<Option<zbus::zvariant::OwnedFd>>>,
    screensaver_cookie: Arc<Mutex<Option<u32>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Toggle,
    BatteryUpdate(Option<BatteryStatus>),
    LidStateUpdate(LidState),
}

impl Application for CaffeineApplet {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.system76.CosmicCaffeineLand";

    fn core(&self) -> &app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut app::Core {
        &mut self.core
    }

    fn init(core: app::Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let wayland_state = Arc::new(Mutex::new(None));
        let event_queue = Arc::new(Mutex::new(None));

        if let Ok(conn) = Connection::connect_to_env() {
            let display = conn.display();
            let mut eq = conn.new_event_queue();
            let qh = eq.handle();

            let mut state = WaylandState {
                compositor: None,
                inhibit_manager: None,
                surface: None,
                inhibitor: None,
            };

            let _registry = display.get_registry(&qh, ());

            if eq.roundtrip(&mut state).is_ok() {
                *wayland_state.lock().unwrap() = Some(state);
                *event_queue.lock().unwrap() = Some(eq);
            }
        }

        // Le connessioni iniziano come None, verranno create alla prima attivazione
        let session_conn = Arc::new(Mutex::new(None));
        let system_conn = Arc::new(Mutex::new(None));

        (
            Self {
                core,
                is_active: false,
                user_enabled: false,
                battery_status: BatteryStatus::check(),
                lid_state: LidState::check(),
                lid_supported: LidState::is_supported(),
                notification_sent: false,
                wayland_state,
                event_queue,
                session_conn,
                system_conn,
                idle_fd: Arc::new(Mutex::new(None)),
                sleep_fd: Arc::new(Mutex::new(None)),
                lid_fd: Arc::new(Mutex::new(None)),
                screensaver_cookie: Arc::new(Mutex::new(None)),
            },
            Task::none(),
        )
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let battery_sub = cosmic::iced::time::every(Duration::from_secs(10))
            .map(|_| {
                tokio::task::block_in_place(|| BatteryStatus::check())
            })
            .map(Message::BatteryUpdate);

        if self.lid_supported {
            let lid_sub = cosmic::iced::time::every(Duration::from_secs(10))
                .map(|_| Message::LidStateUpdate(LidState::check()));

            Subscription::batch(vec![battery_sub, lid_sub])
        } else {
            battery_sub
        }
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::Toggle => {
                if let Some(battery) = &self.battery_status {
                    if battery.is_critical() {
                        let _ = Notification::new()
                            .summary(&LOCALE.get("app-name"))
                            .body(&LOCALE.get("battery-low-notification"))
                            .icon("battery-caution")
                            .urgency(notify_rust::Urgency::Normal)
                            .show();

                        self.user_enabled = false;
                        self.notification_sent = true;
                    } else {
                        self.user_enabled = !self.user_enabled;
                        self.notification_sent = false;
                    }
                } else {
                    self.user_enabled = !self.user_enabled;
                }

                self.update_inhibitor_state();
                Task::none()
            }
            Message::BatteryUpdate(status) => {
                let was_critical = self.battery_status
                    .as_ref()
                    .map(|b| b.is_critical())
                    .unwrap_or(false);

                self.battery_status = status;

                let is_now_critical = self.battery_status
                    .as_ref()
                    .map(|b| b.is_critical())
                    .unwrap_or(false);

                if is_now_critical && !was_critical && self.user_enabled {
                    self.user_enabled = false;

                    if !self.notification_sent {
                        let _ = Notification::new()
                            .summary(&LOCALE.get("app-name"))
                            .body(&LOCALE.get("battery-low-notification"))
                            .icon("battery-caution")
                            .urgency(notify_rust::Urgency::Normal)
                            .show();

                        self.notification_sent = true;
                    }
                }

                if !is_now_critical {
                    self.notification_sent = false;
                }

                // Controlla se il lid è chiuso E il cavo è stato staccato
                if self.lid_supported && self.lid_state == LidState::Closed && self.user_enabled {
                    let is_unplugged = self.battery_status
                        .as_ref()
                        .map(|b| !b.is_charging)
                        .unwrap_or(true);

                    if is_unplugged {
                        self.user_enabled = false;
                        let _ = Notification::new()
                            .summary(&LOCALE.get("app-name"))
                            .body(&LOCALE.get("lid-closed-notification"))
                            .icon("caffeine")
                            .urgency(notify_rust::Urgency::Normal)
                            .show();
                        eprintln!("DEBUG: Cavo staccato con lid chiuso - Caffeine disattivato");
                    }
                }

                self.update_inhibitor_state();
                Task::none()
            }
            Message::LidStateUpdate(new_state) => {
                let old_state = self.lid_state.clone();
                self.lid_state = new_state;

                if self.lid_supported && old_state != LidState::Closed && self.lid_state == LidState::Closed {
                    if self.user_enabled {
                        let is_unplugged = self.battery_status
                            .as_ref()
                            .map(|b| !b.is_charging)
                            .unwrap_or(true);

                        if is_unplugged {
                            self.user_enabled = false;
                            self.update_inhibitor_state();

                            let _ = Notification::new()
                                .summary(&LOCALE.get("app-name"))
                                .body(&LOCALE.get("lid-closed-notification"))
                                .icon("caffeine")
                                .urgency(notify_rust::Urgency::Normal)
                                .show();

                            eprintln!("DEBUG: Lid chiuso a batteria - Caffeine disattivato");
                        } else {
                            eprintln!("DEBUG: Lid chiuso ma collegato alla rete - Caffeine rimane attivo");
                        }
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        /*
        let icon = if self.is_active { "☕" } else { "😴" };

        let content = button::standard(icon)
            .on_press(Message::Toggle)
            .padding(2);

        container(content)
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into() 
        */
        
        let is_dark = cosmic::theme::active().theme_type.is_dark();
        
        println!("Is dark theme: {}", cosmic::theme::active().theme_type.is_dark());
        
        let caffeine_icon = match (is_dark, self.is_active) {
            (true, true) => ICON_ON_DARK,
            (true, false) => ICON_OFF_DARK,
            (false, true) => ICON_ON_LIGHT,
            (false, false) => ICON_OFF_LIGHT,
        };
        let caffeine_icon_obj = icon::from_svg_bytes(caffeine_icon.as_bytes());
        
        let caffeine_icon_button = button::icon(
                caffeine_icon_obj
            )
            .on_press(Message::Toggle);
            
        container(caffeine_icon_button)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center)
            .into() 

    }
    
    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}

impl CaffeineApplet {
    fn update_inhibitor_state(&mut self) {
        let should_be_active = if self.user_enabled {
            if let Some(battery) = &self.battery_status {
                !battery.is_critical()
            } else {
                true
            }
        } else {
            false
        };

        if should_be_active != self.is_active {
            self.is_active = should_be_active;

            if self.is_active {
                // Attivazione: apri nuove connessioni D-Bus
                eprintln!("DEBUG: Apertura nuove connessioni D-Bus");
                *self.session_conn.lock().unwrap() = DbusConnection::session().ok();
                *self.system_conn.lock().unwrap() = DbusConnection::system().ok();
            }

            // Determina se siamo nel caso speciale: lid chiuso + cavo collegato
            let lid_closed_on_power = self.lid_supported 
                && self.lid_state == LidState::Closed 
                && self.battery_status.as_ref().map(|b| b.is_charging).unwrap_or(false);

            // Gli inhibitor di sistema vengono sempre gestiti
            self.update_idle_inhibitor();
            self.update_sleep_inhibitor();
            self.update_lid_inhibitor();
            self.update_wayland_inhibitor();

            // Lo screensaver viene inibito SOLO se NON siamo nel caso lid chiuso + AC
            if !lid_closed_on_power {
                self.update_screensaver_inhibitor();
            } else {
                // Se siamo nel caso lid chiuso + AC, forziamo la disattivazione dello screensaver inhibitor
                eprintln!("DEBUG: Lid chiuso su alimentazione - screensaver NON inibito");
                let is_active_backup = self.is_active;
                self.is_active = false;
                self.update_screensaver_inhibitor();
                self.is_active = is_active_backup;
            }

            if !self.is_active {
                // Disattivazione: chiudi le connessioni D-Bus
                eprintln!("DEBUG: Chiusura connessioni D-Bus");
                *self.session_conn.lock().unwrap() = None;
                *self.system_conn.lock().unwrap() = None;
            }
        }
    }

    fn update_wayland_inhibitor(&mut self) {
        let mut state_guard = self.wayland_state.lock().unwrap();
        let mut eq_guard = self.event_queue.lock().unwrap();

        if let (Some(state), Some(eq)) = (state_guard.as_mut(), eq_guard.as_mut()) {
            if self.is_active {
                if state.inhibitor.is_none() {
                    if let (Some(compositor), Some(manager)) =
                        (&state.compositor, &state.inhibit_manager)
                    {
                        let surface = compositor.create_surface(&eq.handle(), ());
                        surface.commit();

                        let inhibitor = manager.create_inhibitor(&surface, &eq.handle(), ());

                        state.surface = Some(surface);
                        state.inhibitor = Some(inhibitor);

                        let _ = eq.roundtrip(state);
                        eprintln!("DEBUG: Wayland idle inhibitor attivato");
                    }
                }
            } else {
                if let Some(inhibitor) = state.inhibitor.take() {
                    inhibitor.destroy();
                    eprintln!("DEBUG: Wayland idle inhibitor disattivato");
                }
                if let Some(surface) = state.surface.take() {
                    surface.destroy();
                }
                let _ = eq.roundtrip(state);
            }
        }
    }

    fn update_idle_inhibitor(&mut self) {
        let idle_fd = self.idle_fd.clone();
        let system_conn = self.system_conn.clone();
        let is_active = self.is_active;

        tokio::task::spawn_blocking(move || {
            if is_active {
                let conn_guard = system_conn.lock().unwrap();
                if let Some(conn) = conn_guard.as_ref() {
                    if let Ok(proxy) = Login1ManagerProxyBlocking::new(conn) {
                        match proxy.inhibit(
                            "idle",
                            "CosmicCaffeineLand",
                            "User requested to prevent idle",
                            "block"
                        ) {
                            Ok(fd) => {
                                eprintln!("DEBUG: Idle inhibitor attivato");
                                *idle_fd.lock().unwrap() = Some(fd);
                            }
                            Err(e) => eprintln!("WARN: Fallito inhibit idle: {}", e),
                        }
                    }
                }
            } else {
                if idle_fd.lock().unwrap().take().is_some() {
                    eprintln!("DEBUG: Idle inhibitor disattivato");
                }
            }
        });
    }

    fn update_sleep_inhibitor(&mut self) {
        let sleep_fd = self.sleep_fd.clone();
        let system_conn = self.system_conn.clone();
        let is_active = self.is_active;

        tokio::task::spawn_blocking(move || {
            if is_active {
                let conn_guard = system_conn.lock().unwrap();
                if let Some(conn) = conn_guard.as_ref() {
                    if let Ok(proxy) = Login1ManagerProxyBlocking::new(conn) {
                        match proxy.inhibit(
                            "sleep",
                            "CosmicCaffeineLand",
                            "User requested to prevent sleep",
                            "block"
                        ) {
                            Ok(fd) => {
                                eprintln!("DEBUG: Sleep inhibitor attivato");
                                *sleep_fd.lock().unwrap() = Some(fd);
                            }
                            Err(e) => eprintln!("WARN: Fallito inhibit sleep: {}", e),
                        }
                    }
                }
            } else {
                if sleep_fd.lock().unwrap().take().is_some() {
                    eprintln!("DEBUG: Sleep inhibitor disattivato");
                }
            }
        });
    }

    fn update_lid_inhibitor(&mut self) {
        let lid_fd = self.lid_fd.clone();
        let system_conn = self.system_conn.clone();
        let is_active = self.is_active;

        tokio::task::spawn_blocking(move || {
            if is_active {
                let conn_guard = system_conn.lock().unwrap();
                if let Some(conn) = conn_guard.as_ref() {
                    if let Ok(proxy) = Login1ManagerProxyBlocking::new(conn) {
                        match proxy.inhibit(
                            "handle-lid-switch",
                            "CosmicCaffeineLand",
                            "User requested to prevent lid action",
                            "block"
                        ) {
                            Ok(fd) => {
                                eprintln!("DEBUG: Lid switch inhibitor attivato");
                                *lid_fd.lock().unwrap() = Some(fd);
                            }
                            Err(e) => eprintln!("WARN: Fallito inhibit lid-switch: {}", e),
                        }
                    }
                }
            } else {
                if lid_fd.lock().unwrap().take().is_some() {
                    eprintln!("DEBUG: Lid switch inhibitor disattivato");
                }
            }
        });
    }

    fn update_screensaver_inhibitor(&mut self) {
        let screensaver_cookie = self.screensaver_cookie.clone();
        let session_conn = self.session_conn.clone();
        let is_active = self.is_active;

        tokio::task::spawn_blocking(move || {
            if is_active {
                let conn_guard = session_conn.lock().unwrap();
                if let Some(conn) = conn_guard.as_ref() {
                    if let Ok(proxy) = ScreenSaverProxyBlocking::new(conn) {
                        match proxy.inhibit(
                            "io.github.submax82.CosmicCaffeineLand",
                            "User requested to prevent screensaver"
                        ) {
                            Ok(cookie) => {
                                *screensaver_cookie.lock().unwrap() = Some(cookie);
                                eprintln!("DEBUG: ScreenSaver inhibitor attivato (cookie: {})", cookie);
                                return;
                            }
                            Err(e) => eprintln!("WARN: Fallito inhibit ScreenSaver: {}", e),
                        }
                    }
                }
            } else {
                let cookie = screensaver_cookie.lock().unwrap().take();

                if let Some(cookie) = cookie {
                    let conn_guard = session_conn.lock().unwrap();
                    if let Some(conn) = conn_guard.as_ref() {
                        if let Ok(proxy) = ScreenSaverProxyBlocking::new(conn) {
                            let _ = proxy.un_inhibit(cookie);
                        }
                    }
                    eprintln!("DEBUG: ScreenSaver inhibitor disattivato");
                }
            }
        });
    }
}

impl Drop for CaffeineApplet {
    fn drop(&mut self) {
        let mut state_guard = self.wayland_state.lock().unwrap();
        let mut eq_guard = self.event_queue.lock().unwrap();

        if let (Some(state), Some(eq)) = (state_guard.as_mut(), eq_guard.as_mut()) {
            if let Some(inhibitor) = state.inhibitor.take() {
                inhibitor.destroy();
            }
            if let Some(surface) = state.surface.take() {
                surface.destroy();
            }
            let _ = eq.roundtrip(state);
        }

        self.idle_fd.lock().unwrap().take();
        self.sleep_fd.lock().unwrap().take();
        self.lid_fd.lock().unwrap().take();

        // Disattiva lo screensaver prima di droppare
        let cookie = self.screensaver_cookie.lock().unwrap().take();
        if let Some(cookie) = cookie {
            let conn_guard = self.session_conn.lock().unwrap();
            if let Some(conn) = conn_guard.as_ref() {
                if let Ok(proxy) = ScreenSaverProxyBlocking::new(conn) {
                    let _ = proxy.un_inhibit(cookie);
                }
            }
        }

        // Chiudi esplicitamente le connessioni D-Bus
        *self.session_conn.lock().unwrap() = None;
        *self.system_conn.lock().unwrap() = None;
        eprintln!("DEBUG: Connessioni D-Bus chiuse nel drop");
    }
}

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<CaffeineApplet>(())
}
