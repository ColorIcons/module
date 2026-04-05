#!/bin/sh
# ColorOSIconsPatch - customize.sh

SKIPMOUNT=false
PROPFILE=true
POSTFSDATA=true
LATESTARTSERVICE=true

MODID="$(grep -m1 '^id=' "$MODPATH/module.prop" | cut -d= -f2)"
[ -z "$MODID" ] && MODID="ColorOSIconsPatch"

PERSIST_BASE="/data/adb"
PERSIST_DIR="$PERSIST_BASE/$MODID"

UXICONS_DST="$MODPATH/uxicons"

mkdir -p "$UXICONS_DST"

CIP_BIN="$MODPATH/cip"

set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm "$CIP_BIN" 0 0 0755

ui_print "- ColorOS Icons Patch"
ui_print "- Using persist dir: $PERSIST_DIR"

mkdir -p "$PERSIST_DIR" || abort "Create persist dir failed"

[ -x "$CIP_BIN" ] || abort "cip binary not found"

ui_print "- Running cip init..."
"$CIP_BIN" config init || ui_print "- cip init skipped (config exists or error)"
