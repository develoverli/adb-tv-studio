//! scrcpy 5.0.1 wire format: control messages we send and video packet headers we read.
//! Byte layouts follow the upstream tests (`app/tests/test_control_msg_serialize.c`) and
//! `doc/develop.md` at the pinned tag; all integers are big-endian.

/// Control message types (the `sc_control_msg_type` enum order).
const TYPE_INJECT_KEYCODE: u8 = 0;
const TYPE_INJECT_TEXT: u8 = 1;
const TYPE_INJECT_TOUCH_EVENT: u8 = 2;

/// `AKEY_EVENT_ACTION_DOWN` / `_UP` and `AMOTION_EVENT_ACTION_DOWN` / `_UP` share these values.
const ACTION_DOWN: u8 = 0;
const ACTION_UP: u8 = 1;

/// `SC_POINTER_ID_GENERIC_FINGER` (`UINT64_C(-2)`): a plain finger touch.
const POINTER_GENERIC_FINGER: u64 = u64::MAX - 1;

/// Longest text the server accepts in one message (`SC_CONTROL_MSG_INJECT_TEXT_MAX_LENGTH`), in bytes.
pub const MAX_TEXT_BYTES: usize = 300;

/// Codec id the server sends first on the video socket for H.264 (`"h264"` as ASCII).
pub const CODEC_H264: u32 = 0x6832_3634;

/// Size of a session packet and of a media packet header.
pub const HEADER_LEN: usize = 12;

fn keycode_message(action: u8, keycode: u32) -> [u8; 14] {
    let mut message = [0; 14];
    message[0] = TYPE_INJECT_KEYCODE;
    message[1] = action;
    message[2..6].copy_from_slice(&keycode.to_be_bytes());
    // Bytes 6..10 (repeat) and 10..14 (meta state) stay 0.
    message
}

/// Key press: a DOWN message followed by an UP message.
pub fn key_press(keycode: u32) -> Vec<u8> {
    let mut bytes = keycode_message(ACTION_DOWN, keycode).to_vec();
    bytes.extend_from_slice(&keycode_message(ACTION_UP, keycode));
    bytes
}

/// Text injection: type, u32 length, UTF-8 bytes. `None` when empty or longer than [`MAX_TEXT_BYTES`].
pub fn inject_text(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_TEXT_BYTES {
        return None;
    }
    let length = u32::try_from(bytes.len()).ok()?;
    let mut message = Vec::with_capacity(5 + bytes.len());
    message.push(TYPE_INJECT_TEXT);
    message.extend_from_slice(&length.to_be_bytes());
    message.extend_from_slice(bytes);
    Some(message)
}

fn touch_message(action: u8, x: u32, y: u32, width: u16, height: u16) -> [u8; 32] {
    let mut message = [0; 32];
    message[0] = TYPE_INJECT_TOUCH_EVENT;
    message[1] = action;
    message[2..10].copy_from_slice(&POINTER_GENERIC_FINGER.to_be_bytes());
    message[10..14].copy_from_slice(&x.to_be_bytes());
    message[14..18].copy_from_slice(&y.to_be_bytes());
    message[18..20].copy_from_slice(&width.to_be_bytes());
    message[20..22].copy_from_slice(&height.to_be_bytes());
    // Pressure as unsigned 16-bit fixed point: full press while down, none on release.
    let pressure: u16 = if action == ACTION_DOWN { u16::MAX } else { 0 };
    message[22..24].copy_from_slice(&pressure.to_be_bytes());
    // Bytes 24..28 (action button) and 28..32 (buttons) stay 0 for a finger.
    message
}

/// Tap at `(x, y)` in a `width`×`height` frame (the current video size): DOWN then UP.
pub fn tap(x: u32, y: u32, width: u16, height: u16) -> Vec<u8> {
    let mut bytes = touch_message(ACTION_DOWN, x, y, width, height).to_vec();
    bytes.extend_from_slice(&touch_message(ACTION_UP, x, y, width, height));
    bytes
}

/// A 12-byte header read from the video socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Header {
    /// New capture session (start or rotation) with the frame size.
    Session { width: u32, height: u32 },
    /// Media packet: `size` payload bytes follow.
    Media {
        config: bool,
        key_frame: bool,
        pts: u64,
        size: u32,
    },
}

const SESSION_FLAG: u8 = 0x80;
const CONFIG_FLAG: u64 = 1 << 62;
const KEY_FRAME_FLAG: u64 = 1 << 61;
const PTS_MASK: u64 = KEY_FRAME_FLAG - 1;

fn u32_at(bytes: &[u8; HEADER_LEN], start: usize) -> u32 {
    u32::from_be_bytes([
        bytes[start],
        bytes[start + 1],
        bytes[start + 2],
        bytes[start + 3],
    ])
}

/// Parses a session packet or a media packet header (they differ by the first bit).
pub fn parse_header(bytes: &[u8; HEADER_LEN]) -> Header {
    if bytes[0] & SESSION_FLAG != 0 {
        return Header::Session {
            width: u32_at(bytes, 4),
            height: u32_at(bytes, 8),
        };
    }
    let mut pts_bytes = [0; 8];
    pts_bytes.copy_from_slice(&bytes[..8]);
    let flags_and_pts = u64::from_be_bytes(pts_bytes);
    Header::Media {
        config: flags_and_pts & CONFIG_FLAG != 0,
        key_frame: flags_and_pts & KEY_FRAME_FLAG != 0,
        pts: flags_and_pts & PTS_MASK,
        size: u32_at(bytes, 8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_press_should_send_down_then_up() {
        // AKEYCODE_ENTER = 0x42, as in upstream test_serialize_inject_keycode.
        let bytes = key_press(0x42);
        assert_eq!(
            bytes,
            vec![
                0, 0, 0, 0, 0, 0x42, 0, 0, 0, 0, 0, 0, 0, 0, //
                0, 1, 0, 0, 0, 0x42, 0, 0, 0, 0, 0, 0, 0, 0,
            ]
        );
    }

    #[test]
    fn inject_text_should_match_upstream_layout() {
        // Same bytes as upstream test_serialize_inject_text.
        let mut expected = vec![1, 0, 0, 0, 0x0d];
        expected.extend_from_slice(b"hello, world!");
        assert_eq!(inject_text("hello, world!"), Some(expected));
    }

    #[test]
    fn inject_text_should_count_utf8_bytes() {
        assert_eq!(inject_text("canción").map(|bytes| bytes[4]), Some(8));
    }

    #[test]
    fn inject_text_should_reject_too_long() {
        assert_eq!(inject_text(&"a".repeat(MAX_TEXT_BYTES + 1)), None);
    }

    #[test]
    fn tap_should_encode_finger_down_like_upstream() {
        let bytes = tap(100, 200, 1080, 1920);
        assert_eq!(
            &bytes[..24],
            &[
                2, 0, // inject touch, ACTION_DOWN
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe, // generic finger
                0, 0, 0, 0x64, 0, 0, 0, 0xc8, // 100 200
                0x04, 0x38, 0x07, 0x80, // 1080 1920
                0xff, 0xff, // pressure
            ]
        );
    }

    #[test]
    fn tap_should_release_with_zero_pressure() {
        let bytes = tap(1, 1, 10, 10);
        assert_eq!((bytes[33], bytes[54], bytes[55]), (1, 0, 0));
    }

    #[test]
    fn parse_header_should_read_session_size() {
        let bytes = [0x80, 0, 0, 0, 0, 0, 0x07, 0x80, 0, 0, 0x04, 0x38];
        assert_eq!(
            parse_header(&bytes),
            Header::Session {
                width: 1920,
                height: 1080
            }
        );
    }

    #[test]
    fn parse_header_should_read_config_packet() {
        let bytes = [0x40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x20];
        assert_eq!(
            parse_header(&bytes),
            Header::Media {
                config: true,
                key_frame: false,
                pts: 0,
                size: 32
            }
        );
    }

    #[test]
    fn parse_header_should_read_key_frame_pts() {
        let bytes = [0x20, 0, 0, 0, 0, 0, 0x03, 0xe8, 0, 0, 0x10, 0];
        assert_eq!(
            parse_header(&bytes),
            Header::Media {
                config: false,
                key_frame: true,
                pts: 1000,
                size: 4096
            }
        );
    }
}
