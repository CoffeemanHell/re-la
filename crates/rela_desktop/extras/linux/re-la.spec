# Packs an already built re-la into an rpm for the Fedora it was built on.
#   rpmbuild -bb crates/rela_desktop/extras/linux/re-la.spec --define "ver 0.0.16~alpha" --define "bin $PWD/target/release/re-la" \
#            --define "src $PWD/crates/rela_desktop" --define "_rpmdir $PWD/dist"
# Library requirements (Qt, libmpv) are found by rpm from the binary; QML modules (Qt Quick, Qt.labs.platform for
# the tray) are listed by hand.
%global debug_package %{nil}

Name:     re-la
Version:  %{ver}
Release:  1%{?dist}
Summary:  Navidrome music player that listens together
# the app's old name: dnf replaces an installed nsync with this
Obsoletes: nsync < %{version}-%{release}
License:  AGPL-3.0-only AND OFL-1.1
Requires: qt6-qtdeclarative

%description
A small native music player for Navidrome that lets people listen to the same song at the same time.
Uses the system Qt, so it follows the desktop's style and colors.

%install
install -Dm755 %{bin} %{buildroot}%{_bindir}/re-la
install -Dm644 %{src}/extras/linux/re-la.desktop %{buildroot}%{_datadir}/applications/re-la.desktop
install -Dm644 %{src}/extras/linux/re-la.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/re-la.svg
install -Dm644 -t %{buildroot}%{_licensedir}/re-la %{src}/../../LICENSE %{src}/themes/nothing/fonts/OFL-*.txt

%files
%license %{_licensedir}/re-la
%{_bindir}/re-la
%{_datadir}/applications/re-la.desktop
%{_datadir}/icons/hicolor/scalable/apps/re-la.svg
