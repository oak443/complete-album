Name:           complete-album
Version:        0.1.1
Release:        1%{?dist}
Summary:        Write Picard-compatible MusicBrainz tags and Cover Art Archive artwork into FLAC albums
License:        GPL-3.0-only
URL:            https://github.com/oak443/complete-album
Source0:        complete-album-%{version}.tar.gz
Source1:        vendor.tar.xz

BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc
BuildRequires:  make
BuildRequires:  perl

%global debug_package %{nil}

%description
complete-album fills a ripped FLAC album folder with the tags MusicBrainz
Picard expects, including MusicBrainz entity IDs, artist sort names, release
metadata, and ISRCs. It also embeds the Cover Art Archive front cover and saves
the back cover and booklet pages beside the audio.

%prep
%autosetup -n %{name}-%{version}
tar -xJf %{SOURCE1}
mkdir -p .cargo
cat > .cargo/config.toml <<'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

%build
cargo build --release --offline --locked

%check
target/release/complete-album --version

%install
install -Dm755 target/release/complete-album %{buildroot}%{_bindir}/complete-album

%files
%{_bindir}/complete-album
%doc README.md CHANGELOG.md
%license LICENSE

%changelog
* Tue Nov 17 2026 oak443 <tyg8614@gmail.com> - 0.1.0-1
- Initial package
