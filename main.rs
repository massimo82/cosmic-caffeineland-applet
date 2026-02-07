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
// MOLTO IMPORTANTE PER IL FUNZIONAMENTO:
// Connessioni D-Bus persistenti create in init(), session_conn e system_conn che vengono riutilizzate

use cosmic::iced::{Length, Subscription};
use cosmic::widget::{button, container};
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
    // Connessioni D-Bus mantenute attive
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

        // Inizializza le connessioni D-Bus
        let session_conn = Arc::new(Mutex::new(DbusConnection::session().ok()));
        let system_conn = Arc::new(Mutex::new(DbusConnection::system().ok()));

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
        let icon = if self.is_active { "☕" } else { "😴" };

        let content = button::standard(icon)
            .on_press(Message::Toggle)
            .padding(2);

        container(content)
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into()
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
    }
}

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<CaffeineApplet>(())
}
