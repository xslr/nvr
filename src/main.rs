#[macro_use] extern crate nickel;
#[macro_use] extern crate lazy_static;
extern crate regex;

use std::ffi::OsStr;
use std::io::Read;
use std::net::SocketAddr;
use std::process::{Command, Child, Stdio};
use std::str;
use std::sync::mpsc;
use std::char;
use std::thread;

use nickel::Nickel;
use regex::Regex;

enum FfmpegStatusField {
    BitRate,
    Duration,
    Fps,
    Frame,
    Size,
    Speed,
}

struct CaptureStatus {
    fps: u8,
    size: u32,
    time: u32,
    frames: u32,
    bitrate: u32,
}

lazy_static! {
    static ref FFMPEG_STATUS_REGEX: Regex = Regex::new(r"(\w+)= *([\w\.-/:]+)").unwrap();
}


fn create_process_or_panic<I, S>(cmd: &str, args: I) -> Child
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let child = match Command::new(cmd).args(args)
                                       .stdin(Stdio::piped())
                                       .stdout(Stdio::piped())
                                       .stderr(Stdio::piped())
                                       .spawn() {
                                           Ok(process) => process,
                                           Err(err)    => panic!("Error starting capture process: {}", err),
                                       };

    return child;
}

fn start_capture(src: &str, dst: &str) -> Child {
    let cmd = "ffmpeg";
    let args = ["-y",                       // overwrite existing file
                "-rtsp_transport", "tcp",
                "-i", src,
                "-c:v", "copy",
                "-f", "mpegts",
                dst];

    return create_process_or_panic(cmd, args.iter());
}

fn field_kind(s: &str) -> Option<FfmpegStatusField> {
    if s == "frame" { Some(FfmpegStatusField::Frame) }
    else if s == "fps" { Some(FfmpegStatusField::Fps) }
    else if s == "size" { Some(FfmpegStatusField::Size) }
    else if s == "time" { Some(FfmpegStatusField::Duration) }
    else if s == "bitrate" { Some(FfmpegStatusField::BitRate) }
    else if s == "speed" { Some(FfmpegStatusField::Speed) }
    else { None }
}

fn parse_bitrate(s: &str) -> u32 {
    let mut mult: u32 = 1;
    let mut br: u32 = 0;
    for c in s.chars() {
        if 'b' == c { mult = 1; break; }
        else if 'k' == c { mult = 1000; break; }
        else if 'm' == c { mult = 1000000; break; }
        else if c.is_digit(10) { br = br*10 + c.to_digit(10).unwrap(); }
    }

    br
}

fn parse_ffmpeg_statusline(line: &str) -> Option<CaptureStatus> {
    let mut status: CaptureStatus = CaptureStatus {
        frames: 0,
        size: 0,
        fps: 0,
        time: 0,
        bitrate: 0,
    };
    let res: Option<CaptureStatus> = None;

    for cap in FFMPEG_STATUS_REGEX.captures_iter(line) {
        /*
        println!("{}={}",
                 match field_kind(&cap[1]) {
                     Some(FfmpegStatusField::Frame) => "FRAME",
                     _ => "NONE"
                 }, &cap[2]);
         */

        match field_kind(&cap[1]) {
            Some(FfmpegStatusField::BitRate) => { status.bitrate = parse_bitrate(&cap[2]); }
            Some(FfmpegStatusField::Frame) => { status.frames = cap[2].parse().unwrap(); }
            //Some(FfmpegStatusField::Size) => { status.size = &cap[2].parse().unwrap(); }
            //Some(FfmpegStatusField::Duration) => { status.bitrate = parse_bitrate(&cap[2]); }
            _ => {}
        };
    }

    println!("br={} frame={}", status.bitrate, status.frames);

    res
}

fn start_capture_thread(src: &str, dst: &str) -> thread::JoinHandle<()> {
    let (tx, rx) = mpsc::channel::<CaptureStatus>();
    let _src = Box::new(String::from(src));
    let _dst = Box::new(String::from(dst));

    let t = thread::spawn(move || {
        let child: Child = start_capture(_src.as_ref(), _dst.as_ref());
        let err = child.stderr.unwrap();
        let input = child.stdin.unwrap();
        let mut buf: String = String::new();

        for byte in err.bytes() {
            let c: char = char::from(byte.unwrap());
            if !c.is_control() {
                buf.push(c);
            }
            else if c == '\r' && !buf.is_empty() {
                // now that we have read a complete line, we can parse it
                parse_ffmpeg_statusline(buf.trim());
                buf.clear();
            }
            else if c == '\n' && !buf.is_empty() {
                // we are not interested in lines ending with newline
                // as they are not statuslines
                buf.clear();
            }
        }
    });

    t
}

fn main() {
    let src = "rtsp://192.168.0.10:554/user=admin_password=tlJwpbo6_channel=1_stream=0.sdp";
    let dst = "out.ts";
    let mut server = Nickel::new();
    let mut children: Vec<thread::JoinHandle<()>> = Vec::new();

    server.utilize(router! {
        get "**" => |_req, _res| {
            "Hello world!"
        }
    });

    children.push(start_capture_thread(src, dst));


    let addrs = vec![SocketAddr::from(([127, 0, 0, 1], 8888)),
                     SocketAddr::from(([192, 168, 0, 14], 8888)),
                     SocketAddr::from(([192, 168, 0, 8], 8888)),];
    let listener = server.listen(&addrs[..]).expect("Could not start server");
    println!("Listening on: {:?}", listener.socket());

    // kill all capture processes at exit
    while let Some(handle) = children.pop() {
        let thread: &thread::Thread = handle.thread();
        println!("tid:{:?}", thread.id());
    }
}
