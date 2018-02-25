#[macro_use] extern crate nickel;

use nickel::{Nickel, HttpRouter};
use std::process::Command;

fn start_capture(src:&str, dst:&str)
{
    let cmd = "ffmpeg";
    let args = ["-y",                       // overwrite
                "-rtsp_transport", "tcp",
                "-i", src,
                "-c:v", "copy",
                "-f", "mpegts",
                dst];

    let process = match Command::new(cmd)
                                .args(&args)
                                .spawn() {
                                    Ok(process) => process,
                                    Err(err)    => panic!("Error starting capture process: {}", err),
                                };
}

fn main() {
    let mut server = Nickel::new();

    server.utilize(router! {
        get "**" => |_req, _res| {
            "Hello world!"
        }
    });

    start_capture("rtsp://192.168.0.10:554/user=admin_password=tlJwpbo6_channel=1_stream=0.sdp",
                  "out.ts");
    
    server.listen("192.168.0.8:8888");
}
