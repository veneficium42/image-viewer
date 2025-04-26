use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::{env, time::*};

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, Delay, Frame, RgbImage};
use image::{DynamicImage, ImageFormat, ImageReader, imageops::FilterType};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use softbuffer::{Context, Surface};

#[derive(Default)]
struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    image: DynamicImage,
    resized_image: RgbImage,

    // animated images stuff
    is_animated: bool,
    frames: Vec<Frame>,
    resized_frames: Vec<Option<RgbImage>>,
    frame_dur: Vec<Delay>,
    // index of the current frame
    frame: usize,
    last_update: Option<Instant>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("A fantastic window!");
        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());
        let context = Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();

        let size = window.inner_size();

        self.resized_image = self
            .image
            .resize(size.width, size.height, FilterType::Nearest)
            .to_rgb8();

        if self.is_animated {
            self.frame_dur = self.frames.iter().map(|frame| frame.delay()).collect();
            self.last_update = Some(Instant::now());
            self.resized_frames = vec![None; self.frames.len()];
        }

        self.surface = Some(surface);
        self.window = Some(window);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close was requested; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let window = self
                    .window
                    .as_ref()
                    .expect("redraw request without a window");
                let surface = self
                    .surface
                    .as_mut()
                    .expect("redraw request without a surface");

                let size = window.inner_size();

                let mut buffer = surface.buffer_mut().unwrap();

                buffer.fill(0);

                window.pre_present_notify();

                if self.is_animated {
                    let curr_frame = self.frame;
                    let frame = self.resized_frames[curr_frame].clone();
                    match frame {
                        Some(frame) => {
                            self.resized_image = frame;
                        }
                        None => {
                            let original_frame = self.frames[curr_frame].clone();
                            let resized_frame =
                                DynamicImage::ImageRgba8(original_frame.into_buffer())
                                    .resize(size.width, size.height, FilterType::Nearest)
                                    .into_rgb8();
                            self.resized_frames[curr_frame] = Some(resized_frame);
                            self.resized_image = self.resized_frames[curr_frame].clone().unwrap();
                        }
                    }
                }

                for (img_row, buf_row) in self
                    .resized_image
                    .rows()
                    .zip(buffer.chunks_exact_mut(size.width as usize))
                {
                    for (i, pix) in img_row.enumerate() {
                        buf_row[i] = u32::from_be_bytes([0, pix.0[0], pix.0[1], pix.0[2]]);
                    }
                }

                buffer.present().unwrap();

                if self.is_animated {
                    if self.last_update.unwrap().elapsed() > self.frame_dur[self.frame].into() {
                        self.last_update = Some(Instant::now());
                        self.frame += 1;
                        if self.frame >= self.frames.len() {
                            self.frame = 0;
                        }
                    }

                    self.window.as_mut().unwrap().request_redraw();
                }
            }
            WindowEvent::Resized(new_size) => {
                let surface = self
                    .surface
                    .as_mut()
                    .expect("resize request without a surface");

                surface
                    .resize(
                        NonZeroU32::new(new_size.width).unwrap(),
                        NonZeroU32::new(new_size.height).unwrap(),
                    )
                    .expect("surface resize gone wrong");

                if !self.is_animated {
                    self.resized_image = self
                        .image
                        .resize(new_size.width, new_size.height, FilterType::Nearest)
                        .to_rgb8();
                } else {
                    self.resized_frames = vec![None; self.frames.len()];
                }
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("No file provided!\n");
        return Ok(());
    };

    let mut app = App::default();
    let image = ImageReader::open(args[1].clone()).unwrap();
    match image.format().unwrap() {
        ImageFormat::Gif => {
            app.is_animated = true;
            let file = BufReader::new(File::open(args[1].clone())?);
            let decoder = GifDecoder::new(file)?;
            let frames = decoder.into_frames().collect_frames()?;
            app.frames = frames;
        }
        _ => {
            app.image = image.decode().unwrap();
            app.is_animated = false;
        }
    };
    event_loop.run_app(&mut app)?;

    Ok(())
}
