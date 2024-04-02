use std::{env, path::PathBuf, time::Instant};
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

TODO: GUI,  Compression for Audio & Images, File type conversions

*/

slint::slint!{
    //TODO: remove unused imports
    import {Button, VerticalBox, ComboBox, GroupBox, Switch, SpinBox, CheckBox, LineEdit, HorizontalBox,TextEdit} from "std-widgets.slint";
    
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
        }
        Text{ text: "Choose File";}
    }

    export component App inherits Window {
        in property <string> input_path;
        in property <string> output_path;
        in property <int> target_size;
        width: 420px;
        height: 250px;
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
                LineEdit {
                    width:50px;
                    height: 30px;
                    text: "";
                    input-type: number;
                    horizontal-alignment: center;
                }
                ComboBox {
                    width:70px;
                    height: 30px;
                    model: ["MB", "KB"];
    
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
                    font-size: 14px;
                    horizontal-alignment: left;
                    width: 280px;
                    height: 30px;
                    read-only: true;
                    placeholder-text: input_path;
                }
                ChooseFile {
                    type: ChooseFileButtonType.Input;
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
                    font-size: 14px;
                    horizontal-alignment: left;
                    width: 280px;
                    height: 30px;
                    read-only: true;
                    placeholder-text: output_path;
                }
                ChooseFile {
                    type: ChooseFileButtonType.Output;
                }
            }
                    
}
    }
}

fn main() {
    let app = App::new().unwrap();
    let weak = app.as_weak();

    //Opens system file dialog to select input file or path
    app.global::<ChooseFileLogic>().on_button_pressed(move |value|{
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
       
    });
    app.run().unwrap();
    
}

/* Old main, will turn into fn compress_video(input_path: String, output_path: String, target_size: f32) -> Result
fn main() {
    let args: Vec<String> = env::args().collect();

    //TODO: input validation (might make gui handle that idk yet)
    let input_path = &args[1];
    let output_path = &args[2];
    let target_size: f32 = args[3].parse().expect("Invalid target size");
    println!("input_path = {}", input_path);
    println!("output_path = {}", output_path);
    println!("target_size = {} MB", target_size);
    
    //timer is used to find total time elapsed at different parts of the program
    let timer = Instant::now();

    //TODO: Error handling if one of these doesnt return
    let duration: f32 = run_fun!(ffprobe -v error -select_streams v:0 -show_entries stream=duration -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid duration");
    let video_bitrate: f32 = run_fun!(ffprobe -v error -select_streams v:0 -show_entries stream=bit_rate -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid video bitrate");
    let audio_bitrate: f32 = run_fun!(ffprobe -v error -select_streams a:0 -show_entries stream=bit_rate -of default=noprint_wrappers=1:nokey=1 $input_path).unwrap().trim().parse().expect("Invalid audio bitrate");
  
    println!("Total Time Elapsed: {}ms", timer.elapsed().as_millis());
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

    println!("Total Time Elapsed: {}ms", timer.elapsed().as_millis());

}
*/