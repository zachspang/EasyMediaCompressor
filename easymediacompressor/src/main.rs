use std::{env, io::{self, Error}, path::Path, time::Instant, thread};
use cmd_lib::*;
use rfd::FileDialog;
use slint::SharedString;
/*Notes
Compress input.mp4 to target size
ffmpeg -i input.mp4 -b:v 555KiB -bufsize 555KiB output.mp4

Get duration of input.mp4
ffprobe -v error -select_streams v:0 -show_entries stream=duration -of default=noprint_wrappers=1:nokey=1 input.mp4

How to run: 
cargo run -- C:\path\input.mp4 C:\path\output.mp4 targetSizeInMB

TODO: Setting menu with config file, Move gui into a .slint file,  Compression for Audio & Images, File type conversions

*/

slint::slint!{
    //TODO: remove unused imports
    import {Button, Spinner, StandardButton, VerticalBox, ComboBox, GroupBox, Switch, SpinBox, CheckBox, LineEdit, HorizontalBox,TextEdit} from "std-widgets.slint";
    
    //Enum for types of Choose File buttons throughout the app
    enum ChooseFileButtonType{
        Input,
        Output,
        DefaultInput,
        DefaultOutput,
    }
    export global ChooseFileLogic{
        callback button-pressed(ChooseFileButtonType);
    }
    component ChooseFile {
        in property <ChooseFileButtonType> type;
        in-out property <bool> enabled <=> ta.enabled;
        Rectangle {
            background: ta.pressed ? #555: #c0bbbb;
            animate background { duration: 100ms;}
            height: 25px;
            width: 89px;
            border-width: 2px;
            border-radius: 10px;
            border-color: self.background.darker(20%);
            ta := TouchArea{
                clicked => {ChooseFileLogic.button-pressed(root.type);}
            }
            states [
                active when enabled: {
                    background:#c0bbbb;
                }
                inactive when !enabled: {
                    background: #555;
                }
            ]
        }
        Text{ text: "Choose File";}
    }

    export global CompressButtonLogic {
        callback button-pressed();
    }
    component CompressButton {
        in-out property <bool> enabled <=> ta.enabled;
        Rectangle {
            background: ta.pressed ? #0a470d: #12df2d;
            animate background { duration: 100ms;}
            height: 25px;
            width: 89px;
            border-width: 2px;
            border-radius: 10px;
            border-color: self.background.darker(20%);
            ta := TouchArea{
                clicked => {
                    CompressButtonLogic.button-pressed();
                }
            }
            states [
            active when enabled: {
                    background:#12df2d;
                }
                inactive when !enabled: {
                    background: #0a470d;
                }
        ]
        }
        Text{ text: "Compress";}
        
    }

    export component App inherits Window {
        in property <string> compress_status;
        in property <bool> widgets-enabled: true;
        in property <string> input_path;
        in property <string> output_path;
        in-out property <int> target_size;
        width: 420px;
        height: 330px;
        background: #272626;

        VerticalBox {

            //Target Size
            HorizontalBox {
                padding-bottom: 0px;
                Text {
                    color: white;
                    text:"Target File Size: ";
                    font-size: 15px;
                    height: 17px;
                }
            }
            HorizontalBox {
                padding-top: 0px;
                padding: 7px;
                spacing: 20px;
                LineEdit {
                    enabled: widgets-enabled;
                    width: 50px;
                    height: 30px;
                    text: target_size;
                    input-type: number;
                    horizontal-alignment: left;
                    //Keep target size less than 1000, anything else is proably unintentional or should be a different unit
                    edited => {
                        if self.text.to-float() > 1000{
                            self.text = 1000;
                        }
                        target_size = self.text.to-float();
                    }
                    
                }
                //TODO: Make this functional
                ComboBox {
                    enabled: widgets-enabled;
                    width:70px;
                    height: 30px;
                    model: ["MB", "KB", "GB"];
                } 
                Text {
                    color: white;
                    text:" Overwrite output.mp4:";
                    font-size: 15px;
                    height: 17px;
                    width: 155px;
                }
                //TODO: add bool overwrite as an argument to compress_video, if true pass -y ~~~\\output.mp4 into ffmpeg
                Switch {
                    enabled: widgets-enabled;
                    height: 10px;
                }
            }

            //Input
            HorizontalBox {
                padding-bottom: 0px;
                Text {
                    color: white;
                    text:"Input File: ";
                    font-size: 15px;
                    height: 17px;
                }   
            }
            HorizontalBox {
                padding-top: 0px;
                alignment: start;
                padding: 7px;
                spacing: 30px;
                LineEdit {
                    enabled: widgets-enabled;
                    font-size: 14px;
                    horizontal-alignment: left;
                    width: 280px;
                    height: 30px;
                    read-only: true;
                    placeholder-text: input_path;
                }
                ChooseFile {
                    type: ChooseFileButtonType.Input;
                    enabled: widgets-enabled;
                }
            }

            //Output
            HorizontalBox {
                padding-bottom: 0px;
                Text {
                    color: white;
                    text:"Output File Path: ";
                    font-size: 15px;
                    height: 17px;
                }    
            }
            HorizontalBox {
                padding-top: 0px;
                alignment: start;
                padding: 7px;
                spacing: 30px;
                LineEdit {
                    enabled: widgets-enabled;
                    font-size: 14px;
                    horizontal-alignment: left;
                    width: 280px;
                    height: 30px;
                    read-only: true;
                    placeholder-text: output_path;
                }
                ChooseFile {
                    type: ChooseFileButtonType.Output;
                    enabled: widgets-enabled;
                }
            }      
            
            //Compress
            HorizontalBox {
                padding-top: 15px;
                spacing: 15px;
                //compress_status text
                //TODO: find way to display longer messages
                Text {
                    color: white;
                    text: compress_status;
                    font-size: 15px;
                    height: 50px;
                    width: 200px;
                    wrap: word-wrap;
                    horizontal-alignment: left;
                }
                Spinner {
                    indeterminate: true;
                    visible: !widgets-enabled;
                }
                CompressButton{
                    enabled: widgets-enabled;
                }
            }
        }
    }
}

fn main() {
    let app = App::new().unwrap();

    //Opens system file dialog to select a file path
    app.global::<ChooseFileLogic>().on_button_pressed({
        let weak = app.as_weak();
        move |value|{
            let app = weak.unwrap();
            if value == ChooseFileButtonType::Input{
                let files = FileDialog::new()
                .add_filter("video", &["mp4"])
                //TODO: start from default set in settings
                .set_directory("C:\\Users")
                .pick_file();
                if files.is_some(){
                    app.set_input_path(SharedString::from(files.unwrap().as_os_str().to_str().unwrap()))
                }
            }
            else if value == ChooseFileButtonType::Output{
                let files = FileDialog::new()
                .add_filter("video", &["mp4"])
                //TODO: start from default set in settings
                .set_directory("C:\\Users")
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
    app.global::<CompressButtonLogic>().on_button_pressed({
        let weak = app.as_weak();
        move ||{
            let app = weak.unwrap();
            let input_path = app.get_input_path().to_string();
            let output_path = app.get_output_path().to_string();
            let target_size = app.get_target_size() as f32;
            
            //Stop widgets from working when video is compressing and clear previous compress result
            app.set_widgets_enabled(false);
            app.set_compress_status("".into());

            //We need to make a new weak pointer to app since the other one is captured by the outer closure, there might be a better way to deal with this
            let weak = app.as_weak();
            thread::spawn( move ||{
                let weak_copy = weak.clone();
                let compress_result = compress_video(input_path,output_path,target_size);
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
    app.run().unwrap();
    
}

fn compress_video(input_path: String, output_path: String, target_size: f32) -> Result<(), std::io::Error> {
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

    //create output file name, if it already exists add (i)
    let mut output_file = format!("{}{}", &output_path, "\\output.mp4");
    let mut file_number = 1;
    while Path::new(&output_file).exists(){
        output_file = format!("{}\\output({}).mp4", &output_path, file_number);
        file_number += 1;
    }

    if target_size > 1000 as f32 || target_size < 1 as f32{
        return Err(Error::new(io::ErrorKind::InvalidInput, "Invalid target size"));
    }
    
    //timer is used to find total time elapsed
    let timer = Instant::now();

    //TODO: Error handling if one of these doesnt return
    let duration: f32 = run_fun!(ffprobe -v error -select_streams v:0 -show_entries stream=duration -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid duration");
    let video_bitrate: f32 = run_fun!(ffprobe -v error -select_streams v:0 -show_entries stream=bit_rate -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid video bitrate");
    let audio_bitrate: f32 = run_fun!(ffprobe -v error -select_streams a:0 -show_entries stream=bit_rate -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid audio bitrate");
  
    println!("duration = {} seconds", duration);

    //Calculate new bitrate in KB to reach target file size
    let new_bitrate = ((target_size * 1024 as f32) / duration) - (audio_bitrate / 8192 as f32);
    println!("old_bitrate = {} KB/s", video_bitrate / 8192 as f32);
    println!("new_bitrate = {} KB/s", new_bitrate);
    
    //Make sure the new bitrate is lower than the old bitrate
    if new_bitrate > (video_bitrate / 8192 as f32){
        return Err(Error::new(io::ErrorKind::InvalidInput, "The video's current filesize is too close to the target. Please try a smaller target."));
    }

    //This runs ffmpeg to lower the videos bitrate//TODO: Currently "-y" force overwrites any file with the same name as output_file, add check that path is empty, add (1), (2) etc at the end otherwise.
    let compress_status = run_cmd!(ffmpeg -v error -i $input_path -b:v ${new_bitrate}KiB -bufsize ${new_bitrate}KiB $output_file);

    println!("Total Time Elapsed: {}ms", timer.elapsed().as_millis());
    return compress_status;

}
