use anyhow::{bail, Context, Result};
use std::{thread, time::{Duration, Instant}};
use x11rb::{
    connection::Connection,
    protocol::{
        composite::{ConnectionExt as CompositeExt, Redirect},
        damage::{ConnectionExt as DamageExt, ReportLevel},
        xproto::{ConnectionExt as XprotoExt, CreateWindowAux, WindowClass},
        Event,
    },
    COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT, NONE,
};

fn main() -> Result<()> {
    let (conn, screen_num) = x11rb::connect(None).context("connect to X11 display")?;
    let screen = &conn.setup().roots[screen_num];

    let composite = conn
        .composite_query_version(0, 4)
        .context("query XComposite version")?
        .reply()
        .context("negotiate XComposite version")?;
    if (composite.major_version, composite.minor_version) < (0, 2) {
        bail!("XComposite 0.2 or newer is required for named window pixmaps");
    }

    let damage = conn
        .damage_query_version(1, 1)
        .context("query XDamage version")?
        .reply()
        .context("negotiate XDamage version")?;
    if damage.major_version < 1 {
        bail!("XDamage 1.0 or newer is required");
    }

    let window = conn.generate_id().context("allocate probe window id")?;
    conn.create_window(
        COPY_DEPTH_FROM_PARENT,
        window,
        screen.root,
        8,
        8,
        64,
        64,
        0,
        WindowClass::INPUT_OUTPUT,
        COPY_FROM_PARENT,
        &CreateWindowAux::new()
            .background_pixel(screen.white_pixel)
            .override_redirect(1),
    )
    .context("create isolated probe window")?
    .check()
    .context("create isolated probe window")?;
    conn.map_window(window)
        .context("map isolated probe window")?
        .check()
        .context("map isolated probe window")?;

    // Redirect only this disposable probe window. MRX's production WM does
    // not redirect managed clients until its own renderer can own that role.
    conn.composite_redirect_window(window, Redirect::MANUAL)
        .context("redirect probe window with XComposite")?
        .check()
        .context("redirect probe window with XComposite")?;

    let pixmap = conn.generate_id().context("allocate named pixmap id")?;
    conn.composite_name_window_pixmap(window, pixmap)
        .context("name redirected probe window pixmap")?
        .check()
        .context("name redirected probe window pixmap")?;
    let geometry = conn
        .get_geometry(pixmap)
        .context("query named pixmap geometry")?
        .reply()
        .context("query named pixmap geometry")?;
    if (geometry.width, geometry.height) != (64, 64) {
        bail!(
            "unexpected named pixmap dimensions: {}x{}",
            geometry.width,
            geometry.height
        );
    }

    let damage_id = conn.generate_id().context("allocate XDamage id")?;
    conn.damage_create(damage_id, window, ReportLevel::NON_EMPTY)
        .context("create XDamage tracker")?
        .check()
        .context("create XDamage tracker")?;
    conn.flush().context("flush initial capture setup")?;

    // Clear startup/map damage before creating a fresh deterministic update.
    while conn.poll_for_event().context("drain startup X11 events")?.is_some() {}
    conn.damage_subtract(damage_id, NONE, NONE)
        .context("clear initial XDamage region")?
        .check()
        .context("clear initial XDamage region")?;
    conn.clear_area(false, window, 0, 0, 32, 32)
        .context("draw into probe window")?
        .check()
        .context("draw into probe window")?;
    conn.flush().context("flush probe draw")?;

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut damage_events = 0_u32;
    while Instant::now() < deadline {
        match conn.poll_for_event().context("poll XDamage event")? {
            Some(Event::DamageNotify(event)) if event.damage == damage_id => {
                damage_events += 1;
                break;
            }
            Some(_) => {}
            None => thread::sleep(Duration::from_millis(5)),
        }
    }
    if damage_events == 0 {
        bail!("XDamage did not report the probe window update within 2 seconds");
    }

    conn.damage_subtract(damage_id, NONE, NONE)
        .context("acknowledge XDamage event")?
        .check()
        .context("acknowledge XDamage event")?;
    conn.damage_destroy(damage_id)
        .context("destroy XDamage tracker")?
        .check()
        .context("destroy XDamage tracker")?;
    conn.free_pixmap(pixmap)
        .context("free named pixmap")?
        .check()
        .context("free named pixmap")?;
    conn.composite_unredirect_window(window, Redirect::MANUAL)
        .context("unredirect probe window")?
        .check()
        .context("unredirect probe window")?;
    conn.destroy_window(window)
        .context("destroy probe window")?
        .check()
        .context("destroy probe window")?;
    conn.flush().context("flush probe cleanup")?;

    println!(
        "PASS: XComposite {}.{} named a {}x{} redirected window pixmap",
        composite.major_version, composite.minor_version, geometry.width, geometry.height
    );
    println!(
        "PASS: XDamage {}.{} delivered {} non-empty damage event",
        damage.major_version, damage.minor_version, damage_events
    );
    println!("NOTE: capture ran only on a disposable probe window; managed clients were not redirected");
    Ok(())
}
