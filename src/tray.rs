use std::sync::mpsc::Sender;

#[derive(Debug)]
pub enum Command { Toggle, Show, Hide, Quit }

#[derive(Debug)]
pub struct MotionTray(pub Sender<Command>);

impl ksni::Tray for MotionTray {
    fn id(&self) -> String { "ai-motion".into() }
    fn title(&self) -> String { "AI Motion".into() }
    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let mut data = Vec::with_capacity(32 * 32 * 4);
        for y in 0..32 {
            for x in 0..32 {
                let lit = (8..24).contains(&x) && (8..24).contains(&y)
                    && (x < 12 || x >= 20 || y < 12 || y >= 20);
                data.extend_from_slice(if lit { &[255, 191, 255, 92] } else { &[0, 0, 0, 0] });
            }
        }
        vec![ksni::Icon { width: 32, height: 32, data }]
    }
    fn activate(&mut self, _x: i32, _y: i32) { let _ = self.0.send(Command::Toggle); }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem { label: "显示窗口".into(), activate: Box::new(|tray: &mut Self| { let _ = tray.0.send(Command::Show); }), ..Default::default() }.into(),
            StandardItem { label: "隐藏窗口".into(), activate: Box::new(|tray: &mut Self| { let _ = tray.0.send(Command::Hide); }), ..Default::default() }.into(),
            ksni::MenuItem::Separator,
            StandardItem { label: "退出".into(), activate: Box::new(|tray: &mut Self| { let _ = tray.0.send(Command::Quit); }), ..Default::default() }.into(),
        ]
    }
}

pub fn set_visible(window: u32, visible: bool) -> Result<(), Box<dyn std::error::Error>> {
    use x11rb::{connection::Connection, protocol::xproto::*, wrapper::ConnectionExt as _};
    let (connection, _) = x11rb::connect(None)?;
    if visible {
        let atom = |name: &str| -> Result<u32, Box<dyn std::error::Error>> {
            Ok(connection.intern_atom(false, name.as_bytes())?.reply()?.atom)
        };
        // The window manager clears EWMH state on unmap. Restore it before mapping.
        let states = [atom("_NET_WM_STATE_ABOVE")?, atom("_NET_WM_STATE_STICKY")?,
            atom("_NET_WM_STATE_SKIP_TASKBAR")?, atom("_NET_WM_STATE_SKIP_PAGER")?];
        connection.change_property32(PropMode::REPLACE, window, atom("_NET_WM_STATE")?, AtomEnum::ATOM, &states)?.check()?;
        connection.change_property32(PropMode::REPLACE, window, atom("_NET_WM_DESKTOP")?, AtomEnum::CARDINAL, &[u32::MAX])?.check()?;
        connection.change_property32(PropMode::REPLACE, window, atom("_NET_WM_USER_TIME")?, AtomEnum::CARDINAL, &[0])?.check()?;
        connection.map_window(window)?.check()?;
    }
    else { connection.unmap_window(window)?.check()?; }
    connection.flush()?;
    Ok(())
}
