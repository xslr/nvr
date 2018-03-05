#[macro_use] extern crate nickel;

use std::ffi::OsStr;
use std::net::SocketAddr;
use std::process::{Command, Child};
use nickel::Nickel;

fn create_process_or_panic<I, S>(cmd: &str, args: I) -> Child
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let child = match Command::new(cmd)
                                .args(args)
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

fn main() {
    let mut server = Nickel::new();
    let mut children: Vec<Child> = Vec::new();

    server.utilize(router! {
        get "**" => |_req, _res| {
            "Hello world!"
        }
    });

    children.push(start_capture("rtsp://192.168.0.10:554/user=admin_password=tlJwpbo6_channel=1_stream=0.sdp",
                                "out.ts"));

    let addrs = vec![SocketAddr::from(([127, 0, 0, 1], 8888)),
                     SocketAddr::from(([192, 168, 0, 14], 8888)),
                     SocketAddr::from(([192, 168, 0, 8], 8888)),];
    let listener = server.listen(&addrs[..]).expect("Could not start server");
    println!("Listening on: {:?}", listener.socket());

    // kill all capture processes at exit
    while let Some(mut child) = children.pop() {
        println!("Kill capture process: {}", child.id());
        child.kill();
        child.wait();
    }
}
