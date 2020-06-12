use glium::{implement_vertex, uniform, Surface};
use uvc::{Frame};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::{thread, time};


// fn main() {
//
//     let frames: Vec<Arc<Mutex<Option<glium::texture::RawImage2d<u8>>>>> = Vec::new();
// }

#[derive(Copy, Clone)]
pub struct QuadVertex {
    pos: (f32, f32),
    uv: (f32, f32),
}

fn generate_vertices(amount_without_self: u8, display: &glium::Display) -> Vertices {
    if amount_without_self > 3 {
        panic!("Too many people!!!");
    }

    // let small : glium::VertexBuffer<QuadVertex> = if amount_without_self == 0 {
    //     let vertices_full: [QuadVertex; 4] = [
    //         QuadVertex { pos: (-1.0, -1.0), uv: (1.0, 1.0) },
    //         QuadVertex { pos: (-1.0, 1.0), uv: (1.0, 0.0) },
    //         QuadVertex { pos: (1.0, -1.0), uv: (0.0, 1.0) },
    //         QuadVertex { pos: (1.0, 1.0), uv: (0.0, 0.0) },
    //     ];
    //     glium::VertexBuffer::new(&display, &vertices_full).unwrap()
    // } else if amount_without_self == 1 {
    //     let vertices_small: [QuadVertex; 4] = [
    //         QuadVertex { pos: (0.6, -0.9), uv: (1.0, 1.0) },
    //         QuadVertex { pos: (0.6, -0.6), uv: (1.0, 0.0) },
    //         QuadVertex { pos: (0.9, -0.9), uv: (0.0, 1.0) },
    //         QuadVertex { pos: (0.9, -0.6), uv: (0.0, 0.0) },
    //     ];
    //     glium::VertexBuffer::new(&display, &vertices_small).unwrap()
    // } else if amount_without_self == 2 || amount_without_self == 3 {
    //     let vertices: [QuadVertex; 4] = [
    //         QuadVertex { pos: (0.0, -1.0), uv: (1.0, 1.0) },
    //         QuadVertex { pos: (0.0, 0.0), uv: (1.0, 0.0) },
    //         QuadVertex { pos: (1.0, -1.0), uv: (0.0, 1.0) },
    //         QuadVertex { pos: (1.0, 0.0), uv: (0.0, 0.0) },
    //     ];
    //     glium::VertexBuffer::new(&display, &vertices).unwrap()
    // };

    let mut others = Vec::new();
    // if amount_without_self == 1 {
    //     let vertices_full: [QuadVertex; 4] = [
    //         QuadVertex { pos: (-1.0, -1.0), uv: (1.0, 1.0) },
    //         QuadVertex { pos: (-1.0, 1.0), uv: (1.0, 0.0) },
    //         QuadVertex { pos: (1.0, -1.0), uv: (0.0, 1.0) },
    //         QuadVertex { pos: (1.0, 1.0), uv: (0.0, 0.0) },
    //     ];
    //     others.push(glium::VertexBuffer::new(&display, &vertices_full).unwrap());
    // } else if amount_without_self == 2 {
    //     let vertices_1: [QuadVertex; 4] = [
    //         QuadVertex { pos: (-1.0, -1.0), uv: (1.0, 1.0) },
    //         QuadVertex { pos: (-1.0, 0.0), uv: (1.0, 0.0) },
    //         QuadVertex { pos: (0.0, -1.0), uv: (0.0, 1.0) },
    //         QuadVertex { pos: (0.0, 0.0), uv: (0.0, 0.0) },
    //     ];
    //     let vertices_2: [QuadVertex; 4] = [
    //         QuadVertex { pos: (-1.0, 0.0), uv: (1.0, 1.0) },
    //         QuadVertex { pos: (-1.0, 1.0), uv: (1.0, 0.0) },
    //         QuadVertex { pos: (0.0, 0.0), uv: (0.0, 1.0) },
    //         QuadVertex { pos: (0.0, 1.0), uv: (0.0, 0.0) },
    //     ];
    //     others.push(glium::VertexBuffer::new(&display, &vertices_1).unwrap());
    //     others.push(glium::VertexBuffer::new(&display, &vertices_2).unwrap());
    // } else if amount_without_self == 3 {
    //     let vertices_1: [QuadVertex; 4] = [
    //         QuadVertex { pos: (-1.0, -1.0), uv: (1.0, 1.0) },
    //         QuadVertex { pos: (-1.0, 0.0), uv: (1.0, 0.0) },
    //         QuadVertex { pos: (0.0, -1.0), uv: (0.0, 1.0) },
    //         QuadVertex { pos: (0.0, 0.0), uv: (0.0, 0.0) },
    //     ];
    //     let vertices_2: [QuadVertex; 4] = [
    //         QuadVertex { pos: (-1.0, 0.0), uv: (1.0, 1.0) },
    //         QuadVertex { pos: (-1.0, 1.0), uv: (1.0, 0.0) },
    //         QuadVertex { pos: (0.0, 0.0), uv: (0.0, 1.0) },
    //         QuadVertex { pos: (0.0, 1.0), uv: (0.0, 0.0) },
    //     ];
    //     let vertices_3: [QuadVertex; 4] = [
    //         QuadVertex { pos: (0.0, 0.0), uv: (1.0, 1.0) },
    //         QuadVertex { pos: (0.0, 1.0), uv: (1.0, 0.0) },
    //         QuadVertex { pos: (1.0, 0.0), uv: (0.0, 1.0) },
    //         QuadVertex { pos: (1.0, 1.0), uv: (0.0, 0.0) },
    //     ];
    //     others.push(glium::VertexBuffer::new(&display, &vertices_1).unwrap());
    //     others.push(glium::VertexBuffer::new(&display, &vertices_2).unwrap());
    //     others.push(glium::VertexBuffer::new(&display, &vertices_3).unwrap());
    // }

    Vertices { small, others }
}

struct Vertices {
    small: glium::VertexBuffer<QuadVertex>,
    others: Vec<glium::VertexBuffer<QuadVertex>>,
}



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

// fn start() -> (Arc<Mutex<Option<glium::texture::RawImage2d<'static, u8>>>>, uvc::streaming::ActiveStream) {
//
//     // thread::spawn(move|| {
//     // });
//
//     println!("After");
//
//     (frame, _stream)
// }

fn main() {
    println!("========== Starting ==========");

    let frame : Arc<Mutex<Option<glium::texture::RawImage2d<u8>>>> = Arc::new(Mutex::new(None));
    // let ctx = uvc::Context::new().expect("Could not get context");
    // let dev = ctx
    //     .find_device(Some(0x0bda), Some(0x5652), None)
    //     .expect("Could not find device");
    //
    // let description = dev.description().unwrap();
    // println!(
    //     "Found device: Bus {:03} Device {:03} : ID {:04x}:{:04x} {} ({})",
    //     dev.bus_number(),
    //     dev.device_address(),
    //     description.vendor_id,
    //     description.product_id,
    //     description.product.unwrap_or_else(|| "Unknown".to_owned()),
    //     description
    //     .manufacturer
    //     .unwrap_or_else(|| "Unknown".to_owned())
    // );
    //
    // let devh = dev.open().expect("Could not open device");
    //
    // // Most webcams support this format
    // let format = uvc::StreamFormat {
    //     width: 1280,
    //     height: 720,
    //     fps: 30,
    //     format: uvc::FrameFormat::MJPEG,
    // };
    //
    // // Get the necessary stream information
    // let mut streamh = devh
    //     .get_stream_handle_with_format(format)
    //     .expect("Could not open a stream with this format");
    //
    // let _stream = streamh
    //     .start_stream(callback_frame_to_image, frame.clone())
    //     .unwrap();

    use glium::glutin;
    let events_loop = glutin::event_loop::EventLoop::new();
    let window = glutin::window::WindowBuilder::new().with_title("Mirror");
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    implement_vertex!(QuadVertex, pos, uv);

    let (window_width, window_height) = {
        let size = display.gl_window().window().inner_size();
        (size.width, size.height)
    };
    println!("Size={},{}", window_width, window_height);


    // #[derive(Copy, Clone)]
    // pub struct QuadVertex {
    //     pos: (f32, f32),
    //     uv: (f32, f32),
    // }
    // implement_vertex!(QuadVertex, pos, uv);





    let indices: [u8; 6] = [0, 1, 2, 1, 3, 2];
    let indices = glium::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &indices,
    ).unwrap();

    let program : glium::program::Program= glium::Program::from_source(&display, vertex_shader(), fragment_shader(), None).unwrap();

    let mut cnt = 0;
    let mut cnt2 = 0;

    let mut buffer: Option<glium::texture::SrgbTexture2d> = None;
    events_loop.run(move |event, _, control_flow| {
        if let glutin::event::Event::WindowEvent { event, .. } = event {
            if let glutin::event::WindowEvent::CloseRequested = event {
                *control_flow = glutin::event_loop::ControlFlow::Exit;
                println!("Frames={}, Passed={}", cnt2, cnt);
                return;
            }
        }

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 1.0, 1.0);

        match Mutex::lock(&frame).unwrap().take() {
            None => {
                // println!("Nothing new so far...");
                cnt += 1;
                // No new frame to render
            }
            Some(image) => {
                cnt2 += 1;
                // println!("New frame");
                let image = glium::texture::SrgbTexture2d::new(&display, image)
                    .expect("Could not use image");
                buffer = Some(image);
            }
        }

        // if let Some(ref b) = buffer {
        //     let uniforms = uniform! { u_image: b };
        //     target.draw(
        //         &vertices_full,
        //         &indices,
        //         &program,
        //         &uniforms,
        //         &Default::default(),
        //     ).unwrap();
        // }
        //
        // if let Some(ref b) = buffer {
        //     let uniforms = uniform! { u_image: b };
        //     target.draw(
        //         &vertices_small,
        //         &indices,
        //         &program,
        //         &uniforms,
        //         &Default::default(),
        //     ).unwrap();
        // }

        target.finish().unwrap();

        std::thread::sleep(time::Duration::from_millis(30));
    });
}

// struct Image {
//     program: glium::program::program::Program,
//     vertices: glium::vertex::buffer::VertexBuffer,
//     // indices
// }

fn vertex_shader() -> &'static str {
    r#"
        #version 140

        in vec2 pos;
        in vec2 uv;
        out vec2 v_position;

        void main() {
            v_position = uv;
            gl_Position = vec4(pos.x, pos.y, 0.0, 1.0);
        }
    "#
}

fn fragment_shader() -> &'static str {
    r#"
        #version 140

        in vec2 v_position;
        out vec4 color;
        uniform sampler2D u_image;

        void main() {
            vec2 pos = v_position;

            color = texture(u_image, pos);
        }
    "#
}
