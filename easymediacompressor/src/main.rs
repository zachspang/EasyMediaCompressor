use std::{collections::HashMap, env, fs::File, io::{self, BufRead, BufReader, Error, Write}, path::Path, thread, time::Instant};
use cmd_lib::*;
use rfd::FileDialog;
use slint::{SharedString, Weak};
/*Notes
How to run: 
cd easymediacompressor
cargo run

TODO: Setting menu with config file, Move gui into a .slint file,  Compression for Audio & Images, File type conversions
*/

slint::slint!{
    //TODO: remove unused imports
    import {Button, Spinner, StandardButton, VerticalBox, ComboBox, GroupBox, Switch, SpinBox, CheckBox, LineEdit, HorizontalBox,TextEdit, Slider} from "std-widgets.slint";
    
    //Enum for types of Choose File buttons throughout the app
    enum ChooseFileButtonType{
        Input,
        Output,
        DefaultInput,
        DefaultOutput,
    }

    export global ButtonLogic{
        callback choose-file-button-pressed(ChooseFileButtonType);
        callback compress-button-pressed();
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
                clicked => {ButtonLogic.choose-file-button-pressed(root.type);}
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
                    ButtonLogic.compress-button-pressed();
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

    component SidebarButton inherits Rectangle {
        in-out property <bool> active;

        height: 50px;
        width: 50px;

        callback activate;

        TouchArea {
            clicked => {root.activate()}
        }

    }


    
    export component App inherits Window {
        //settings, can only change by hitting apply after changing them in the setting menu
        in property <int> default_target_size: 25;
        in property <string> default_size_unit: "MB";
        in property <bool> overwrite: true;
        in property <string> output_name_style: "_Compressed"; //"_Compressed" or "timestamp"
        in property <bool> two_pass_encoding: false;

        //properties that change at anytime during execution
        in property <string> compress_status;
        in property <bool> widgets-enabled: true;
        in property <string> input_path;
        in property <string> output_path;
        in-out property <int> target_size: default_target_size;
        in-out property <string> size_unit: default_size_unit;

        //Current active page, 0 = video, 1 = image, 2 = audio, 3 = settings
        out property <int> active_page:0;

        width: 480px;
        height: 330px;
        background: #272626;


        GridLayout {
            spacing: 12px;
            //Sidebar with settings menu and tabs for video, audio and images
            Rectangle {
                background: #555;
                width: 59px;
                height: 330px;

                video := SidebarButton {
                    y:10px;
                    activate => {
                        root.active_page = 0;
                    }
                    Image{
                        width: 50px;
                        source: @image-url("../icons/video.svg");  
                        colorize: (active_page == 0) ? #706d6d : black;
                    }

                }

                image := SidebarButton {
                    y:70px;
                    activate => {
                        root.active_page = 1;
                    }
                    Image{
                        width: 50px;
                        source: @image-url("../icons/image.svg");  
                        colorize: (active_page == 1) ? #706d6d : black;
                    }

                }

                audio := SidebarButton {
                    y:130px;
                    activate => {
                        root.active_page = 2;
                    }
                    Image{
                        width: 50px;
                        source: @image-url("../icons/audio.svg");  
                        colorize: (active_page == 2) ? #706d6d : black;
                    }

                }

                settings := SidebarButton {
                    y:260px;
                    activate => {
                        root.active_page = 3;
                    }
                    Image{
                        width: 50px;
                        source: @image-url("../icons/settings.svg");  
                        colorize: (active_page == 3) ? #706d6d : black;
                    }

                }
               

            }

            //Video tab
            VerticalLayout {
                visible: root.active_page == 0;
                padding-top: 15px;
                spacing: 15px;

                //Target Size
                HorizontalLayout {
                    Text {
                        color: white;
                        text:"Target File Size: ";
                        font-size: 15px;
                        height: 17px;
                    }
                }
                HorizontalLayout {
                    spacing: 15px;
                    LineEdit {
                        enabled: widgets-enabled;
                        width: 50px;
                        height: 30px;
                        text: target_size;
                        input-type: number;
                        horizontal-alignment: left;
                        //Keep target size less than 1000, anything else is proably unintentional or should be a different unit
                        edited => {
                            if self.text.to-float() > 9999{
                                self.text = 9999;
                            }
                            target_size = self.text.to-float();
                        }
                        
                    }
                    
                    ComboBox {
                        enabled: widgets-enabled;
                        width:70px;
                        height: 30px;
                        current-value: size_unit;
                        model: ["MB","GB"];
                        selected => {
                            size_unit = self.current-value;
                        }
                    } 
                    // Text {
                    //     color: white;
                    //     text:" Overwrite output.mp4:";
                    //     font-size: 15px;
                    //     height: 17px;
                    //     width: 155px;
                    // }
                    // //TODO: add bool overwrite as an argument to compress_video, if true pass -y ~~~\\output.mp4 into ffmpeg
                    // Switch {
                    //     enabled: widgets-enabled;
                    //     height: 10px;
                    // }
                }

                //Input
                HorizontalLayout {
                    Text {
                        color: white;
                        text:"Input File: ";
                        font-size: 15px;
                        height: 17px;
                    }   
                }
                HorizontalLayout {
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
                HorizontalLayout {
                    Text {
                        color: white;
                        text:"Output File Path: ";
                        font-size: 15px;
                        height: 17px;
                    }    
                }
                HorizontalLayout {
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
                HorizontalLayout {
                    spacing: 35px;
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

            //Image tab
            VerticalLayout {
                col: 1;
                visible: root.active_page == 1;
                padding-top: 15px;
                spacing: 15px;
                HorizontalLayout {
                    Text {
                        color: white;
                        text: "WIP IMAGE";
                        font-size: 15px;
                        height: 17px;
                    }
                }
            }

            //Audio tab
            VerticalLayout {
                col: 1;
                visible: root.active_page == 2;
                padding-top: 15px;
                spacing: 15px;
                Text {
                    color: white;
                    text: "WIP AUDIO";
                    font-size: 15px;
                    height: 17px;
                }
            }

            //Settings tab
            VerticalLayout {
                col: 1;
                visible: root.active_page == 3;
                padding-top: 15px;
                spacing: 15px;
                
                HorizontalLayout {
                    Text {
                    color: white;
                    text: "WIP SETTINGS";
                    font-size: 15px;
                    height: 17px;
                }
                }
            }
            
        }
    } 
}

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