#[macro_use] extern crate nickel;

use std::ffi::OsStr;
use std::io::Read;
use std::net::SocketAddr;
use std::process::{Command, Child, Stdio};
use std::str;
use std::thread;
use nickel::Nickel;

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

fn start_capture_thread(src: &str, dst: &str) -> thread::JoinHandle<()>
{
    let _src = Box::new(String::from(src));
    let _dst = Box::new(String::from(dst));

    let t = thread::spawn(move || {
        let child: Child = start_capture(_src.as_ref(), _dst.as_ref());
        let err = child.stderr.unwrap();
        let mut buf: Vec<u8> = Vec::new();

        for byte in err.bytes() {
            let b = byte.unwrap();
            if b == 13 {
                println!("{}\n", str::from_utf8(buf.as_slice()).unwrap());
                buf.clear();
            }
            else if b == 10 {
            }
            else {
                buf.push(b);
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

    children.push(
        start_capture_thread(src, dst)
    );


    let addrs = vec![SocketAddr::from(([127, 0, 0, 1], 8888)),
                     SocketAddr::from(([192, 168, 0, 14], 8888)),
                     SocketAddr::from(([192, 168, 0, 8], 8888)),];
    let listener = server.listen(&addrs[..]).expect("Could not start server");
    println!("Listening on: {:?}", listener.socket());

    // kill all capture processes at exit
    while let Some(handle) = children.pop() {
        let mut thread: &thread::Thread = handle.thread();
        println!("tid:{:?}", thread.id());
    }
}
