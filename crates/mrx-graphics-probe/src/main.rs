use anyhow::{bail, Context, Result};
use std::{thread, time::{Duration, Instant}};
use x11rb::{
    connection::Connection,
    protocol::{
        composite::{ConnectionExt as CompositeExt, Redirect},
        damage::{ConnectionExt as DamageExt, ReportLevel},
        xproto::{ConnectionExt as XprotoExt, CreateWindowAux, ImageFormat, Rectangle, WindowClass},
        render::{ConnectionExt as RenderExt, Color, CreatePictureAux, PictOp},
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
    // Validate the XRender composition path off-screen. This deliberately
    // does not claim compositor-manager ownership or redirect managed clients.
    let render_version = conn
        .render_query_version(0, 11)
        .context("query XRender version")?
        .reply()
        .context("negotiate XRender version")?;
    let formats = conn
        .render_query_pict_formats()
        .context("query XRender picture formats")?
        .reply()
        .context("read XRender picture formats")?;
    let window_visual = conn
        .get_window_attributes(window)
        .context("query probe window visual")?
        .reply()
        .context("read probe window visual")?
        .visual;
    let visual_format = |visual: u32| {
        formats.screens[screen_num]
            .depths
            .iter()
            .flat_map(|depth| depth.visuals.iter())
            .find(|entry| entry.visual == visual)
            .map(|entry| entry.format)
    };
    let source_format = visual_format(window_visual)
        .context("XRender has no picture format for the probe window visual")?;
    let destination_format = visual_format(screen.root_visual)
        .context("XRender has no picture format for the root visual")?;

    let destination_pixmap = conn.generate_id().context("allocate XRender destination pixmap")?;
    conn.create_pixmap(screen.root_depth, destination_pixmap, screen.root, 64, 64)
        .context("create XRender destination pixmap")?
        .check()
        .context("create XRender destination pixmap")?;
    let source_picture = conn.generate_id().context("allocate XRender source picture")?;
    conn.render_create_picture(source_picture, pixmap, source_format, &CreatePictureAux::new())
        .context("create picture for XComposite named pixmap")?
        .check()
        .context("create picture for XComposite named pixmap")?;
    let destination_picture = conn.generate_id().context("allocate XRender destination picture")?;
    conn.render_create_picture(destination_picture, destination_pixmap, destination_format, &CreatePictureAux::new())
        .context("create off-screen XRender destination picture")?
        .check()
        .context("create off-screen XRender destination picture")?;
    conn.render_fill_rectangles(
        PictOp::CLEAR,
        destination_picture,
        Color::default(),
        &[Rectangle { x: 0, y: 0, width: 64, height: 64 }],
    )
    .context("clear XRender destination")?
    .check()
    .context("clear XRender destination")?;
    conn.render_composite(
        PictOp::OVER,
        source_picture,
        NONE,
        destination_picture,
        0,
        0,
        0,
        0,
        0,
        0,
        64,
        64,
    )
    .context("composite captured pixmap with XRender")?
    .check()
    .context("composite captured pixmap with XRender")?;
    conn.flush().context("flush XRender composition")?;
    let rendered = conn
        .get_image(ImageFormat::Z_PIXMAP, destination_pixmap, 0, 0, 64, 64, u32::MAX)
        .context("read back XRender destination pixels")?
        .reply()
        .context("read back XRender destination pixels")?;
    if !rendered.data.iter().any(|byte| *byte != 0) {
        bail!("XRender composition produced an empty off-screen destination");
    }
    conn.render_free_picture(source_picture)
        .context("free XRender source picture")?
        .check()
        .context("free XRender source picture")?;
    conn.render_free_picture(destination_picture)
        .context("free XRender destination picture")?
        .check()
        .context("free XRender destination picture")?;
    conn.free_pixmap(destination_pixmap)
        .context("free XRender destination pixmap")?
        .check()
        .context("free XRender destination pixmap")?;

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
    println!(
        "PASS: XRender {}.{} composited the captured pixmap into a readable off-screen destination",
        render_version.major_version, render_version.minor_version
    );
    println!("NOTE: capture ran only on a disposable probe window; managed clients were not redirected");
    Ok(())
}
