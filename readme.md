# CosmicCaffeineLand - Applet for COSMIC Desktop

## Description
**CosmicCaffeineLand** is a panel applet for COSMIC Desktop (System76) that prevents the system from going to sleep or turning off the screen, similar to the Caffeine utility for macOS.
No timed modes or process-linked features are planned. I prefer useful integrations and having a straightforward applet rather than many niche functions. For example, battery monitoring integration and laptop lid status detection are fundamental.

## Main Features

### ☕ Core Functionality
- **Sleep/screensaver inhibition**: Uses the Wayland `idle-inhibit-unstable-v1` protocol to prevent system suspension
- **Simple toggle**: Click the icon to enable/disable
- **Visual indicator**: Coffee cup icon ☕ that changes state
  - Full cup = active
  - Empty cup = inactive

### 🔋 Smart Battery Protection
- **Battery monitoring**: Checks battery status every x seconds via UPower
- **Automatic deactivation**: When battery drops below 10% and the PC is not charging
- **Reactivation lock**: Prevents reactivating Caffeine until the battery level rises or the charger is connected
- **Low battery notification**: _"Caffeine has been disabled and cannot be reactivated, battery level too low"_

### 💻 Screen Closure Detection
- **Lid monitoring**: Checks laptop lid status every y seconds
- **Hardware detection**: Only works if the hardware supports detection via ACPI (`/proc/acpi/button/lid`)
- **Automatic deactivation**: When the screen is physically closed, Caffeine automatically deactivates
- **Closure notification**: _"Caffeine has been disabled after detecting screen closure"_
- **Safety**: Only operates when it can be certain of the lid state through hardware

### 🔧 Technical Features
- **No systemd dependency**: Completely independent
- **Native Wayland support**: Uses `zwp_idle_inhibit_manager_v1`
- **COSMIC integration**: Integrates seamlessly with the desktop's light and dark theme
- **Resource management**: Automatic cleanup of Wayland resources on shutdown

## Project Setup

### Build
```bash
# Create the project
cargo new cosmic-caffeineland-applet
cd cosmic-caffeineland-applet

# Copy the code into src/main.rs
# Update Cargo.toml with dependencies

# Build
cargo build --release

# The executable will be in target/release/cosmic-caffeineland
```

### Installation
```bash
# Copy the executable to the appropriate directory for COSMIC applets
# (the exact location depends on COSMIC configuration)
sudo cp target/release/cosmic-caffeineland /usr/bin/
```

## Compatibility
- ✅ COSMIC Desktop 1.0 (stable)
- ✅ Wayland idle-inhibit protocol (latest stable version)
- ✅ Any Linux system with UPower for battery monitoring
- ✅ Laptops with ACPI support for lid detection

## Application ID
`com.system76.CosmicCaffeineLand`

## Important Technical Detail
**Continuously creating and destroying D-Bus connections** caused problems with inhibitors, especially for the screensaver.

**Persistent connections** ensure that:
1. The session/system connection remains open for as long as caffeine stays active
2. Proxies can maintain state correctly
3. Inhibitor cookies/file descriptors remain valid

Therefore, we maintain connections as fields of the main struct.
To be more precise, the D-Bus connection is opened and closed at each activation/deactivation cycle.
The main characteristics are:

1. **`init()`**: D-Bus connections start as `None` instead of being created at startup

2. **`update_inhibitor_state()`**: 
   - **On activation** (`is_active = true`): new connections are opened
   - **On deactivation** (`is_active = false`): connections are closed by setting to `None`

3. **`Drop`**: Explicit closure of connections even when the applet terminates

This ensures that:
- Fresh D-Bus connections are established with each reactivation
- Connections are released upon deactivation
- There are no unused persistent connections when caffeine is inactive
