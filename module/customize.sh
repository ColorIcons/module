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

OLD_UXICONS_DST="/data/adb/modules/$MODID/uxicons"
UXICONS_DST="$MODPATH/uxicons"

if [ -d "$OLD_UXICONS_DST" ]; then
  ui_print "- Found icons in module folder, migrating..."

  if mv "$OLD_UXICONS_DST" "$UXICONS_DST"; then
    ui_print "- Migration completed"
  else
    ui_print "- Move failed, attempting compatibility copy..."
    mkdir -p "$UXICONS_DST"
    if cp -af "$OLD_UXICONS_DST/." "$UXICONS_DST/"; then
      ui_print "- Copy completed"
    else
      ui_print "- Critical: Migration failed"
    fi
  fi
else
  ui_print "- No icons found in module folder"
  mkdir -p "$UXICONS_DST"
fi

CIP_BIN="$MODPATH/cip"

set_perm_recursive "$MODPATH" 0 0 0755 0644
set_perm "$CIP_BIN" 0 0 0755

ui_print "- ColorOS Icons Patch"
ui_print "- Using persist dir: $PERSIST_DIR"

mkdir -p "$PERSIST_DIR" || abort "Create persist dir failed"

[ -x "$CIP_BIN" ] || abort "cip binary not found"

ui_print "- Running cip init..."
"$CIP_BIN" config init || ui_print "- cip init skipped (config exists or error)"
