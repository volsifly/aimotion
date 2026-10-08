use aimotion_desktop::{GRID, Player, Reply};
use fs2::FileExt;
use gpui::{prelude::*, *};
use std::{collections::HashSet, fs, fs::OpenOptions, path::PathBuf, sync::mpsc, time::{Duration, Instant}};

struct MotionView {
    player: Player,
}

impl MotionView {
    fn new(path: PathBuf, cx: &mut Context<Self>) -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut last_bytes = Vec::new();
            loop {
                if let Ok(bytes) = fs::read(&path) {
                    if bytes != last_bytes {
                        last_bytes = bytes.clone();
                        match serde_json::from_slice::<Reply>(&bytes) {
                            Ok(reply) => if sender.send(reply).is_err() { break; },
                            Err(error) => eprintln!("Ignoring invalid reply: {error}"),
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        });
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(20)).await;
                if this.update(cx, |view, cx| {
                    let now = Instant::now();
                    let mut changed = false;
                    // Apply only the newest reply if publishing outruns the UI.
                    if let Some(reply) = receiver.try_iter().last() {
                        match view.player.set_reply(reply, now) {
                            Ok(update) => changed |= update,
                            Err(error) => eprintln!("Ignoring invalid sequence: {error}"),
                        }
                    }
                    changed |= view.player.tick(now);
                    if changed { cx.notify(); }
                }).is_err() { break; }
            }
        }).detach();
        Self { player: Player::default() }
    }
}

impl Render for MotionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let lit: HashSet<_> = self.player.positions().into_iter().collect();
        let mut content = div().flex().flex_col().gap(px(2.)).p(px(12.)).bg(rgb(0x101414))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
            .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.quit());
        for row in 0..GRID {
            content = content.child(div().flex().gap(px(2.)).children((0..GRID).map(|column| {
                div().size(px(12.)).rounded(px(2.)).bg(rgb(if lit.contains(&(row * GRID + column)) { 0xbfff5c } else { 0x202b29 }))
            })));
        }
        content = content.child(div().id("reply").mt(px(10.)).h(px(81.)).w(px(222.))
            .overflow_y_scroll().text_size(px(13.)).text_color(rgb(0xedf4ec))
            .child(self.player.text.clone()));
        content
    }
}

// EWMH keeps the widget above other windows and visible on every workspace.
// GPUI uses X11 here, including XWayland on GNOME Wayland sessions.
fn pin_window(pid: u32) -> Result<u32, Box<dyn std::error::Error>> {
    use x11rb::{connection::Connection, protocol::xproto::*, wrapper::ConnectionExt as _};
    let (connection, screen) = x11rb::connect(None)?;
    let root = connection.setup().roots[screen].root;
    let atom = |name: &str| -> Result<u32, Box<dyn std::error::Error>> {
        Ok(connection.intern_atom(false, name.as_bytes())?.reply()?.atom)
    };
    let pid_atom = atom("_NET_WM_PID")?;
    let clients_atom = atom("_NET_CLIENT_LIST")?;
    let state = atom("_NET_WM_STATE")?;
    let above = atom("_NET_WM_STATE_ABOVE")?;
    let sticky = atom("_NET_WM_STATE_STICKY")?;
    let desktop = atom("_NET_WM_DESKTOP")?;
    for _ in 0..100 {
        let clients = connection.get_property(false, root, clients_atom, AtomEnum::WINDOW, 0, 4096)?.reply()?;
        for window in clients.value32().into_iter().flatten() {
            let value = connection.get_property(false, window, pid_atom, AtomEnum::CARDINAL, 0, 1)?.reply()?;
            if value.value32().and_then(|mut values| values.next()) != Some(pid) { continue; }
            let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
            connection.send_event(false, root, mask, ClientMessageEvent::new(32, window, state, [1, above, sticky, 1, 0]))?;
            connection.send_event(false, root, mask, ClientMessageEvent::new(32, window, desktop, [u32::MAX, 1, 0, 0, 0]))?;
            connection.change_property32(PropMode::REPLACE, window, atom("_MOTIF_WM_HINTS")?, atom("_MOTIF_WM_HINTS")?, &[2, 0, 0, 0, 0])?;
            connection.flush()?;
            return Ok(window);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err("Desktop window did not appear in the window manager".into())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let data_file = match args.next().as_deref() {
        Some("--data-file") => PathBuf::from(args.next().ok_or("--data-file requires a path")?),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("current.json"),
        _ => return Err("Usage: aimotion-desktop [--data-file PATH]".into()),
    };
    let data_file = if data_file.is_absolute() { data_file } else { std::env::current_dir()?.join(data_file) };
    let root = data_file.parent().ok_or("Invalid data file path")?.to_path_buf();
    let lock = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(root.join(".desktop.lock"))?;
    if lock.try_lock_exclusive().is_err() { return Ok(()); }
    let status_path = root.join(".desktop-status.json");
    let _ = fs::remove_file(&status_path);
    let pid = std::process::id();
    Application::new().run(move |cx: &mut App| {
        cx.on_window_closed(|cx| if cx.windows().is_empty() { cx.quit(); }).detach();
        let bounds = cx.primary_display().map(|display| {
            let screen = display.bounds();
            Bounds::new(point(screen.origin.x + screen.size.width - px(270.), screen.origin.y + px(48.)), size(px(246.), px(339.)))
        }).unwrap_or_else(|| Bounds::centered(None, size(px(246.), px(339.)), cx));
        cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions { title: Some("AI Motion".into()), ..Default::default() }),
            focus: false,
            is_resizable: false,
            is_minimizable: false,
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some("ai-motion".into()),
            ..Default::default()
        }, |_, cx| cx.new(|cx| MotionView::new(data_file.clone(), cx))).expect("Unable to create GPUI window");
        std::thread::spawn(move || match pin_window(pid) {
            Ok(window) => {
                let status = serde_json::json!({"pid": pid, "window": window, "backend": "gpui-x11", "data_file": data_file});
                if let Err(error) = fs::write(status_path, status.to_string()) { eprintln!("Status write failed: {error}"); }
            }
            Err(error) => eprintln!("Unable to pin desktop window: {error}"),
        });
    });
    let _ = fs::remove_file(root.join(".desktop-status.json"));
    drop(lock);
    Ok(())
}
