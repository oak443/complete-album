# COPR project metadata

Project: <https://copr.fedorainfracloud.org/coprs/oak443/complete-album/>

The project page is deliberately short: install and use only. The Atomic
Fedora path and build details live below, not in the first screen.

## What users see

Description:

```text
A command-line tool that adds MusicBrainz/Picard tags and Cover Art Archive artwork to ripped FLAC albums.
```

Instructions:

````markdown
Install:

```bash
sudo dnf copr enable oak443/complete-album
sudo dnf install complete-album
```

Use:

```bash
complete-album --dry-run /path/to/album
complete-album --user-agent 'complete-album/0.1 (you@example.com)' /path/to/album
```

More: https://github.com/oak443/complete-album
````

## Project settings

- Fedora 43/44/45 on x86_64 and aarch64
- Pulp storage
- `follow_fedora_branching = true`
- `auto_prune = true`
- `appstream = false`
- One SCM package, `complete-album`, with webhook rebuild enabled

`enable_net` is `false`: `.copr/Makefile` vendors the Rust crates into the SRPM,
so the RPM build does not need network access. This is the one deliberate
difference from `oak443/clash-verge-rev`.

## Advanced: Atomic Fedora

```bash
# Get current Fedora version
FEDORA_VERSION=$(rpm -E %fedora)

# Download the COPR repo file into /etc/yum.repos.d/
sudo wget https://copr.fedorainfracloud.org/coprs/oak443/complete-album/repo/fedora-"$FEDORA_VERSION"/oak443-complete-album-fedora-"$FEDORA_VERSION".repo -O /etc/yum.repos.d/_copr:oak443:complete-album.repo

# Install the latest build
rpm-ostree install complete-album
```

## Build source

The package uses this repository directly:

```text
clone_url: https://github.com/oak443/complete-album.git
spec: complete-album.spec
source_build_method: make_srpm
```

The release workflow builds GitHub binaries; COPR builds RPMs through a GitHub
`create` webhook, with no COPR token stored in GitHub.
