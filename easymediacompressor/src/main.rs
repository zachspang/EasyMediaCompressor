use std::{collections::HashMap, env, fs::File, io::{self, BufRead, BufReader, Error, LineWriter, Write}, path::Path, thread, time::Instant};
use cmd_lib::*;
use rfd::FileDialog;
use slint::{SharedString, Weak};
/*Notes
How to run: 
cd easymediacompressor
cargo run

TODO: implement overwrite and two pass encoding settings, rename layouts, ex: target_file_size_box := HorizontalLayout, maybe use arrays in slint for the settings
add more info to settings Compression for Audio & Images, File type conversions
*/

slint::include_modules!();
fn main() {
    let app = App::new().unwrap();

    //Opens system file dialog to select a file path
    app.global::<ButtonLogic>().on_choose_file_button_pressed({
        let weak = app.as_weak();
        move |value|{
            let app = weak.unwrap();
            if value == ChooseFileButtonType::Input{
                let files = FileDialog::new()
                .add_filter("video", &["mp4"])
                .pick_file();
                if files.is_some(){
                    app.set_input_path(SharedString::from(files.unwrap().as_os_str().to_str().unwrap()))
                }
            }
            else if value == ChooseFileButtonType::Output{
                let files = FileDialog::new()
                .add_filter("video", &["mp4"])
                .pick_folder();
                if files.is_some(){
                    app.set_output_path(SharedString::from(files.unwrap().as_os_str().to_str().unwrap()))
                }
            }
            //TODO: else if value == default_input/output for settings menu
        }
    });

    //Calls compress_video when the compress button is pressed. 
    //compress_video is called on a new thread so the gui still responds while video is compressing
    app.global::<ButtonLogic>().on_compress_button_pressed({
        let weak = app.as_weak();
        move ||{
            let app = weak.unwrap();
            let input_path = app.get_input_path().to_string();
            let output_path = app.get_output_path().to_string();
            let target_size = app.get_target_size() as f32;
            let size_unit = app.get_size_unit();
            
            //Stop widgets from working when video is compressing and clear previous compress result
            app.set_widgets_enabled(false);
            app.set_compress_status("".into());

            //We need to make a new weak pointer to app since the other one is captured by the outer closure, there might be a better way to deal with this
            let weak = app.as_weak();
            thread::spawn( move ||{
                let weak_copy = weak.clone();
                let compress_result = compress_video(input_path,output_path,target_size, size_unit.to_string());
                let string_result;

                match compress_result {
                    Err(e) =>string_result = format!("Compression Error: {e}"),
                    Ok(_) => {
                        string_result = "Compression Done!".to_string();
                    }
                }

                let _ = slint::invoke_from_event_loop(
                    move || {
                        weak_copy.unwrap().set_widgets_enabled(true);
                        weak_copy.unwrap().set_compress_status(string_result.into());
                    });
            });
        }
    });

    //Write settings to config file
    app.global::<ButtonLogic>().on_settings_apply({
        let weak = app.as_weak();
        move ||{
            let app = weak.unwrap();
            app.set_default_target_size(app.get_temp_default_target_size().parse().unwrap());
            app.set_default_size_unit(app.get_temp_default_size_unit());
            app.set_overwrite(app.get_temp_overwrite());
            app.set_output_name_style(app.get_temp_output_name_style());
            app.set_two_pass_encoding(app.get_temp_two_pass_encoding());

            //write to config file
            //TODO: Handle errors writing to config
            let file = File::create("..\\config.txt").unwrap();
            let mut lw = LineWriter::new(file);
            lw.write_all(format!("default_target_size={}\n", app.get_default_target_size()).as_bytes()).expect("Error writing to config");
            lw.write_all(format!("default_size_unit={}\n", app.get_default_size_unit()).as_bytes()).expect("Error writing to config");
            lw.write_all(format!("overwrite={}\n", app.get_overwrite()).as_bytes()).expect("Error writing to config");
            lw.write_all(format!("output_name_style={}\n", app.get_output_name_style()).as_bytes()).expect("Error writing to config");
            lw.write_all(format!("two_pass_encoding={}\n", app.get_two_pass_encoding()).as_bytes()).expect("Error writing to config");

            println!("Config applied");
        }
    });

    //Reset displayed values of settings to last apply
    app.global::<ButtonLogic>().on_settings_cancel({    
        let weak = app.as_weak();
        move ||{
            let app = weak.unwrap();
            app.set_temp_default_target_size(SharedString::from(app.get_default_target_size().to_string()));
            app.set_temp_default_size_unit(app.get_default_size_unit());
            app.set_temp_overwrite(app.get_overwrite());
            app.set_temp_output_name_style(app.get_output_name_style());
            app.set_temp_two_pass_encoding(app.get_two_pass_encoding());
        }
    });

    //initialize variables with config file
    match read_config(app.as_weak()){
        Err(e) => println!("Config error {}", e),
        Ok(_) => println!("Config successfully read")
    }

    app.run().unwrap();
    
}

fn compress_video(input_path: String, output_path: String, mut target_size: f32, size_unit: String) -> Result<(), std::io::Error> {
    /*
    TODO: Make way to compress without opening gui by dragging and dropping a file onto the exe
    let args: Vec<String> = env::args().collect();

    let input_path = &args[1];
    let output_path = &args[2];
    let target_size: f32 = args[3].parse().expect("Invalid target size");
    */
    println!("\nStarting compression");
    println!("input_path = {}", input_path);
    println!("output_path = {}", output_path);
    println!("target_size = {} MB", target_size);

    //input validation
    if !Path::new(&input_path).exists(){
        return Err(Error::new(io::ErrorKind::InvalidInput, "The input file doesn't exist"));
    }

    if !Path::new(&output_path).exists(){
        return Err(Error::new(io::ErrorKind::InvalidInput, "The output path doesn't exist"));
    }

    if target_size > 9999 as f32 || target_size < 1 as f32{
        return Err(Error::new(io::ErrorKind::InvalidInput, "Invalid target size"));
    }

    //Turn target size into kB
    match size_unit.as_str(){
        "MB" => target_size *= 1024 as f32,
        "GB" => target_size *= 1048576 as f32,
        _ =>  return Err(Error::new(io::ErrorKind::InvalidInput, "Invalid size unit"))
    }
    dbg!(size_unit);
    dbg!(target_size);
    //create output file name, if it already exists add (i)
    let mut output_file = format!("{}{}", &output_path, "\\output.mp4");
    let mut file_number = 1;
    while Path::new(&output_file).exists(){
        output_file = format!("{}\\output({}).mp4", &output_path, file_number);
        file_number += 1;
    }

    //timer is used to find total time elapsed
    let timer = Instant::now();

    //TODO: Error handling if one of these doesnt return
    let duration: f32 = run_fun!(ffprobe -v error -select_streams v:0 -show_entries stream=duration -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid duration");
    let video_bitrate: f32 = run_fun!(ffprobe -v error -select_streams v:0 -show_entries stream=bit_rate -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid video bitrate");
  
    println!("duration = {} seconds", duration);

    //Calculate new bitrate in kB/s with space for 16 kB/s for audio to reach target file size 
    let mut new_video_bitrate = (target_size / duration) - 16 as f32;

    println!("old_bitrate = {} kB/s", video_bitrate / 8192 as f32);
    println!("new_bitrate = {} kB/s", new_video_bitrate);
    
    //Make sure the new bitrate is lower than the old bitrate
    if new_video_bitrate > (video_bitrate / 8192 as f32){
        return Err(Error::new(io::ErrorKind::InvalidInput, "The video's current filesize is too close to the target. Please try a smaller target."));
    }

    //If the user inputs a really small size the bitrate can end up negative since we leave 16kB/s for audio.
    if new_video_bitrate < 1.0 {
        new_video_bitrate = 1.0;
    }
    
    //This runs ffmpeg to lower the videos bitrate
    //TODO: Currently "-y" force overwrites any file with the same name as output_file, add check that path is empty, add (1), (2) etc at the end otherwise.
    let compress_status = run_cmd!(ffmpeg -v error -i $input_path -b:v ${new_video_bitrate}KiB -bufsize ${new_video_bitrate}KiB $output_file);
    
    //Slower but better quality two pass encoding to compress video
    //TODO: Add option to enable this
    //TODO:Add check for operating system and change NUL to /dev/null for Unix based systems
    //let pass1 = run_cmd!(ffmpeg -y -i $input_path -c:v libx265 -b:v ${new_video_bitrate}KiB -x265-params pass=1 -f null NUL);
    //let pass2 = run_cmd!(ffmpeg -i $input_path -c:v libx265 -b:v ${new_video_bitrate}KiB -x265-params pass=2 -c:a aac -b:a 128k $output_file);
    
    println!("Total Time Elapsed: {}ms", timer.elapsed().as_millis());
    return compress_status;


    //returns for two pass encoding
    // if pass1.is_err(){
    //     return pass1;
    // }
    // else{
    //     return pass2;
    // }

}

fn read_config(weak: Weak<App>)-> Result<(), std::io::Error>{
    let app = weak.unwrap();
    let file = File::open("..\\config.txt")?;
    let buffer = BufReader::new(file);
    let mut config_map:HashMap<String, String> = HashMap::new();
    let mut split_vector:Vec<String>;

    //For each line in the config map a key to value
    for line in buffer.lines(){   
        split_vector = line?.split("=").map(String::from).collect();
        config_map.insert(split_vector.get(0).unwrap_or(&"".to_string()).trim().to_owned(), split_vector.get(1).unwrap_or(&"".to_string()).trim().to_owned());
    }
    
    //if a key is in the map and the value is valid change the setting in the app
    if config_map.contains_key("default_target_size"){
        let value = config_map.get("default_target_size").unwrap().parse::<i32>().unwrap();
        if value < 9999 || value > 1{
            app.set_default_target_size(value);
        }
    }

    if config_map.contains_key("default_size_unit"){
        let value = config_map.get("default_size_unit").unwrap();
        if value == "MB" || value == "GB"{
            app.set_default_size_unit(SharedString::from(value));
        }
    }

    if config_map.contains_key("overwrite"){
        let value = config_map.get("overwrite").unwrap();
        if value == "true"{
            app.set_overwrite(true);
        }
        else {
            app.set_overwrite(false);
        }
    }
    
    if config_map.contains_key("output_name_style"){
        let value = config_map.get("output_name_style").unwrap();
        if value == "_Compressed" || value == "timestamp"{
            app.set_output_name_style(SharedString::from(value));
        }
    }

    if config_map.contains_key("two_pass_encoding"){
        let value = config_map.get("two_pass_encoding").unwrap();
        if value == "true"{
            app.set_two_pass_encoding(true);
        }
        else {
            app.set_two_pass_encoding(false);
        }
    }

    Ok(())
}