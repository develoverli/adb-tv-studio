//! Live screen and instant input through scrcpy-server.
//!
//! Per TV: push the bundled server, open an adb forward tunnel, start the server over `adb shell`,
//! then connect two sockets: video (H.264 packets, relayed to the webview over a Tauri channel and
//! decoded there with `WebCodecs`) and control (key, text and touch messages).

use super::remote::RemoteKey;
use super::scrcpy_proto::{self, CODEC_H264, HEADER_LEN, Header};
use super::{AdbError, AdbState, run, sidecar};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

/// Must equal the bundled server's version (see `scripts/fetch-sidecars.mjs`); the server refuses others.
const SCRCPY_VERSION: &str = "5.0.1";
/// Where the server is pushed on the TV. Our own name, so it never clashes with a scrcpy install.
const REMOTE_JAR: &str = "/data/local/tmp/adb-tv-studio-scrcpy.jar";
/// The server needs a moment to start; retry the first connection for up to 5 s.
const CONNECT_ATTEMPTS: u32 = 50;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(100);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// Refuse absurd packet sizes instead of allocating them (a 4K key frame is far below this).
const MAX_PACKET_BYTES: u32 = 32 * 1024 * 1024;

/// Message kinds sent to the webview; the first byte of every channel message.
const MESSAGE_SESSION: u8 = 0;
const MESSAGE_PACKET: u8 = 1;
const MESSAGE_END: u8 = 2;
const PACKET_FLAG_CONFIG: u8 = 1;
const PACKET_FLAG_KEY: u8 = 2;

/// Video quality presets offered in the Control tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Quality {
    /// 720p, 30 fps, 4 Mbps: comfortable over Wi-Fi.
    Smooth,
    /// 1080p, 30 fps, 8 Mbps.
    Sharp,
    /// Native resolution, 60 fps, 12 Mbps: for wired networks.
    Max,
}

impl Quality {
    /// `(max_size, video_bit_rate, max_fps)`; `max_size = 0` keeps the native resolution.
    const fn params(self) -> (u32, u32, u32) {
        match self {
            Self::Smooth => (1280, 4_000_000, 30),
            Self::Sharp => (1920, 8_000_000, 30),
            Self::Max => (0, 12_000_000, 60),
        }
    }
}

/// Frame size of the stream, returned when it starts.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StreamInfo {
    pub width: u32,
    pub height: u32,
}

struct Session {
    video: TcpStream,
    control: TcpStream,
    child: CommandChild,
    port: u16,
    /// Current frame size, updated on rotation; touch messages must carry it.
    width: Arc<AtomicU32>,
    height: Arc<AtomicU32>,
}

/// Running scrcpy sessions, keyed by serial.
#[derive(Default)]
pub struct ScrcpyState {
    sessions: Mutex<HashMap<String, Session>>,
}

impl ScrcpyState {
    fn sessions(&self) -> MutexGuard<'_, HashMap<String, Session>> {
        // A poisoned lock means a panic mid-update; the sockets themselves are still usable.
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn io_error(error: &std::io::Error) -> AdbError {
    AdbError::Command(error.to_string())
}

/// A random 31-bit id that keeps two sessions from sharing a socket name on the TV.
fn new_scid() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.subsec_nanos());
    (nanos ^ std::process::id().rotate_left(16)) & 0x7fff_ffff
}

/// A free local TCP port for the adb forward tunnel.
fn free_port() -> Result<u16, AdbError> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| io_error(&error))?;
    let port = listener
        .local_addr()
        .map_err(|error| io_error(&error))?
        .port();
    Ok(port)
}

/// Connects the video socket once the server listens, reading the dummy byte that proves it.
/// Through an adb forward the TCP connect succeeds even before the server is up; the read fails then.
fn connect_video(port: u16) -> Result<TcpStream, AdbError> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut last_error = String::from("no attempt");
    for _ in 0..CONNECT_ATTEMPTS {
        match TcpStream::connect(address) {
            Ok(mut stream) => {
                stream
                    .set_read_timeout(Some(HANDSHAKE_TIMEOUT))
                    .map_err(|error| io_error(&error))?;
                let mut dummy = [0; 1];
                match stream.read_exact(&mut dummy) {
                    Ok(()) => return Ok(stream),
                    Err(error) => last_error = error.to_string(),
                }
            }
            Err(error) => last_error = error.to_string(),
        }
        std::thread::sleep(CONNECT_RETRY_DELAY);
    }
    Err(AdbError::Command(format!(
        "scrcpy server did not answer: {last_error}"
    )))
}

/// Opens both sockets and reads the stream header. Blocking.
fn handshake(port: u16) -> Result<(TcpStream, TcpStream, StreamInfo), AdbError> {
    let mut video = connect_video(port)?;
    let control = TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], port)))
        .map_err(|error| io_error(&error))?;
    control
        .set_nodelay(true)
        .map_err(|error| io_error(&error))?;

    let mut codec = [0; 4];
    video
        .read_exact(&mut codec)
        .map_err(|error| io_error(&error))?;
    if u32::from_be_bytes(codec) != CODEC_H264 {
        return Err(AdbError::Command(
            "the TV did not start an H.264 stream".to_owned(),
        ));
    }
    let mut header = [0; HEADER_LEN];
    video
        .read_exact(&mut header)
        .map_err(|error| io_error(&error))?;
    let Header::Session { width, height } = scrcpy_proto::parse_header(&header) else {
        return Err(AdbError::Command(
            "unexpected first video packet".to_owned(),
        ));
    };
    video
        .set_read_timeout(None)
        .map_err(|error| io_error(&error))?;
    Ok((video, control, StreamInfo { width, height }))
}

/// Reads packets until the stream ends and relays them to the webview. Blocking; runs on its own thread.
///
/// Channel messages: `[0][width u32][height u32]` for a new session, `[1][flags][pts u64][payload]`
/// for a packet (flags: 1 = codec config, 2 = key frame), and `[2]` when the stream ends.
fn relay(
    mut video: TcpStream,
    frames: &Channel<InvokeResponseBody>,
    width: &AtomicU32,
    height: &AtomicU32,
) {
    let mut header = [0; HEADER_LEN];
    while video.read_exact(&mut header).is_ok() {
        let message = match scrcpy_proto::parse_header(&header) {
            Header::Session {
                width: new_width,
                height: new_height,
            } => {
                width.store(new_width, Ordering::Relaxed);
                height.store(new_height, Ordering::Relaxed);
                let mut message = vec![MESSAGE_SESSION];
                message.extend_from_slice(&new_width.to_be_bytes());
                message.extend_from_slice(&new_height.to_be_bytes());
                message
            }
            Header::Media {
                config,
                key_frame,
                pts,
                size,
            } => {
                if size > MAX_PACKET_BYTES {
                    break;
                }
                let mut flags = 0;
                if config {
                    flags |= PACKET_FLAG_CONFIG;
                }
                if key_frame {
                    flags |= PACKET_FLAG_KEY;
                }
                let payload_len = usize::try_from(size).unwrap_or_default();
                let mut message = Vec::with_capacity(10 + payload_len);
                message.push(MESSAGE_PACKET);
                message.push(flags);
                message.extend_from_slice(&pts.to_be_bytes());
                message.resize(10 + payload_len, 0);
                if video.read_exact(&mut message[10..]).is_err() {
                    break;
                }
                message
            }
        };
        // The webview closed the channel (tab left, window closed): stop reading.
        if frames.send(InvokeResponseBody::Raw(message)).is_err() {
            return;
        }
    }
    if let Err(error) = frames.send(InvokeResponseBody::Raw(vec![MESSAGE_END])) {
        eprintln!("could not signal the end of the scrcpy stream: {error}");
    }
}

/// Closes a session: sockets, server process and tunnel.
async fn close(app: &AppHandle, serial: &str, session: Session) {
    let _ = session.video.shutdown(Shutdown::Both);
    let _ = session.control.shutdown(Shutdown::Both);
    if let Err(error) = session.child.kill() {
        eprintln!("could not stop scrcpy-server on {serial}: {error}");
    }
    let tunnel = format!("tcp:{}", session.port);
    if let Err(error) = run(app, &["-s", serial, "forward", "--remove", &tunnel]).await {
        eprintln!("could not remove the scrcpy tunnel: {error}");
    }
}

/// Starts (or restarts) live video and fast input for a TV. Frames arrive on `frames`.
#[tauri::command]
pub async fn scrcpy_start(
    app: AppHandle,
    adb: State<'_, AdbState>,
    scrcpy: State<'_, ScrcpyState>,
    serial: String,
    quality: Quality,
    frames: Channel<InvokeResponseBody>,
) -> Result<StreamInfo, AdbError> {
    adb.require_device(&serial)?;
    let previous = scrcpy.sessions().remove(&serial);
    if let Some(previous) = previous {
        close(&app, &serial, previous).await;
    }

    let jar = app
        .path()
        .resolve("scrcpy-server.jar", BaseDirectory::Resource)
        .map_err(|error| AdbError::Spawn(error.to_string()))?;
    let jar_text = jar.to_string_lossy().into_owned();
    let pushed = run(&app, &["-s", &serial, "push", &jar_text, REMOTE_JAR]).await?;
    if !pushed.success {
        return Err(AdbError::Command(pushed.text().trim().to_owned()));
    }

    let port = free_port()?;
    let scid = format!("{:08x}", new_scid());
    let tunnel = format!("tcp:{port}");
    let socket = format!("localabstract:scrcpy_{scid}");
    let forwarded = run(&app, &["-s", &serial, "forward", &tunnel, &socket]).await?;
    if !forwarded.success {
        return Err(AdbError::Command(forwarded.text().trim().to_owned()));
    }

    let (max_size, bit_rate, max_fps) = quality.params();
    let arguments = [
        format!("CLASSPATH={REMOTE_JAR}"),
        "app_process".to_owned(),
        "/".to_owned(),
        "com.genymobile.scrcpy.Server".to_owned(),
        SCRCPY_VERSION.to_owned(),
        format!("scid={scid}"),
        "log_level=warn".to_owned(),
        "tunnel_forward=true".to_owned(),
        "audio=false".to_owned(),
        "video_codec=h264".to_owned(),
        format!("max_size={max_size}"),
        format!("video_bit_rate={bit_rate}"),
        format!("max_fps={max_fps}"),
        "send_device_meta=false".to_owned(),
        "clipboard_autosync=false".to_owned(),
    ];
    let mut command_args = vec!["-s".to_owned(), serial.clone(), "shell".to_owned()];
    command_args.extend(arguments);
    let (mut events, child) = sidecar(&app)?
        .args(command_args)
        .spawn()
        .map_err(|error| AdbError::Spawn(error.to_string()))?;
    // Drain the server's output so its pipe never fills; keep stderr lines for diagnostics.
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.recv().await {
            if let CommandEvent::Stderr(line) = event {
                eprintln!("scrcpy-server: {}", String::from_utf8_lossy(&line).trim());
            }
        }
    });

    let handshake = tauri::async_runtime::spawn_blocking(move || handshake(port))
        .await
        .map_err(|error| AdbError::Command(error.to_string()));
    let (video, control, info) = match handshake {
        Ok(Ok(result)) => result,
        Ok(Err(error)) | Err(error) => {
            let _ = child.kill();
            let _ = run(&app, &["-s", &serial, "forward", "--remove", &tunnel]).await;
            return Err(error);
        }
    };

    let width = Arc::new(AtomicU32::new(info.width));
    let height = Arc::new(AtomicU32::new(info.height));
    let reader = video.try_clone().map_err(|error| io_error(&error))?;
    let (relay_width, relay_height) = (Arc::clone(&width), Arc::clone(&height));
    std::thread::spawn(move || relay(reader, &frames, &relay_width, &relay_height));

    scrcpy.sessions().insert(
        serial,
        Session {
            video,
            control,
            child,
            port,
            width,
            height,
        },
    );
    Ok(info)
}

/// Stops a TV's session (leaving the Control tab, changing quality).
#[tauri::command]
pub async fn scrcpy_stop(
    app: AppHandle,
    scrcpy: State<'_, ScrcpyState>,
    serial: String,
) -> Result<(), AdbError> {
    let session = scrcpy.sessions().remove(&serial);
    if let Some(session) = session {
        close(&app, &serial, session).await;
    }
    Ok(())
}

/// Writes control bytes to a TV's session.
fn send_control(scrcpy: &ScrcpyState, serial: &str, bytes: &[u8]) -> Result<(), AdbError> {
    let sessions = scrcpy.sessions();
    let session = sessions
        .get(serial)
        .ok_or_else(|| AdbError::Command("no live session for this TV".to_owned()))?;
    (&session.control)
        .write_all(bytes)
        .map_err(|error| io_error(&error))
}

/// Presses a key through the live session.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments and State by value"
)]
#[tauri::command]
pub fn scrcpy_key(
    scrcpy: State<'_, ScrcpyState>,
    serial: String,
    key: RemoteKey,
) -> Result<(), AdbError> {
    send_control(
        &scrcpy,
        &serial,
        &scrcpy_proto::key_press(u32::from(key.keycode())),
    )
}

/// Types text (any Unicode, up to 300 UTF-8 bytes) into the focused field.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments and State by value"
)]
#[tauri::command]
pub fn scrcpy_text(
    scrcpy: State<'_, ScrcpyState>,
    serial: String,
    text: String,
) -> Result<(), AdbError> {
    let message = scrcpy_proto::inject_text(&text).ok_or(AdbError::InvalidText)?;
    send_control(&scrcpy, &serial, &message)
}

/// Taps at video-frame coordinates.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments and State by value"
)]
#[tauri::command]
pub fn scrcpy_tap(
    scrcpy: State<'_, ScrcpyState>,
    serial: String,
    x: u32,
    y: u32,
) -> Result<(), AdbError> {
    let (width, height) = {
        let sessions = scrcpy.sessions();
        let session = sessions
            .get(&serial)
            .ok_or_else(|| AdbError::Command("no live session for this TV".to_owned()))?;
        (
            session.width.load(Ordering::Relaxed),
            session.height.load(Ordering::Relaxed),
        )
    };
    let (Ok(frame_width), Ok(frame_height)) = (u16::try_from(width), u16::try_from(height)) else {
        return Err(AdbError::Command("frame size out of range".to_owned()));
    };
    if x >= width || y >= height {
        return Err(AdbError::Command(format!(
            "tap outside the screen: {x},{y}"
        )));
    }
    send_control(
        &scrcpy,
        &serial,
        &scrcpy_proto::tap(x, y, frame_width, frame_height),
    )
}
