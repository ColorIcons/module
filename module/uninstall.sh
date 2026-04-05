#!/bin/sh
# ColorOSIconsPatch - uninstall.sh

MODDIR="${0%/*}"

MODID="$(grep -m1 '^id=' "$MODDIR/module.prop" | cut -d= -f2)"
[ -z "$MODID" ] && MODID="ColorOSIconsPatch"

PERSIST_DIR="/data/adb/$MODID"

rm -rf "$PERSIST_DIR" 2>/dev/null

exit 0
