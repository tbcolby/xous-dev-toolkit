//! system.md snippets
use num_traits::*;

#[derive(Debug, num_derive::FromPrimitive, num_derive::ToPrimitive)]
enum AppOp {
    WifiChanged = 20,
}

fn com_demo(xns: &xous_names::XousNames) -> Result<(), xous::Error> {
    // Connection (mut: some calls, e.g. get_batt_stats_blocking, take &mut self)
    let mut com = com::Com::new(xns)?;

    // ---- Battery & Power ----
    let stats: com::BattStats = com.get_batt_stats_blocking()?;
    log::info!("{} mV, {}%, {} mA, {} mAh left", stats.voltage, stats.soc, stats.current, stats.remaining_capacity);
    if com.is_charging()? {
        log::info!("Device is charging");
    }
    com.request_charging()?; // ask the EC to enable charging (if USB connected)

    com.set_boost(true)?; // boost mode on (more power draw)
    if let Some(current_ma) = com.get_standby_current()? {
        log::info!("Standby draw: {}mA", current_ma);
    }

    // ---- Sensors ----
    // Accelerometer: raw two's-complement samples; 4th value is the IMU id, not temperature
    let (x, y, z, _id) = com.gyro_read_blocking()?;
    let (x, y, z) = (x as i16, y as i16, z as i16);

    // ---- Backlight (0-255 each) ----
    com.set_backlight(200, 0)?; // main display, keyboard

    // ---- WiFi power ----
    com.wifi_disable()?;
    let _rev = com.wifi_reset()?;

    // ---- SSID scanning ----
    // ssid_scan_updated()/ssid_fetch_as_string() are #[deprecated]; read the aggregated
    // scan results from the connection manager instead.
    com.set_ssid_scanning(true)?;
    let netmgr = net::NetManager::new();
    let (ssids, scan_state) = netmgr.wifi_get_ssid_list()?;
    for r in &ssids {
        log::info!("  {} (rssi -{} dBm)", r.name, r.rssi); // rssi is reported as a positive u8
    }
    com.set_ssid_scanning(false)?;

    // ---- Device information ----
    let (revision, is_dirty) = com.get_ec_git_rev()?;
    log::info!("EC rev: {:08x}{}", revision, if is_dirty { " (dirty)" } else { "" });
    let semver = com.get_ec_sw_tag()?;
    log::info!("EC version: {}", semver);
    Ok(())
}

fn display_demo(xns: &xous_names::XousNames) -> Result<(), xous::Error> {
    let gam = gam::Gam::new(xns)?;
    gam.set_vibe(true)?; // Buzz
    gam.set_vibe(false)?; // Stop
    gam.selftest(2000); // LCD test pattern for 2000 ms
    Ok(())
}

// ---- WiFi state via the net service ----
fn net_demo(sid: xous::SID) -> Result<(), xous::Error> {
    let mut netmgr = net::NetManager::new(); // no args

    // Subscribe: updates arrive as MEMORY messages carrying com::WlanStatusIpc
    netmgr.wifi_state_subscribe(xous::connect(sid)?, AppOp::WifiChanged.to_u32().unwrap())?;

    // Connection manager control
    netmgr.connection_manager_run()?; // start auto-connect
    netmgr.connection_manager_wifi_on_and_run()?; // WiFi on + manager
    netmgr.connection_manager_wifi_off_and_stop()?; // WiFi off + stop manager

    // Current IP configuration (None = not connected)
    if let Some(ipv4) = netmgr.get_ipv4_config() {
        log::info!("IP: {:?}", ipv4.addr);
    }

    netmgr.wifi_state_unsubscribe()?;
    Ok(())
}

fn handle_wifi(msg: &xous::MessageEnvelope) {
    match FromPrimitive::from_usize(msg.body.id()) {
        Some(AppOp::WifiChanged) => {
            let buffer = unsafe { xous_ipc::Buffer::from_memory_message(msg.body.memory_message().unwrap()) };
            let status = com::WlanStatus::from_ipc(buffer.to_original::<com::WlanStatusIpc, _>().unwrap());
            log::info!("WiFi state: {:?}, ssid {:?}", status.link_state, status.ssid.map(|s| s.name.to_string()));
        }
        None => {}
    }
}
