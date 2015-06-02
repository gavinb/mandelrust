//============================================================================
//
// A simple Mandelbrot image generator in Rust
//
// Command-line Interface
//
// Copyright (c) 2014 Gavin Baker <gavinb@antonym.org>
// Published under the MIT license
//
//============================================================================

use std::vec::Vec;
use std::fs::File;
use std::thread;
use std::io::{Error, ErrorKind, Write};
use std::sync::mpsc::{channel, Sender, Receiver};

use engine::MandelEngine;
use protocol::{RenderType, EngineCommand, EngineStatus, PREVIEW_WIDTH, PREVIEW_HEIGHT};

mod engine;
mod protocol;

//----------------------------------------------------------------------------

struct CommandLine {
    width: u32,
    height: u32,
    chan_cli_to_engine: Option<Sender<protocol::EngineCommand>>,
    chan_cli_from_engine: Option<Receiver<protocol::EngineStatus>>,
    chan_engine_to_cli: Option<Sender<protocol::EngineStatus>>,
    chan_engine_from_cli: Option<Receiver<protocol::EngineCommand>>,
    image: Option<Vec<u8>>,
}

impl CommandLine {

    pub fn new(w: u32, h: u32) -> CommandLine {
        let (chan_cli_to_engine, chan_engine_from_cli) = channel();
        let (chan_engine_to_cli, chan_cli_from_engine) = channel();
        CommandLine {
            width: w,
            height: h,
            chan_cli_to_engine: Some(chan_cli_to_engine),
            chan_cli_from_engine: Some(chan_cli_from_engine),
            chan_engine_to_cli: Some(chan_engine_to_cli),
            chan_engine_from_cli: Some(chan_engine_from_cli),
            image: None,
        }
    }

    pub fn start_engine(&mut self) {

        let progress_ch = self.chan_engine_to_cli.take().expect("no engine_to_cli chan");
        let cmd_ch = self.chan_engine_from_cli.take().expect("no engine_from_cli chan");

        let (w,h) = (self.width, self.height);

        thread::spawn( move || {
            let mut engine = MandelEngine::new(w, h);
            engine.serve(&cmd_ch, &progress_ch);
        });

        match self.chan_cli_to_engine {
            Some(ref cmd_ch) => cmd_ch.send(EngineCommand::Render(RenderType::FullRender)).unwrap(),
            _ => panic!("No chan")
        }
    }

    pub fn stop_engine(&mut self) {
        match self.chan_cli_to_engine {
            Some(ref cmd_ch) => cmd_ch.send(EngineCommand::Shutdown).unwrap(),
            _ => panic!("No chan")
        }
    }

    pub fn handle_update(&mut self) -> bool {
        match self.chan_cli_from_engine {
            Some(ref ch) => {
                let status_msg = ch.try_recv();
                match status_msg {
                    Ok(status) =>
                        match status {
                            EngineStatus::Startup => {
                                println!("Startup...");
                                false
                            },
                            EngineStatus::Processing(progress) => {
                                println!("Processing {}", progress);
                                false
                            },
                            EngineStatus::RenderComplete(typ, img) => {
                                println!("Render Complete!");
                                self.image = Some(img);
                                match typ {
                                    RenderType::FullRender => {
                                        println!("fullRender {} {}", self.width, self.height);
                                    },
                                    RenderType::PreviewRender => {
                                        println!("Preview {} {}", PREVIEW_WIDTH, PREVIEW_HEIGHT);
                                    },
                                };
                                true
                            },
                            EngineStatus::Error(code) => {
                                println!("Error {}", code);
                                false
                            },
                        },
                    _ => false,
                }
            },
            None => false,
        }
    }

    pub fn save_as_ppm(&self, filename: &str) -> std::io::Result<()> {
        match self.image {
            Some(ref img) => {
                println!("Saving {}", filename);
                let mut file = try!(File::create(filename));
                try!(file.write_all("P6\n".as_bytes()));
                try!(file.write_all(format!("{} {}\n255\n", self.width, self.height).as_bytes()));
                try!(file.write_all(&img));
                Ok(())
            },
            None => Err(Error::new(ErrorKind::NotFound, "file")),
        }
    }
}

fn main() {

    let mut cli = CommandLine::new(640, 640);

    cli.start_engine();

    loop {
        if cli.handle_update() == true {
            cli.save_as_ppm("test.ppm").unwrap();
            cli.stop_engine();
            break;
        }
    }
}
