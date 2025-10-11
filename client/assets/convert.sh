#!/usr/bin/env nix-shell
#!nix-shell -i bash -p imagemagick

function convert() {
  before=$1
  shift
  magick convert \
    $before \
    -background none \
    -resize '56x56^' -gravity center -crop 56x56+0+0 \
    -bordercolor none \
    -border 8x8 \
    ./nix-snowflake.svg "$@"
}

convert "" ./logo-color.png
convert "-colorspace Gray" ./logo-gray.png
convert "-colorspace Gray -gamma 3.0" ./logo-white.png
