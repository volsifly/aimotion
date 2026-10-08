#!/usr/bin/env python3
"""Install a launcher and login autostart for the already-built desktop app."""
from pathlib import Path

root = Path(__file__).resolve().parent.parent
binary = next((p for p in [root / 'target/release/aimotion-desktop', root / 'target/debug/aimotion-desktop'] if p.is_file()), None)
if binary is None:
    raise SystemExit('Run scripts/run-desktop.sh first.')

def desktop_quote(path):
    return '"' + str(path).replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$') + '"'

entry = f'''[Desktop Entry]
Type=Application
Name=AI Motion
Comment=16×16 pixel canvas for Agent expression
Exec={desktop_quote(binary)} --data-file {desktop_quote(root / 'current.json')}
Icon=applications-graphics
Terminal=false
Categories=Graphics;
StartupWMClass=ai-motion
X-GNOME-Autostart-enabled=true
'''
for relative in ['.local/share/applications/ai-motion.desktop', '.config/autostart/ai-motion.desktop']:
    path = Path.home() / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(entry, encoding='utf-8')
    print(path)
