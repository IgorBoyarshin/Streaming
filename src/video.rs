use glium::{implement_vertex, uniform, Surface};
use uvc::{Frame};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::{thread, time};


fn frame_to_raw_image(
    frame: &Frame,
) -> Result<glium::texture::RawImage2d<'static, u8>, Box<dyn Error>> {
    let new_frame = frame.to_rgb()?;
    let data = new_frame.to_bytes();

    let image = glium::texture::RawImage2d::from_raw_rgb(
        data.to_vec(),
        (new_frame.width(), new_frame.height()),
    );

    Ok(image)
}

fn callback_frame_to_image(
    frame: &Frame,
    data: &mut Arc<Mutex<Option<glium::texture::RawImage2d<u8>>>>,
) {
    let image = frame_to_raw_image(frame);
    match image {
        Err(x) => println!("{:#?}", x),
        Ok(x) => {
            let mut data = Mutex::lock(&data).unwrap();
            *data = Some(x);
        }
    }
}


// fn main() {
//     println!("========== Starting ==========");
//
//     let ctx = uvc::Context::new().expect("Could not get context");
//     let dev = ctx
//         .find_device(Some(0x0bda), Some(0x5652), None)
//         .expect("Could not find device");
//
//     let description = dev.description().unwrap();
//     println!(
//         "Found device: Bus {:03} Device {:03} : ID {:04x}:{:04x} {} ({})",
//         dev.bus_number(),
//         dev.device_address(),
//         description.vendor_id,
//         description.product_id,
//         description.product.unwrap_or_else(|| "Unknown".to_owned()),
//         description
//         .manufacturer
//         .unwrap_or_else(|| "Unknown".to_owned())
//     );
//
//     let devh = dev.open().expect("Could not open device");
//
//     // for i in devh.supported_formats().into_iter() {
//     //     println!(":> Subtype = {:?}", i.subtype());
//     //     for j in i.supported_formats().into_iter() {
//     //         // println!("Subtype inner = {:?}", j.subtype());
//     //         println!("[{}, {}] intervals={:?} millis, interval durations={:?}",
//     //             j.width(), j.height(), j.intervals().iter().map(|&x| x / 10_000).collect::<Vec<_>>(), j.intervals_duration());
//     //     }
//     // }
//
//     // Most webcams support this format
//     let format = uvc::StreamFormat {
//         width: 1280,
//         height: 720,
//         fps: 30,
//         format: uvc::FrameFormat::MJPEG,
//     };
//
//     // Get the necessary stream information
//     let mut streamh = devh
//         .get_stream_handle_with_format(format)
//         .expect("Could not open a stream with this format");
//
//     let frame = Arc::new(Mutex::new(None));
//     let _stream = streamh
//         .start_stream(callback_frame_to_image, frame.clone())
//         .unwrap();
//
//
//     use glium::glutin;
//     let events_loop = glutin::event_loop::EventLoop::new();
//     let window = glutin::window::WindowBuilder::new().with_title("Mirror");
//     let context = glutin::ContextBuilder::new();
//     let display = glium::Display::new(window, context, &events_loop).unwrap();
//
//
//     #[derive(Copy, Clone)]
//     pub struct QuadVertex {
//         pos: (f32, f32),
//     }
//     implement_vertex!(QuadVertex, pos);
//
//
//     let vertices: [QuadVertex; 4] = [
//         QuadVertex { pos: (-1.0, -1.0) },
//         QuadVertex { pos: (-1.0, 1.0) },
//         QuadVertex { pos: (1.0, -1.0) },
//         QuadVertex { pos: (1.0, 1.0) },
//     ];
//     let vertices = glium::VertexBuffer::new(&display, &vertices).unwrap();
//
//
//     let indices: [u8; 6] = [0, 1, 2, 1, 3, 2];
//     let indices = glium::IndexBuffer::new(
//         &display,
//         glium::index::PrimitiveType::TrianglesList,
//         &indices,
//     ).unwrap();
//
//
//     let vertex_shader_source = r#"
//         #version 140
//
//         in vec2 pos;
//
//         out vec2 v_position;
//
//         void main() {
//             v_position = (pos + 1.0)/2.0;
//             gl_Position = vec4(-pos.x, -pos.y, 0.0, 1.0);
//         }
//     "#;
//     let fragment_shader_source = r#"
//         #version 140
//
//         in vec2 v_position;
//
//         out vec4 color;
//
//         uniform sampler2D u_image;
//
//         void main() {
//             vec2 pos = v_position;
//
//             color = texture(u_image, pos);
//         }
//     "#;
//     let program = glium::Program::from_source(&display, vertex_shader_source, fragment_shader_source, None).unwrap();
//
//     let mut cnt = 0;
//     let mut cnt2 = 0;
//
//     let mut buffer: Option<glium::texture::SrgbTexture2d> = None;
//     events_loop.run(move |event, _, control_flow| {
//         if let glutin::event::Event::WindowEvent { event, .. } = event {
//             if let glutin::event::WindowEvent::CloseRequested = event {
//                 *control_flow = glutin::event_loop::ControlFlow::Exit;
//                 println!("Frames={}, Passed={}", cnt2, cnt);
//                 return;
//             }
//         }
//
//         let mut target = display.draw();
//         target.clear_color(0.0, 0.0, 1.0, 1.0);
//
//         let mut mutex = Mutex::lock(&frame).unwrap();
//
//         match mutex.take() {
//             None => {
//                 println!("Nothing new so far...");
//                 cnt += 1;
//                 // No new frame to render
//             }
//             Some(image) => {
//                 cnt2 += 1;
//                 println!("New frame");
//                 let image = glium::texture::SrgbTexture2d::new(&display, image)
//                     .expect("Could not use image");
//                 buffer = Some(image);
//             }
//         }
//
//         if let Some(ref b) = buffer {
//             let uniforms = uniform! { u_image: b };
//             target.draw(
//                 &vertices,
//                 &indices,
//                 &program,
//                 &uniforms,
//                 &Default::default(),
//             ).unwrap();
//         }
//
//         target.finish().unwrap();
//
//         std::thread::sleep(time::Duration::from_millis(30));
//     });
//
//     // println!(
//     //     "Scanning mode: {:?}\nAuto-exposure mode: {:?}\nAuto-exposure priority: {:?}\nAbsolute exposure: {:?}\nRelative exposure: {:?}\nAboslute focus: {:?}\nRelative focus: {:?}",
//     //     devh.scanning_mode(),
//     //     devh.ae_mode(),
//     //     devh.ae_priority(),
//     //     devh.exposure_abs(),
//     //     devh.exposure_rel(),
//     //     devh.focus_abs(),
//     //     devh.focus_rel(),
//     // );
//
//     // // This is a counter, increasing by one for every frame
//     // // This data must be 'static + Send + Sync to be used in
//     // // the callback used in the stream
//     // let counter = Arc::new(AtomicUsize::new(0));
//     //
//     // // Get a stream, calling the closure as callback for every frame
//     // let stream = streamh
//     //     .start_stream(
//     //         |_frame, count| {
//     //             count.fetch_add(1, Ordering::SeqCst);
//     //         },
//     //         counter.clone(),
//     //     ).expect("Could not start stream");
//     //
//     // // Wait 10 seconds
//     // std::thread::sleep(Duration::new(3, 0));
//     //
//     // // Explicitly stop the stream
//     // // The stream would also be stopped
//     // // when going out of scope (dropped)
//     // stream.stop();
//     // println!("Counter: {}", counter.load(Ordering::SeqCst));
//
//     // println!("========== Ending ==========");
// }
