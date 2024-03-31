use std::{env,time::Instant};
use cmd_lib::*;
/*Notes
Compress input.mp4 to target size
ffmpeg -i input.mp4 -b:v 555KiB -bufsize 555KiB output.mp4

Get duration of input.mp4
ffprobe -v error -select_streams v:0 -show_entries stream=duration -of default=noprint_wrappers=1:nokey=1 input.mp4

How to run: 
cargo run -- C:\path\input.mp4 C:\path\output.mp4 targetSizeInMB

TODO: GUI,  Compression for Audio & Images, File type conversions

*/
fn main() {
    let args: Vec<String> = env::args().collect();

    //TODO: input validation (might make gui handle that idk yet)
    let input_path = &args[1];
    let output_path = &args[2];
    let target_size: f32 = args[3].parse().expect("Invalid target size");
    println!("input_path = {}", input_path);
    println!("output_path = {}", output_path);
    println!("target_size = {} MB", target_size);
    
    //Start is used to find total time elapsed at different parts of the program
    let start = Instant::now();

    //TODO: Error handling if one of these doesnt return
    let duration: f32 = run_fun!(ffprobe -v error -select_streams v:0 -show_entries stream=duration -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid duration");
    let video_bitrate: f32 = run_fun!(ffprobe -v error -select_streams v:0 -show_entries stream=bit_rate -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid video bitrate");
    let audio_bitrate: f32 = run_fun!(ffprobe -v error -select_streams a:0 -show_entries stream=bit_rate -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid audio bitrate");
  
    println!("Total Time Elapsed: {}ms", start.elapsed().as_millis());
    println!("duration = {} seconds", duration);

    //Calculate new bitrate in KB to reach target file size
    let new_bitrate = ((target_size * 1024 as f32) / duration) - (audio_bitrate / 8192 as f32);
    println!("old_bitrate = {} KB/s", video_bitrate / 8192 as f32);
    println!("new_bitrate = {} KB/s", new_bitrate);
    
    if new_bitrate > (video_bitrate / 8192 as f32){
        println!("The video's current filesize is too close to the target. Please try a smaller target.");
        return;
    }

    //This runs ffmpeg to compress the video with less bitrate
    //TODO: Currently "-y" force overwrites any file with the same name as output_path, add check that path is empty, add (1), (2) etc at the end otherwise.
    let compress_status = run_cmd!(ffmpeg -v error -i $input_path -b:v ${new_bitrate}KiB -bufsize ${new_bitrate}KiB -y $output_path);

    match compress_status {
        Err(e) => println!("Compression Error: {}", e),
        Ok(_) => println!("Compression Done!")
    }
 
    
}
