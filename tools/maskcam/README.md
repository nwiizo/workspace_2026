# Maskcam

Maskcam is a macOS-only, privacy-first camera preview written in Rust. It detects
exactly one face, covers it with a chosen image, and presents the result in a
dedicated window for OBS Studio to capture and publish as a virtual camera.

> [!IMPORTANT]
> Maskcam does not register a camera device in this MVP. Google Meet and Zoom
> display it as **OBS Virtual Camera**, not as Maskcam.

If no face or more than one face is detected, Maskcam hides the entire frame.
Camera frames stay on the Mac and are never uploaded.

## MVP architecture

```text
AVFoundation camera
        |
        v
Rust + OpenCV + YuNet ----> Maskcam Output window
                                   |
                                   v
                         OBS Window Capture
                                   |
                                   v
                         OBS Virtual Camera
                                   |
                                   v
                         Zoom / Google Meet
```

OBS is deliberately used for the virtual-camera boundary in the MVP. A native
Core Media I/O camera extension requires a signed host app, a system extension,
an App Group, installation under `/Applications`, and explicit administrator
approval. That is a separate packaging milestone rather than a face-processing
concern.

## Requirements

- macOS 13 or newer
- Rust 1.88 or newer
- OpenCV 5 with AVFoundation, DNN, HighGUI, and Video I/O
- OBS Studio 30 or newer

With Homebrew:

```sh
brew install opencv
brew install --cask obs
```

## Setup

```sh
cd tools/maskcam
./scripts/download-model.sh
cargo build --release
```

On first launch, macOS asks for camera access. The default overlay is the image
requested for this MVP:

```sh
cargo run --release -- --overlay ~/Downloads/icon.jpg
```

The default is already `~/Downloads/icon.jpg`, so this is equivalent:

```sh
cargo run --release
```

Validate the image, model checksum result, and OpenCV model compatibility without
opening the camera:

```sh
cargo run --release -- --check
```

Controls:

- `q` or `Esc`: quit
- `f`: toggle fullscreen

Useful options:

```sh
cargo run --release -- \
  --camera 0 \
  --overlay ~/Downloads/icon.jpg \
  --overlay-scale 1.9 \
  --confidence 0.85 \
  --detection-width 640 \
  --width 1280 \
  --height 720
```

Add `--mirror` only when a deliberately mirrored outgoing picture is wanted.
Zoom and Google Meet normally mirror their local preview themselves.

## OBS and meeting setup

1. Open OBS and add a **macOS Screen Capture** source.
2. Select **Window Capture** and choose `Maskcam Output`.
3. Set the OBS canvas and output resolution to `1280x720`.
4. Crop window chrome if OBS includes it.
5. Click **Start Virtual Camera**.
6. In Zoom or Google Meet, select **OBS Virtual Camera**.
7. Verify the remote-facing result with a second device before joining a real
   meeting.

On recent macOS versions, OBS may ask for approval of its camera/media system
extension. Complete the prompt in System Settings and restart OBS.

### `OBS Virtual Camera` is missing in Google Meet

1. In OBS, click **Start Virtual Camera** in the Controls panel.
2. Approve the OBS extension when macOS asks. Start with **System Settings →
   General → Login Items & Extensions → Camera Extensions**. If that section is
   absent, look under **Privacy & Security → Extensions → OBS → Media
   Extension** instead; Apple has moved this setting between macOS updates.
3. Quit OBS completely and open it again.
4. Quit Chrome completely with `Command-Q` and open it again. Reloading the Meet
   tab alone may retain the old camera list.
5. Open Meet settings and select **OBS Virtual Camera**.

The OBS preview must already show the Maskcam output before joining a meeting.
If it does not, verify that the OBS macOS Screen Capture source is set to Window
Capture and its selected window is `Maskcam Output`.

## Privacy behavior and limitations

- Output remains fully hidden until exactly one face passes the confidence
  threshold.
- Losing detection for even one frame hides the full output immediately.
- Detecting multiple faces also hides the full output so another person is not
  exposed accidentally.
- The overlay is enlarged beyond the detector box and follows eye-line rotation.
- Detection runs on a smaller frame and its coordinates are mapped back to the
  full-resolution output. Consecutive valid detections are smoothed to reduce
  jitter.
- The MVP has no temporal grace period. This favors privacy over a smooth picture
  and may briefly switch to the hidden frame during fast movement.
- Window capture and OBS add latency. A signed Core Media I/O extension is the
  next step for a standalone virtual camera.
- Face detection reduces risk but cannot guarantee anonymity. Always perform a
  private test call and keep the physical camera shutter available.

## Verification

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-targets
```
