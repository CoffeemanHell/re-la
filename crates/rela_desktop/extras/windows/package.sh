#!/bin/bash
# The Windows zip: dist/re-la-windows-x64.zip, a folder with re-la.exe, the Qt and MinGW DLLs it loads and libmpv-2.dll.
# Run from the MSYS2 UCRT64 shell at the repo root. Needs:
#   pacman -S zip mingw-w64-ucrt-x86_64-{toolchain,lld,rust,cmake,nasm,qt6-base,qt6-declarative,qt6-svg,qt6-imageformats,qt6-tools}
#   libmpv (https://github.com/shinchiro/mpv-winbuild-cmake/releases, mpv-dev-x86_64-*.7z) unpacked to $LIBMPV
set -euo pipefail
LIBMPV=${LIBMPV:-/c/libmpv}
QTBIN=$(qmake6 -query QT_INSTALL_BINS)
export QMAKE="$(cygpath -w /ucrt64/bin/qmake6.exe)"
# libmpv2-sys links "mpv" (libmpv.dll.a); lld because GNU ld reads archives once and cxx-qt's initializers come late
export RUSTFLAGS="-L native=$(cygpath -w "$LIBMPV") -C link-arg=-fuse-ld=lld"

# qt-build-utils runs Qt's build tools (moc, rcc, qmltyperegistrar...) with an empty environment, so Windows finds their
# DLLs only next to them; MSYS2 keeps the tools in share/qt6/ and the DLLs in /ucrt64/bin. Copy what they load there.
for dir in "$QTBIN" "$(qmake6 -query QT_HOST_LIBEXECS)"; do
    find "$dir" -maxdepth 1 -name '*.exe' | while read -r t; do ldd "$t"; done \
        | awk '$3 ~ /^\/ucrt64\// {print $3}' | sort -u | while read -r d; do cp -n "$d" "$dir/"; done
done

cargo build --release --locked -p rela_desktop

out=dist/re-la
rm -rf dist/re-la dist/re-la-windows-x64.zip
mkdir -p "$out"
cp target/release/re-la.exe "$LIBMPV/libmpv-2.dll" "$out/"
cp -r LICENSE LICENSES README.md crates/rela_desktop/themes/nothing/fonts/OFL-*.txt "$out/"
# the QML the themes import, and the style main.rs picks at run time (no QML file imports it); the SVG tray icon and
# WebP/JPEG covers are plugins nothing links
style=$(mktemp -d)
echo 'import QtQuick.Controls.FluentWinUI3' > "$style/style.qml"
"$QTBIN/windeployqt" --release --no-translations --no-system-d3d-compiler --no-opengl-sw \
    --qmldir crates/rela_desktop/themes --qmldir crates/rela_desktop/qml --qmldir "$style" \
    --include-plugins qsvgicon,qsvg,qjpeg,qwebp "$out/re-la.exe"
# windeployqt copies Qt's own DLLs; the MinGW runtime and the other MSYS2 libraries they load come from ldd
find "$out" -name '*.dll' -o -name '*.exe' | while read -r f; do ldd "$f"; done \
    | awk '$3 ~ /^\/ucrt64\// {print $3}' | sort -u | while read -r d; do cp -n "$d" "$out/"; done
# MSYS2's Qt looks for its plugins and QML under its own prefix (share/qt6/...): here they sit next to the exe
printf '[Paths]\nPrefix = .\nPlugins = .\nQmlImports = qml\n' > "$out/qt.conf"
(cd dist && zip -qr re-la-windows-x64.zip re-la)
ls -l dist/re-la-windows-x64.zip
