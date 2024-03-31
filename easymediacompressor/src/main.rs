use std::{env, process::Command, time::Instant};
/*Notes

0:45 Video, Target 25MB = 25000KB, Target / Time = bitrate = 555KB

Compress input.mp4 to target size
ffmpeg -i input.mp4 -b:v 555KiB -bufsize 555KiB output.mp4

Get duration of input.mp4
ffprobe -v error -select_streams v:0 -show_entries stream=duration -of default=noprint_wrappers=1:nokey=1 input.mp4

How to run: 
cargo run -- C:\path\input.mp4 C:\path\output.mp4 targetSizeInMB
TODO: try to use only Command:new to get duration, audio bitrate and video bitrate
TODO: GUI, Add check that calculated bitrate isnt higher than current bitrate, Compression for Audio & Images, File type conversions

*/
fn main() {
    let args: Vec<String> = env::args().collect();

    //TODO: input validation (might make gui handle that idk yet)
    let input_path = &args[1];
    let output_path = &args[2];
    let target_size: i32 = args[3].parse().expect("Invalid target size");
    println!("input_path = {}", input_path);
    println!("target_size = {}", target_size);
    
    //Start is used to find total time elapsed at different parts of the program
    let start = Instant::now();

    //This runs ffprobe to get the duration of the video
    let get_duration = Command::new("ffprobe")
            .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=duration", 
            "-of", "default=noprint_wrappers=1:nokey=1", &input_path])
            .output()
            .expect("Failed to get video duration");
    println!("get_duration status: {}", get_duration.status);

    println!("Total Time Elapsed: {}ms", start.elapsed().as_millis());

    //TODO: add error handling if something goes wrong with ffprobe
    let duration: f32 = String::from_utf8(get_duration.stdout).unwrap().trim().parse().expect("Invalid duration");
    println!("duration = {} seconds", duration);

    //Multiply target size by 950 instead of 1000 to allow a video size 5% below target to make space for audio
    //TODO: this doesnt work for longer videos, need to read audio bitrate to calculate what to make the video bitrate
    //ffprobe -v error -select_streams a:0 -show_entries stream=bit_rate -of default=noprint_wrappers=1:nokey=1 input.mp4
    let bitrate = (target_size * 950) / duration as i32;
    println!("bitrate = {} KB/s", bitrate);

    //This runs ffmpeg to compress the video with less bitrate
    //TODO: Currently "-y" force overwrites any file with the same name as output_path, add check that path is empty, add (1), (2) etc at the end otherwise.
    let compress = Command::new("ffmpeg")
        .args(["-i", &input_path, "-b:v", &format!("{}KiB",bitrate), "-bufsize", &format!("{}KiB",bitrate), "-y", &output_path])
        .output()
        .expect("Failed to compress file");
    println!("compress status: {}", compress.status);
    println!("Total Time Elapsed: {}ms", start.elapsed().as_millis());
    
}
