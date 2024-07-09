//#![windows_subsystem = "windows"]
use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{ self, BufRead, BufReader, Error, LineWriter, Write },
    os::windows::process::CommandExt,
    path::Path,
    process::{ Command, ExitCode },
    str::from_utf8,
    thread,
};
use rfd::FileDialog;
use chrono::prelude::*;
use slint::{ SharedString, Weak };

/*Notes
TODO: use a different formatter for main, add fuctions for repetative parts of the compress functions, 
add tests and setup github actions, Seperate default sizes for each type, 
File type conversions, make changes to run on unix based systems, add gif compression
*/
const CREATE_NO_WINDOW: u32 = 0x08000000;

slint::include_modules!();
fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    let mut default_target_size = 25;
    let mut default_size_unit = SharedString::from("MB");
    let mut overwrite = true;
    let mut output_name_style = SharedString::from("_Compressed");
    let mut two_pass_encoding = false;

    //initialize variables with config file
    match read_config() {
        Ok(result) => {
            println!("Config successfully read");
            let config_map = result;

            //if a key is in the map and the value is valid set the variable
            if config_map.contains_key("default_target_size") {
                let value = config_map.get("default_target_size").unwrap().parse::<i32>().unwrap();
                if value < 9999 || value > 1 {
                    default_target_size = value;
                }
            }

            if config_map.contains_key("default_size_unit") {
                let value = config_map.get("default_size_unit").unwrap();
                if value == "MB" || value == "GB" {
                    default_size_unit = SharedString::from(value);
                }
            }

            if config_map.contains_key("overwrite") {
                let value = config_map.get("overwrite").unwrap();
                if value == "true" {
                    overwrite = true;
                } else {
                    overwrite = false;
                }
            }

            if config_map.contains_key("output_name_style") {
                let value = config_map.get("output_name_style").unwrap();
                if value == "_Compressed" || value == "timestamp" {
                    output_name_style = SharedString::from(value);
                }
            }

            if config_map.contains_key("two_pass_encoding") {
                let value = config_map.get("two_pass_encoding").unwrap();
                if value == "true" {
                    two_pass_encoding = true;
                } else {
                    two_pass_encoding = false;
                }
            }
        }
        Err(e) => println!("Config error {}", e),
    }

    //if a file is dragged onto the executable we want to not run the ui and just compress the file
    if args.get(1).is_some() && Path::new(&args[1]).exists() {
        //arrays of supported file formats
        let video_formats = ["mp4"];
        let audio_formats = ["mp3"];
        let image_formats = ["jpg, png"];

        let input_path = (&args[1]).to_owned();
        let file_extension = &input_path[input_path.rfind(".").unwrap() + 1..input_path.len()];
        let output_path = input_path[0..input_path.rfind("\\").unwrap()].to_string();

        //TODO: add some type of popup dialog when there is an error here
        if video_formats.contains(&file_extension) {
            compress_video(
                input_path,
                output_path,
                default_target_size as f32,
                default_size_unit.to_string(),
                overwrite,
                output_name_style.to_string(),
                two_pass_encoding
            ).expect("Compression Error: ");
        } else if audio_formats.contains(&file_extension) {
            compress_audio(
                input_path,
                output_path,
                default_target_size as f32,
                default_size_unit.to_string(),
                overwrite,
                output_name_style.to_string()
            ).expect("Compression Error: ");
        } else if image_formats.contains(&file_extension) {
            compress_image(
                input_path,
                output_path,
                overwrite,
                output_name_style.to_string()
            ).expect("Compression Error: ");
        }
        return ExitCode::SUCCESS;
    }

    let app = App::new().unwrap();

    //set variables in app
    app.set_default_target_size(default_target_size);
    app.set_default_size_unit(SharedString::from(default_size_unit));
    app.set_overwrite(overwrite);
    app.set_output_name_style(SharedString::from(output_name_style));
    app.set_two_pass_encoding(two_pass_encoding);

    //Opens system file dialog to select a file path
    app.global::<ButtonLogic>().on_choose_file_button_pressed({
        let weak = app.as_weak();
        move |value| {
            let app = weak.unwrap();
            let is_input: bool;
            let name: &str;
            let mut extensions: Vec<&str> = Vec::new();

            match value {
                ChooseFileButtonType::VideoIn => {
                    is_input = true;
                    name = "video";
                    extensions = ["mp4"].to_vec();
                }
                ChooseFileButtonType::VideoOut => {
                    is_input = false;
                    name = "video";
                }
                ChooseFileButtonType::AudioIn => {
                    is_input = true;
                    name = "audio";
                    extensions = ["mp3"].to_vec();
                }
                ChooseFileButtonType::AudioOut => {
                    is_input = false;
                    name = "audio";
                }
                ChooseFileButtonType::ImageIn => {
                    is_input = true;
                    name = "image";
                    extensions = ["png", "jpg"].to_vec();
                }
                ChooseFileButtonType::ImageOut => {
                    is_input = false;
                    name = "image";
                }
            }

            if is_input {
                let files = FileDialog::new().add_filter(name, &extensions).pick_file();
                if files.is_some() {
                    app.set_input_path(
                        SharedString::from(files.unwrap().as_os_str().to_str().unwrap())
                    )
                }
            } else {
                let files = FileDialog::new().add_filter("video", &["mp4"]).pick_folder();
                if files.is_some() {
                    app.set_output_path(
                        SharedString::from(files.unwrap().as_os_str().to_str().unwrap())
                    )
                }
            }
        }
    });

    //Calls the corresponding compress function when the compress button is pressed.
    //compress_... functions are called on a new thread so the gui still responds while video is compressing
    app.global::<ButtonLogic>().on_compress_button_pressed({
        let weak = app.as_weak();
        move |value| {
            let app = weak.unwrap();
            let input_path = app.get_input_path().to_string();
            let output_path = app.get_output_path().to_string();
            let target_size = app.get_target_size() as f32;
            let size_unit = app.get_size_unit().to_string();
            let overwrite = app.get_overwrite();
            let output_name_style = app.get_output_name_style().to_string();
            let two_pass_encoding = app.get_two_pass_encoding();

            //Stop widgets from working when video is compressing and clear previous compress result
            app.set_widgets_enabled(false);
            app.set_compress_status("".into());

            //We need to make a new weak pointer to app since the other one is captured by the outer closure, there might be a better way to deal with this
            let weak = app.as_weak();
            thread::spawn(move || {
                let weak_copy = weak.clone();
                let compress_result: Result<(), Error>;
                dbg!(&value);
                if value == CompressButtonType::Video {
                    compress_result = compress_video(
                        input_path,
                        output_path,
                        target_size,
                        size_unit,
                        overwrite,
                        output_name_style,
                        two_pass_encoding
                    );
                } else if value == CompressButtonType::Audio {
                    compress_result = compress_audio(
                        input_path,
                        output_path,
                        target_size,
                        size_unit,
                        overwrite,
                        output_name_style
                    );
                } else {
                    compress_result = compress_image(
                        input_path,
                        output_path,
                        overwrite,
                        output_name_style
                    );
                }
                let string_result;

                match compress_result {
                    Err(e) => {
                        string_result = format!("Compression Error: {e}");
                    }
                    Ok(_) => {
                        string_result = "Compression Done!".to_string();
                    }
                }

                let _ = slint::invoke_from_event_loop(move || {
                    weak_copy.unwrap().set_widgets_enabled(true);
                    weak_copy.unwrap().set_compress_status(string_result.into());
                });
            });
        }
    });

    //Write settings to config file
    app.global::<ButtonLogic>().on_settings_apply({
        let weak = app.as_weak();
        move || {
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
            lw.write_all(
                format!("default_target_size={}\n", app.get_default_target_size()).as_bytes()
            ).expect("Error writing to config");
            lw.write_all(
                format!("default_size_unit={}\n", app.get_default_size_unit()).as_bytes()
            ).expect("Error writing to config");
            lw.write_all(format!("overwrite={}\n", app.get_overwrite()).as_bytes()).expect(
                "Error writing to config"
            );
            lw.write_all(
                format!("output_name_style={}\n", app.get_output_name_style()).as_bytes()
            ).expect("Error writing to config");
            lw.write_all(
                format!("two_pass_encoding={}\n", app.get_two_pass_encoding()).as_bytes()
            ).expect("Error writing to config");

            println!("Config applied");
        }
    });

    //Reset displayed values of settings to last apply
    app.global::<ButtonLogic>().on_settings_cancel({
        let weak = app.as_weak();
        move || {
            let app = weak.unwrap();
            app.set_temp_default_target_size(
                SharedString::from(app.get_default_target_size().to_string())
            );
            app.set_temp_default_size_unit(app.get_default_size_unit());
            app.set_temp_overwrite(app.get_overwrite());
            app.set_temp_output_name_style(app.get_output_name_style());
            app.set_temp_two_pass_encoding(app.get_two_pass_encoding());
        }
    });

    app.run().unwrap();
    return ExitCode::SUCCESS;
}

fn compress_video(
    input_path: String,
    output_path: String,
    mut target_size: f32,
    size_unit: String,
    overwrite: bool,
    output_name_style: String,
    two_pass_encoding: bool
) -> Result<(), Error> {
    /*
    TODO: Make way to compress without opening gui by dragging and dropping a file onto the exe
    let args: Vec<String> = env::args().collect();

    let input_path = &args[1];
    let output_path = &args[2];
    let target_size: f32 = args[3].parse().expect("Invalid target size");
    */

    //timer is used to track total time elapsed during compression and for the timestamp name style
    let timer: DateTime<Local> = Local::now();
    println!("\nStarting compression");
    println!("input_path = {}", input_path);
    println!("output_path = {}", output_path);
    println!("target_size = {} {}", target_size, size_unit);

    //input validation
    if !Path::new(&input_path).exists() {
        return Err(Error::new(io::ErrorKind::InvalidInput, "The input file doesn't exist"));
    }

    if !Path::new(&output_path).exists() {
        return Err(Error::new(io::ErrorKind::InvalidInput, "The output path doesn't exist"));
    }

    if target_size > (9999 as f32) || target_size < (1 as f32) {
        return Err(Error::new(io::ErrorKind::InvalidInput, "Invalid target size"));
    }

    //Turn target size into kB
    match size_unit.as_str() {
        "MB" => {
            target_size *= 1024 as f32;
        }
        "GB" => {
            target_size *= 1048576 as f32;
        }
        _ => {
            return Err(Error::new(io::ErrorKind::InvalidInput, "Invalid size unit"));
        }
    }

    //create output file name
    let mut output_file;
    if output_name_style == "_Compressed" {
        //add the name of the input file to the end of the output path
        output_file = format!(
            "{}{}",
            &output_path,
            &input_path[input_path.rfind("\\").unwrap()..input_path.len()]
        );
        //add _Compressed before the file extension
        output_file.insert_str(output_file.rfind(".").unwrap(), "_Compressed");
    } else if output_name_style == "timestamp" {
        //add the name of the input file to the end of the output path
        output_file = format!(
            "{}{}",
            &output_path,
            &input_path[input_path.rfind("\\").unwrap()..input_path.len()]
        );
        //get timestamp
        let mut timestamp = timer.to_rfc3339_opts(SecondsFormat::Secs, false).to_owned();

        //remove utc offset from timestmap
        timestamp.truncate(timestamp.len() - 6);

        timestamp = timestamp.replace(':', ".");

        //replace from start of file name to file extension with timestamp
        output_file.replace_range(
            output_file.rfind("\\").unwrap() + 1..output_file.rfind(".").unwrap(),
            &timestamp
        );
    } else {
        output_file = format!("{}{}", &output_path, "\\output.mp4");
    }

    let mut file_number = 1;
    //if file exist and overwrite is false, add (file_number) to the file name, file_number is incremented until a file with the name doesnt exist
    while Path::new(&output_file.as_str()).exists() && !overwrite {
        //add the parentheses on the first loop
        if file_number == 1 {
            output_file.insert_str(output_file.rfind(".").unwrap(), "( )");
        }

        output_file.replace_range(
            output_file.rfind(")").unwrap() - 1..output_file.rfind(")").unwrap(),
            &file_number.to_string()
        );
        file_number += 1;
    }

    //TODO: Error handling if one of these doesnt return
    let duration: f32 = from_utf8(
        Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                &input_path,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .expect("Invalid duration")
            .stdout.as_ref()
    )
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let video_bitrate: f32 = from_utf8(
        Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=bit_rate",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                &input_path,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .expect("Invalid video bitrate")
            .stdout.as_ref()
    )
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    println!("duration = {} seconds", duration);

    //Calculate new bitrate in kB/s with space for 16 kB/s for audio to reach target file size
    let mut new_video_bitrate = target_size / duration - (16 as f32);

    println!("old_bitrate = {} kB/s", video_bitrate / (8192 as f32));
    println!("new_bitrate = {} kB/s", new_video_bitrate);

    //Make sure the new bitrate is lower than the old bitrate
    if new_video_bitrate >= video_bitrate / (8192 as f32) {
        return Err(
            Error::new(
                io::ErrorKind::InvalidInput,
                "The video's current filesize is too close to the target. Please try a smaller target."
            )
        );
    }

    //If the user inputs a really small size the bitrate can end up negative since we leave 16kB/s for audio.
    if new_video_bitrate < 1.0 {
        new_video_bitrate = 1.0;
    }

    if two_pass_encoding {
        //Slower but better quality two pass encoding to compress video
        //TODO:Add check for operating system and change NUL to /dev/null for Unix based systems
        let pass1 = Command::new("ffmpeg")
            .args([
                "-v",
                "fatal",
                "-y",
                "-i",
                &input_path,
                "-c:v",
                "libx265",
                "-b:v",
                &format!("{new_video_bitrate}KiB"),
                "-x265-params",
                "log-level=0:pass=1",
                "-f",
                "null",
                "NUL",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let pass2 = Command::new("ffmpeg")
            .args([
                "-v",
                "fatal",
                "-i",
                &input_path,
                "-c:v",
                "libx265",
                "-b:v",
                &format!("{new_video_bitrate}KiB"),
                "-x265-params",
                "log-level=0:pass=2",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
                "-y",
                &output_file,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        println!("Total Time Elapsed: {}ms", timer.signed_duration_since(Local::now()));
        if output_contains_error(&pass1) {
            return get_output_error(pass1);
        } else if output_contains_error(&pass2) {
            return get_output_error(pass2);
        } else {
            return Ok(());
        }
    } else {
        //This runs ffmpeg to lower the videos bitrate
        let compress_status = Command::new("ffmpeg")
            .args([
                "-v",
                "fatal",
                "-i",
                &input_path,
                "-b:v",
                &format!("{new_video_bitrate}KiB"),
                "-bufsize",
                &format!("{new_video_bitrate}KiB"),
                "-y",
                &output_file,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        println!(
            "Total Time Elapsed: {}ms",
            Local::now().signed_duration_since(timer).num_milliseconds()
        );
        if output_contains_error(&compress_status) {
            return get_output_error(compress_status);
        } else {
            return Ok(());
        }
    }
}

fn compress_audio(
    input_path: String,
    output_path: String,
    mut target_size: f32,
    size_unit: String,
    overwrite: bool,
    output_name_style: String
) -> Result<(), Error> {
    //timer is used to track total time elapsed during compression and for the timestamp name style
    let timer: DateTime<Local> = Local::now();
    println!("\nStarting compression");
    println!("input_path = {}", input_path);
    println!("output_path = {}", output_path);
    println!("target_size = {} {}", target_size, size_unit);

    //input validation
    if !Path::new(&input_path).exists() {
        return Err(Error::new(io::ErrorKind::InvalidInput, "The input file doesn't exist"));
    }

    if !Path::new(&output_path).exists() {
        return Err(Error::new(io::ErrorKind::InvalidInput, "The output path doesn't exist"));
    }

    if target_size > (9999 as f32) || target_size < (1 as f32) {
        return Err(Error::new(io::ErrorKind::InvalidInput, "Invalid target size"));
    }

    //Turn target size into kB
    match size_unit.as_str() {
        "MB" => {
            target_size *= 1024 as f32;
        }
        "GB" => {
            target_size *= 1048576 as f32;
        }
        "KB" => {}
        _ => {
            return Err(Error::new(io::ErrorKind::InvalidInput, "Invalid size unit"));
        }
    }

    //create output file name
    let mut output_file;
    if output_name_style == "_Compressed" {
        //add the name of the input file to the end of the output path
        output_file = format!(
            "{}{}",
            &output_path,
            &input_path[input_path.rfind("\\").unwrap()..input_path.len()]
        );
        //add _Compressed before the file extension
        output_file.insert_str(output_file.rfind(".").unwrap(), "_Compressed");
    } else if output_name_style == "timestamp" {
        //add the name of the input file to the end of the output path
        output_file = format!(
            "{}{}",
            &output_path,
            &input_path[input_path.rfind("\\").unwrap()..input_path.len()]
        );
        //get timestamp
        let mut timestamp = timer.to_rfc3339_opts(SecondsFormat::Secs, false).to_owned();

        //remove utc offset from timestmap
        timestamp.truncate(timestamp.len() - 6);

        timestamp = timestamp.replace(':', ".");

        //replace from start of file name to file extension with timestamp
        output_file.replace_range(
            output_file.rfind("\\").unwrap() + 1..output_file.rfind(".").unwrap(),
            &timestamp
        );
    } else {
        output_file = format!("{}{}", &output_path, "\\output.mp3");
    }

    let mut file_number = 1;
    //if file exist and overwrite is false, add (file_number) to the file name, file_number is incremented until a file with the name doesnt exist
    while Path::new(&output_file.as_str()).exists() && !overwrite {
        //add the parentheses on the first loop
        if file_number == 1 {
            output_file.insert_str(output_file.rfind(".").unwrap(), "( )");
        }

        output_file.replace_range(
            output_file.rfind(")").unwrap() - 1..output_file.rfind(")").unwrap(),
            &file_number.to_string()
        );
        file_number += 1;
    }

    //TODO: Error handling if one of these doesnt return
    let duration: f32 = from_utf8(
        Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                &input_path,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .expect("Invalid duration")
            .stdout.as_ref()
    )
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let audio_bitrate: f32 = from_utf8(
        Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=bit_rate",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                &input_path,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .expect("Invalid audio bitrate")
            .stdout.as_ref()
    )
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    println!("duration = {} seconds", duration);

    //Calculate new bitrate in Kbit/s
    let mut new_audio_bitrate = (target_size * 8.192) / duration;
    //Using constant bitrate only the following bitrates are available for audio
    // prettier-ignore
    let possible_bitrates:Vec<f32> = vec![8.0, 16.0, 24.0, 32.0, 40.0, 48.0, 64.0, 80.0, 96.0, 112.0, 128.0, 160.0, 192.0, 224.0, 256.0, 320.0, 999999.0];

    for bitrate_index in 0..possible_bitrates.len() {
        if
            new_audio_bitrate > possible_bitrates[bitrate_index] &&
            new_audio_bitrate < possible_bitrates[bitrate_index + 1]
        {
            new_audio_bitrate = possible_bitrates[bitrate_index];
            break;
        }
    }

    println!("old_bitrate = {} Kbit/s", audio_bitrate / (1000 as f32));
    println!("new_bitrate = {} Kbit/s", new_audio_bitrate);

    //Make sure the new bitrate is lower than the old bitrate
    if new_audio_bitrate >= audio_bitrate / (1000 as f32) {
        return Err(
            Error::new(
                io::ErrorKind::InvalidInput,
                "The audio's current filesize is too close to the target. Please try a smaller target."
            )
        );
    }

    //This runs ffmpeg to lower the audio bitrate
    let compress_status = Command::new("ffmpeg")
        .args([
            "-v",
            "fatal",
            "-i",
            &input_path,
            "-b:a",
            &format!("{new_audio_bitrate}k"),
            "-y",
            &output_file,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    println!(
        "Total Time Elapsed: {}ms",
        Local::now().signed_duration_since(timer).num_milliseconds()
    );
    if output_contains_error(&compress_status) {
        return get_output_error(compress_status);
    } else {
        return Ok(());
    }
}

fn compress_image(
    input_path: String,
    output_path: String,
    overwrite: bool,
    output_name_style: String
) -> Result<(), Error> {
    //timer is used to track total time elapsed during compression and for the timestamp name style
    let timer: DateTime<Local> = Local::now();
    println!("\nStarting compression");
    println!("input_path = {}", input_path);
    println!("output_path = {}", output_path);

    //input validation
    if !Path::new(&input_path).exists() {
        return Err(Error::new(io::ErrorKind::InvalidInput, "The input file doesn't exist"));
    }

    if !Path::new(&output_path).exists() {
        return Err(Error::new(io::ErrorKind::InvalidInput, "The output path doesn't exist"));
    }

    //create output file name
    let mut output_file;
    if output_name_style == "_Compressed" {
        //add the name of the input file to the end of the output path
        output_file = format!(
            "{}{}",
            &output_path,
            &input_path[input_path.rfind("\\").unwrap()..input_path.len()]
        );
        //add _Compressed before the file extension
        output_file.insert_str(output_file.rfind(".").unwrap(), "_Compressed");
    } else if output_name_style == "timestamp" {
        //add the name of the input file to the end of the output path
        output_file = format!(
            "{}{}",
            &output_path,
            &input_path[input_path.rfind("\\").unwrap()..input_path.len()]
        );
        //get timestamp
        let mut timestamp = timer.to_rfc3339_opts(SecondsFormat::Secs, false).to_owned();

        //remove utc offset from timestmap
        timestamp.truncate(timestamp.len() - 6);

        timestamp = timestamp.replace(':', ".");

        //replace from start of file name to file extension with timestamp
        output_file.replace_range(
            output_file.rfind("\\").unwrap() + 1..output_file.rfind(".").unwrap(),
            &timestamp
        );
    } else {
        output_file = format!("{}{}", &output_path, "\\output.jpg");
    }

    let mut file_number = 1;
    //if file exist and overwrite is false, add (file_number) to the file name, file_number is incremented until a file with the name doesnt exist
    while Path::new(&output_file.as_str()).exists() && !overwrite {
        //add the parentheses on the first loop
        if file_number == 1 {
            output_file.insert_str(output_file.rfind(".").unwrap(), "( )");
        }

        output_file.replace_range(
            output_file.rfind(")").unwrap() - 1..output_file.rfind(")").unwrap(),
            &file_number.to_string()
        );
        file_number += 1;
    }

    //This runs ffmpeg to compress the image
    let compress_status = Command::new("ffmpeg")
        .args([
            "-v",
            "fatal",
            "-i",
            &input_path,
            "-vframes",
            "1",
            "-r",
            "1",
            "-compression_level",
            "9",
            "-flags",
            "-ildct",
            "-y",
            &output_file,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    println!(
        "Total Time Elapsed: {}ms",
        Local::now().signed_duration_since(timer).num_milliseconds()
    );
    if output_contains_error(&compress_status) {
        return get_output_error(compress_status);
    } else {
        return Ok(());
    }
}

fn read_config() -> Result<HashMap<String, String>, Error> {
    let file = File::open("..\\config.txt")?;
    let buffer = BufReader::new(file);
    let mut config_map: HashMap<String, String> = HashMap::new();
    let mut split_vector: Vec<String>;

    //For each line in the config map a key to value
    for line in buffer.lines() {
        split_vector = line?.split("=").map(String::from).collect();
        config_map.insert(
            split_vector.get(0).unwrap_or(&"".to_string()).trim().to_owned(),
            split_vector.get(1).unwrap_or(&"".to_string()).trim().to_owned()
        );
    }
    return Ok(config_map);
}

fn output_contains_error(result: &Result<std::process::Output, Error>) -> bool {
    let stderr = result.as_ref().unwrap().clone().stderr;
    if String::from_utf8(stderr).unwrap() == "" {
        return false;
    }
    return true;
}

fn get_output_error(result: Result<std::process::Output, Error>) -> Result<(), Error> {
    return Err(
        Error::new(io::ErrorKind::Other, String::from_utf8(result.unwrap().stderr).unwrap())
    );
}
