#!/bin/sh

cp ../../target/release/flymoon ./package/usr/local/bin/
cp ../../ui/images/brand.png ./package/usr/share/icons/hicolor/256x256/apps/flymoon.png
cp ../../ui/images/brand.png ./package/usr/share/icons/hicolor/symbolic/apps/flymoon.png
cp ../../ui/images/brand.png ./package/usr/share/icons/hicolor/scalable/apps/flymoon.png

chmod a+x ./package/usr/local/bin/flymoon

dpkg-deb --build package

rm -f ./package/usr/local/bin/flymoon
rm -f ./package/usr/share/icons/hicolor/256x256/apps/flymoon.png
rm -f ./package/usr/share/icons/hicolor/symbolic/apps/flymoon.png
rm -f ./package/usr/share/icons/hicolor/scalable/apps/flymoon.png

mv -f package.deb flymoon.deb

exit $?
