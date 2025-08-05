use std::fs::File;
use std::io::BufReader;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::{env, time::*};

use anyhow::Context;

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, Delay, Frame, RgbImage};
use image::{DynamicImage, ImageFormat, ImageReader, imageops::FilterType};

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use softbuffer::Surface;

#[derive(Default)]
struct App {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    image: DynamicImage,
    resized_image: RgbImage,

    // animated images stuff
    animation: Option<Animation>,
}

#[derive(Default)]
struct Animation {
    frames: Vec<Frame>,
    resized_frames: Vec<Option<RgbImage>>,
    frame_dur: Vec<Delay>,
    //current frame
    frame: usize,
    last_update: Option<Instant>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("A fantastic window!");
        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();

        let size = window.inner_size();

        self.resized_image = self
            .image
            .resize(size.width, size.height, FilterType::Nearest)
            .to_rgb8();

        if let Some(anim) = self.animation.as_mut() {
            anim.frame_dur = anim.frames.iter().map(|frame| frame.delay()).collect();
            anim.last_update = Some(Instant::now());
            anim.resized_frames = vec![None; anim.frames.len()];
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

                if let Some(anim) = self.animation.as_mut() {
                    let curr_frame = anim.frame;
                    match anim.resized_frames[curr_frame].as_ref() {
                        Some(frame) => {
                            self.resized_image = frame.clone();
                        }
                        None => {
                            let original_buffer = anim.frames[curr_frame].buffer().clone();
                            let resized_frame = DynamicImage::ImageRgba8(original_buffer)
                                .resize(size.width, size.height, FilterType::Nearest)
                                .into_rgb8();
                            anim.resized_frames[curr_frame] = Some(resized_frame);
                            self.resized_image = anim.resized_frames[curr_frame].clone().unwrap();
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

                if let Some(anim) = self.animation.as_mut() {
                    if anim.last_update.unwrap().elapsed() > anim.frame_dur[anim.frame].into() {
                        anim.last_update = Some(Instant::now());
                        anim.frame += 1;
                        if anim.frame >= anim.frames.len() {
                            anim.frame = 0;
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

                if let Some(anim) = self.animation.as_mut() {
                    anim.resized_frames = vec![None; anim.frames.len()];
                } else {
                    self.resized_image = self
                        .image
                        .resize(new_size.width, new_size.height, FilterType::Nearest)
                        .to_rgb8();
                }
            }
            _ => (),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new().context("Failed to create event loop")?;
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("No file provided!\n");
        return Ok(());
    };

    let mut app = App::default();
    let image = ImageReader::open(args[1].clone())
        .context(format!("Failed to open image at {}", args[1]))?;
    match image.format().unwrap() {
        ImageFormat::Gif => {
            let mut animation = Animation::default();
            let file = BufReader::new(
                File::open(args[1].clone())
                    .context(format!("Failed to open GIF file at {}", args[1]))?,
            );
            let decoder = GifDecoder::new(file).context("Failed to create GIF decoder")?;
            let frames = decoder
                .into_frames()
                .collect_frames()
                .context("Failed to grab GIF frames")?;
            animation.frames = frames;

            app.animation = Some(animation);
        }
        _ => {
            app.image = image.decode().context("Failed to decode image")?;
        }
    };
    event_loop.run_app(&mut app)?;

    Ok(())
}
