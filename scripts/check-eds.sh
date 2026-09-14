#!/usr/bin/env bash
# Verifies the Evolution Data Server mirror (docs/06-integracion-gnome.md section 4).
# Read-only: lists ugc-* sources, opens the first one and counts its events, and
# checks that GNOME Shell's calendar server is reachable. Uses python3 + Gio,
# which Ubuntu ships. Exit 1 if no ugc-* source exists or it has no events.
set -euo pipefail
python3 - <<'PY'
import sys, gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib

bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)

def call(dest, path, iface, method, params=None, sig=None):
    return bus.call_sync(dest, path, iface, method, params, sig, Gio.DBusCallFlags.NONE, 10000, None)

names = call('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'ListNames').unpack()[0]
def pick(prefix):
    c = sorted([n for n in names if n.startswith(prefix)], key=lambda n: int(n[len(prefix):] or 0))
    if not c: print(f'FAIL: no bus name starting with {prefix}'); sys.exit(1)
    return c[-1]
sources_bus = pick('org.gnome.evolution.dataserver.Sources')
cal_bus = pick('org.gnome.evolution.dataserver.Calendar')
print(f'registry bus: {sources_bus}   calendar factory bus: {cal_bus}')

objs = call(sources_bus, '/org/gnome/evolution/dataserver/SourceManager', 'org.freedesktop.DBus.ObjectManager', 'GetManagedObjects').unpack()[0]
ugc = []
for path, ifaces in objs.items():
    src = ifaces.get('org.gnome.evolution.dataserver.Source')
    if src and str(src.get('UID', '')).startswith('ugc-'):
        data = src.get('Data', '')
        sel = 'Selected=true' in data; en = 'Enabled=true' in data; alarms = 'IncludeMe=false' in data
        name = next((l.split('=',1)[1] for l in data.splitlines() if l.startswith('DisplayName=')), '?')
        ugc.append((src['UID'], name, sel, en, alarms))
if not ugc:
    print('FAIL: no ugc-* sources in Evolution Data Server'); sys.exit(1)
print(f'{len(ugc)} ugc-* sources:')
ok = True
for uid, name, sel, en, alarms in ugc:
    flag = 'ok' if (sel and en and alarms) else 'CHECK'
    if flag == 'CHECK': ok = False
    print(f'  [{flag}] {uid}  "{name}"  Selected={sel} Enabled={en} AlarmsExcluded={alarms}')

uid = ugc[0][0]
path, dest = call(cal_bus, '/org/gnome/evolution/dataserver/CalendarFactory', 'org.gnome.evolution.dataserver.CalendarFactory',
                  'OpenCalendar', GLib.Variant('(s)', (uid,))).unpack()
call(dest, path, 'org.gnome.evolution.dataserver.Calendar', 'Open')
objects = call(dest, path, 'org.gnome.evolution.dataserver.Calendar', 'GetObjectList', GLib.Variant('(s)', ('#t',))).unpack()[0]
call(dest, path, 'org.gnome.evolution.dataserver.Calendar', 'Close')
valarm = sum(1 for o in objects if 'BEGIN:VALARM' in o)
print(f'{uid}: {len(objects)} VEVENT objects, {valarm} with VALARM (must be 0)')
if valarm: ok = False
if not objects: print('FAIL: first ugc source has no events'); sys.exit(1)

if 'org.gnome.Shell.CalendarServer' in names:
    print('gnome-shell-calendar-server is on the bus')
else:
    print('WARN: org.gnome.Shell.CalendarServer not on the bus (not running under GNOME Shell?)')
sys.exit(0 if ok else 1)
PY
