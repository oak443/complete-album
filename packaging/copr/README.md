# COPR project metadata

Project: <https://copr.fedorainfracloud.org/coprs/oak443/complete-album/>

The page is modelled on the `oak443/clash-verge-rev` project:

- Description
- Installation instructions, including the Atomic Fedora path
- Homepage
- Fedora 43/44/45 on x86_64 and aarch64
- Pulp storage and Fedora branching follow enabled
- One SCM package with webhook rebuild enabled

`enable_net` stays `false` here because the RPM build uses the vendor tarball
created by `.copr/Makefile`; the build does not need network access. This is
the one deliberate difference from `clash-verge-rev`.

## Description

```text
A command-line tool that adds MusicBrainz/Picard tags and Cover Art Archive artwork to ripped FLAC albums.
```

## Instructions

````markdown
Enable this COPR repository:

```bash
sudo dnf copr enable oak443/complete-album
```

Install the latest build:

```bash
sudo dnf install complete-album
```

On Atomic Fedora variants, enable the repository and install with `rpm-ostree`:

```bash
# Get current Fedora version
FEDORA_VERSION=$(rpm -E %fedora)

# Download the COPR repo file into /etc/yum.repos.d/
sudo wget https://copr.fedorainfracloud.org/coprs/oak443/complete-album/repo/fedora-"$FEDORA_VERSION"/oak443-complete-album-fedora-"$FEDORA_VERSION".repo -O /etc/yum.repos.d/_copr:oak443:complete-album.repo

# Install the latest build
rpm-ostree install complete-album
```

Usage:

```bash
complete-album --dry-run /path/to/album
complete-album --user-agent 'complete-album/0.1 (you@example.com)' /path/to/album
```

See the project page for the full guide:

https://github.com/oak443/complete-album
```
````
