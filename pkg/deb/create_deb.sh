#!/bin/sh

icon_name="brand.png"
icon_dir="../../ui/images"
dst_icon_name="flymoon.png"
dst_icon_name_svg="flymoon.svg"

cp ../../target/release/flymoon ./package/usr/local/bin/

sizes=(16x16 22x22 24x24 32x32 36x36 48x48 64x64 72x72 96x96 128x128 192x192 256x256 512x512)

for size in "${sizes[@]}"; do
    mkdir -p ./package/usr/share/icons/hicolor/${size}/apps
    magick "${icon_dir}/${icon_name}" -resize "$size" -background none -gravity center -extent "$size" "./package/usr/share/icons/hicolor/${size}/apps/${dst_icon_name}"
done

# cp ../../ui/images/${icon_name} ./package/usr/share/icons/hicolor/symbolic/apps/${dst_icon_name_svg}
# cp ../../ui/images/${icon_name} ./package/usr/share/icons/hicolor/scalable/apps/${dst_icon_name_svg}

chmod a+x ./package/usr/local/bin/flymoon

dpkg-deb --build package

rm -f ./package/usr/local/bin/flymoon

for size in "${sizes[@]}"; do
    rm -f ./package/usr/share/icons/hicolor/${size}/apps/${dst_icon_name}
done

# rm -f ./package/usr/share/icons/hicolor/symbolic/apps/${dst_icon_name_svg}
# rm -f ./package/usr/share/icons/hicolor/scalable/apps/${dst_icon_name_svg}

mv -f package.deb flymoon.deb

exit $?
