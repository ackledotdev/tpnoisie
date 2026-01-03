use input::ffi::{
    libinput_event_pointer, libinput_event_pointer_get_dx, libinput_event_pointer_get_dy,
};
use input::{AsRaw, Libinput, LibinputInterface};
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::PlaybackState,
    sound::static_sound::StaticSoundData,
};
use libc::{O_RDONLY, O_RDWR, O_WRONLY};
use std::fs::{File, OpenOptions};
use std::os::unix::{fs::OpenOptionsExt, io::OwnedFd};
use std::path::Path;
use std::time::Duration;
struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_RDONLY != 0) | (flags & O_RDWR != 0))
            .write((flags & O_WRONLY != 0) | (flags & O_RDWR != 0))
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(File::from(fd));
    }
}

fn main() {
    let version = "0.3.1";
    println!("tpnoisie v{}", version);

    let multiplier = 6.0;

    let volume_adjustment: f32 = -96.0;

    let args = std::env::args().collect::<Vec<_>>();

    let device_path_supplied = args.get(1).unwrap_or_else(|| {
        println!("No device path provided.");
        println!("Scanning for input devices...");
        scan_print_input_devices();
        std::process::exit(0);
    });
    let audio_path = args.get(2).unwrap_or_else(|| {
        println!("No audio path provided.");
        std::process::exit(1);
    });

    let devices = evdev::enumerate();
    let mut device_path = String::new();
    if device_path_supplied == "auto" {
        println!("Auto-detecting TrackPoint device...");
        let mut found = false;
        for dev in devices {
            let name = dev.1.name().unwrap_or("Unknown device");
            let devpath = dev.0.as_path().to_str().clone();
            if name.to_lowercase().contains("trackpoint") {
                device_path = devpath
                    .expect(&format!("Failed to convert path for device {}", name))
                    .to_string();
                println!("Auto-detected TrackPoint device: {}", device_path);
                found = true;
                break;
            }
        }
        if !found {
            println!("No TrackPoint device found.");
            std::process::exit(1);
        }
    } else {
        device_path = device_path_supplied.to_string();
    }

    if !std::fs::exists(&device_path).unwrap_or(false) {
        println!("Device path does not exist.");
        std::process::exit(1);
    }
    if !std::fs::exists(audio_path).unwrap_or(false) {
        println!("Audio directory does not exist.");
        std::process::exit(1);
    }

    let audio_type = args.get(3).unwrap_or(&"wav".to_string()).to_lowercase();

    if audio_type != "wav" && audio_type != "ogg" {
        println!(
            "Unsupported audio type: {}. Only 'wav' and 'ogg' are supported.",
            audio_type
        );
        std::process::exit(1);
    }

    for i in 0..=9 {
        if !std::fs::exists(format!("{}/{}.{}", audio_path, i, audio_type)).unwrap_or(false) {
            println!("Audio file {audio_path}/{i}.{audio_type} does not exist.");
            std::process::exit(1);
        }
    }

    let mut input = Libinput::new_from_path(Interface);
    println!("Using device: {}", device_path);
    let _device = input
        .path_add_device(device_path.as_str())
        .expect("Failed to add hardware device");

    println!("Loading sounds from {}", audio_path);
    // Create an audio manager. This plays sounds and manages resources.
    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
        .expect("Failed to create audio manager");
    let mut sound_data = Vec::new();
    // load sounds
    for i in 0..=9 {
        sound_data.push(
            StaticSoundData::from_file(&format!("{audio_path}/{i}.{audio_type}"))
                .expect("Failed to load sound")
                .volume(volume_adjustment),
        );
    }

    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }

    println!("Starting input dispatch loop");

    let mut current_handle = None;

    while running.load(Ordering::SeqCst) {
        input.dispatch().expect("Failed to dispatch input");
        for event in &mut input {
            unsafe {
                // if event
                //     .type_id()
                //     .ne(&std::any::TypeId::of::<input::event::PointerEvent>())
                // {
                //     continue;
                // }

                // Check if current sound is still playing
                if !current_handle
                    .as_ref()
                    .map(|handle: &kira::sound::static_sound::StaticSoundHandle| {
                        !handle.state().eq(&PlaybackState::Playing)
                    })
                    .unwrap_or(true)
                {
                    continue;
                }

                let ev_ptr = event.as_raw();

                let dx = libinput_event_pointer_get_dx(ev_ptr as *mut libinput_event_pointer);
                let dy = libinput_event_pointer_get_dy(ev_ptr as *mut libinput_event_pointer);

                let threshold = 0.0;

                if dx <= threshold && dy <= threshold {
                    continue;
                }

                // use pythagorean theorem and log scale to obtain a scaled magnitude of velocity
                let mut speed = ((dx * dx + dy * dy) * multiplier as f64)
                    .sqrt()
                    .log(1.8)
                    // let the speed go over 10 so random sounds can be triggered
                    .clamp(1.0, 12.0) as i8;

                let raw_speed = speed;

                // choose randomly if speed is over 10
                if speed > 10 {
                    println!("Random speed next; original: {}", speed);
                    speed = rand::random_range(6..=10);
                } else if speed <= 0 {
                    continue;
                } else if speed == 1 {
                    // to avoid too many speed 1 sounds, sometimes pick 2 or 3 instead
                    let roll: f64 = rand::random();
                    if roll < 0.3 {
                        speed = 2;
                    } else if roll < 0.5 {
                        speed = 3;
                    }
                }

                if speed == raw_speed {
                    println!("Speed: {}", speed);
                } else {
                    println!("Speed: {} (Raw {})", speed, raw_speed);
                }

                // play the selected audio file
                let handle = manager
                    .play(sound_data[speed as usize - 1].clone())
                    .expect("Failed to play sound");
                current_handle = Some(handle);
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}

fn scan_print_input_devices() {
    let mut table = vec!["TrackPoint?\tDevice Name\tDevice Path".to_string()];
    let devs = evdev::enumerate();
    for dev in devs {
        let path = dev.0.as_path().to_str().expect("Failed to convert path");
        let name = dev.1.name().unwrap_or("Unknown device");
        table.push(format!(
            "{}\t{}\t{}",
            if name.to_lowercase().contains("trackpoint") {
                "XXXXXXXXXXX"
            } else {
                " "
            },
            name,
            path
        ));
    }
    format_table::format_table(table);
}
