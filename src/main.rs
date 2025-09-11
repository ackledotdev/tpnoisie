use input::ffi::{
    libinput_event_pointer, libinput_event_pointer_get_dx, libinput_event_pointer_get_dy,
};
use input::{AsRaw, Libinput, LibinputInterface};
use kira::sound::PlaybackState;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
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
    let device_path = "/dev/input/event5"; // Change this to your device path
    let wav_path = "/home/ackle/bin/nubmoan/moanswav"; // Change this to your WAV files path

    let mut input = Libinput::new_from_path(Interface);
    println!("Using device: {}", device_path);
    let _device = input
        .path_add_device(device_path)
        .expect("Failed to add hardware device");

    println!("Loading sounds from {}", wav_path);
    // Create an audio manager. This plays sounds and manages resources.
    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
        .expect("Failed to create audio manager");
    let mut sound_data = Vec::new();
    // load sounds
    for i in 1..=10 {
        sound_data.push(
            StaticSoundData::from_file(&format!("{}/{}.wav", wav_path, i))
                .expect("Failed to load sound"),
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

                let ev_ptr = event.as_raw();

                let dx = libinput_event_pointer_get_dx(ev_ptr as *mut libinput_event_pointer);
                let dy = libinput_event_pointer_get_dy(ev_ptr as *mut libinput_event_pointer);

                let threshold = 0.1;

                if dx <= threshold && dy <= threshold {
                    continue;
                }

                // use pythagorean theorem and log scale to obtain a scaled magnitude of velocity
                let mut speed = (((dx * dx + dy * dy) * 6.0 as f64).sqrt())
                    .log2()
                    .abs()
                    .trunc() as i64;

                // just in case
                if speed <= 0 || speed > 10 {
                    println!("Random speed next; original: {}", speed);
                    speed = rand::random_range(5..=8) as i64;
                }

                println!("Speed: {}", speed);

                // play the selected audio file
                let handle = manager
                    .play(sound_data[speed as usize - 1].clone())
                    .expect("Failed to play sound");
                while handle.state().eq(&PlaybackState::Playing) {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }
}
