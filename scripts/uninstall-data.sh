#!/usr/bin/env bash
# Removes all user data of Unified Google Calendar: config, database, tokens,
# logs, and the ugc-* mirror sources in Evolution Data Server. Asks first.
# Does not uninstall the .deb (use: sudo apt remove unified-google-calendar).
set -euo pipefail
CFG="$HOME/.config/unified-google-calendar"
DATA="$HOME/.local/share/unified-google-calendar"
echo "This deletes:"
echo "  $CFG  (oauth.json, verification files)"
echo "  $DATA (data.db, tokens.bin, logs)"
echo "  $HOME/.local/share/icons/hicolor/*/apps/unified-google-calendar.png (day-of-month icon)"
echo "  Evolution Data Server sources named ugc-* and their events"
read -r -p "Continue? [y/N] " a; [ "${a,,}" = "y" ] || { echo "aborted"; exit 1; }

python3 - <<'PY' || echo "warning: could not remove EDS sources (is evolution-source-registry running?)"
import gi; gi.require_version('Gio', '2.0')
from gi.repository import Gio
bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
names = bus.call_sync('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','ListNames',None,None,0,5000,None).unpack()[0]
c = sorted([n for n in names if n.startswith('org.gnome.evolution.dataserver.Sources')])
if not c: raise SystemExit(1)
dest = c[-1]
objs = bus.call_sync(dest,'/org/gnome/evolution/dataserver/SourceManager','org.freedesktop.DBus.ObjectManager','GetManagedObjects',None,None,0,10000,None).unpack()[0]
n = 0
for path, ifaces in objs.items():
    src = ifaces.get('org.gnome.evolution.dataserver.Source')
    if src and str(src.get('UID','')).startswith('ugc-') and 'org.gnome.evolution.dataserver.Source.Removable' in ifaces:
        bus.call_sync(dest, path, 'org.gnome.evolution.dataserver.Source.Removable', 'Remove', None, None, 0, 10000, None); n += 1
print(f'removed {n} EDS sources')
PY
rm -rf "$CFG" "$DATA"
rm -f "$HOME"/.local/share/icons/hicolor/*/apps/unified-google-calendar.png
echo "done"
