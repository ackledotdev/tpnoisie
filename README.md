# tpnoisie

Make the ThinkPad TrackPoint make noises

## Functionality

The program reads the TrackPoint displacement data from the provided device file (e.g., `/dev/input/eventX`) and generates sound based on the movement intensity. `libinput` is used to read the input events. Compiling may require `libinput-devel` or equivalent.

## Installation

### From source

Build requirements: `libinput-devel` or equivalent installed.

```bash
cargo install --git https://github.com/ackledotdev/tpnoisie.git
```

### From the AUR

```bash
paru -S tpnoisie
# or
yay -S tpnoisie
```

### From GitHub Releases

Precompiled binaries are available on the Releases page.

## Usage

Runtime requirements: Some directory `/path/to/sounds/` containing 10 files named exactly `{0..9}.{EXT}` where `{EXT}` is either `wav` or `ogg`, representing increasing intensity levels.

```bash
tpnoisie # find TrackPoint, trackpad, or other pointer device location (e.g. /dev/input/eventX)
tpnoisie /dev/input/eventX /path/to/sounds/ {EXT} # e.g. wav or ogg
```

Alternatively, specify `auto` for the device path to have the program automatically find the first TrackPoint device.

```bash
tpnoisie auto /path/to/sounds/ {EXT}
```

### Systemd Service

A systemd user unit file is provided in [`tpnoisie.service`](./tpnoisie.service). It should be copied to `~/.config/systemd/user/tpnoisie.service`. You may need to adjust the `ExecStart` path, device path, and sound directory.

```bash
systemctl --user enable --now tpnoisie.service
```
