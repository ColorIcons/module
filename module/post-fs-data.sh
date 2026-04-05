#!/bin/sh
# ColorOSIconsPatch - post-fs-data.sh

MODPATH=${0%/*}

# 获取 Android 版本
android_ver=$(getprop ro.build.version.release | cut -d. -f1)

is_C16() {
  [ "$android_ver" -ge 16 ]
}

if is_C16; then
  mount --bind "$MODPATH/uxicons" "/my_product/media/theme/uxicons/hdpi"
  mount --bind "$MODPATH/uxicons" "/data/oplus/uxicons"
else
  mount --bind "$MODPATH/uxicons" "/my_stock/media/theme/uxicons/xhdpi"
  mount --bind "$MODPATH/uxicons" "/my_stock/media/theme/uxicons/xxhdpi"
  mount --bind "$MODPATH/uxicons" "/my_stock/media/theme/uxicons/xxxhdpi"
fi
