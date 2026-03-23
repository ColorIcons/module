#!/system/bin/sh
# customize.sh
SKIPMOUNT=false
PROPFILE=true
POSTFSDATA=true
LATESTARTSERVICE=true

MODID="$(grep -m1 '^id=' "$MODPATH/module.prop" | cut -d= -f2)"
[ -z "$MODID" ] && MODID="ColorOSIconsPatch"

PERSIST_BASE="/data/adb"
PERSIST_DIR="$PERSIST_BASE/$MODID"

BIN_DIR="$MODPATH/bin"
RUNTIME_DIR="$PERSIST_DIR/runtime"

UXICONS_DST="uxicons"

mkdir -p "$MODPATH/$UXICONS_DST"

set_perm_recursive $MODPATH 0 0 0755 0644
set_perm_recursive $BIN_DIR 0 0 0755 0755

ui_print "- ColorOS Icons Patch"
ui_print "- Using persist dir: $PERSIST_DIR"

mkdir -p "$RUNTIME_DIR" || abort "Create persist dir failed"

CIP_BIN="$BIN_DIR/cip"
[ -x "$CIP_BIN" ] || abort "cip binary not found"

ui_print "- Running cip init..."
"$CIP_BIN" init --config "$PERSIST_DIR/config.toml" \
  --target-dir "/data/adb/modules/$MODID/$UXICONS_DST" \
  --temp-dir "$RUNTIME_DIR" || ui_print "- cip init skipped (config exists or error)"

# ui_print "- Checking for updates via cip..."
# "$CIP_BIN" check --config "$PERSIST_DIR/config.toml" ||
#   ui_print "- cip check failed"

ui_print "- Done"
