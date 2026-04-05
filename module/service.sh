#!/bin/sh
# ColorOSIconsPatch - post-fs-data.sh
MODPATH=${0%/*}

[ -L "$MODPATH/webroot/uxicons" ] && rm -f "$MODPATH/webroot/uxicons"

ln -s "$MODPATH/uxicons" "$MODPATH/webroot/uxicons"
