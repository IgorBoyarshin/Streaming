use glium::{implement_vertex, uniform, Surface};
use uvc::{Frame};
use std::error::Error;
use std::sync::{Arc, Mutex, RwLock};
// use std::{thread, time};

use flate2::write::DeflateEncoder;
use flate2::bufread::DeflateDecoder;
use flate2::Compression;
use std::io::prelude::*;
use std::time::{Instant, Duration};

use std::env;
use std::net::{TcpStream, TcpListener};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;


// ============================================================================
// ============================================================================
// ============================================================================
fn generate_random_id() -> u8 {
    rand::random::<u8>()
}

fn main() {
    let client_connect_to = "127.0.0.1:1234";
    let server_at         = "127.0.0.1:1234";

    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!(":> Please specify either 'client' or 'server'");
    } else {
        if args[1] == "client" {
            as_client(generate_random_id(), client_connect_to);
        } else if args[1] == "server" {
            as_server(server_at);
        } else {
            println!(":> Please specify either 'client' or 'server'");
        }
    }
}
// ============================================================================
// ============================================================================
// ============================================================================
type Consumer = Sender<Packet>;

#[derive(Clone)]
struct Packet {
    from_id: u8,
    order: u32,
    width: u16,
    height: u16,
    compressed_length: u16, // is equal to compressed_video.len()
    compressed_video: Vec<u8>,
}

fn read_packet(stream: &mut TcpStream) -> Result<Packet, std::io::Error> {
    let mut buff = [0_u8; 11]; // Packet header size in bytes
    stream.read_exact(&mut buff)?;
    let from_id = buff[0];
    let order = ((buff[1] as u32) << 24) |
                ((buff[2] as u32) << 16) |
                ((buff[3] as u32) << 8) |
                ( buff[4] as u32);
    let width = ((buff[5] as u16) << 8) |
                ( buff[6] as u16);
    let height = ((buff[7] as u16) << 8) |
                 ( buff[8] as u16);
    let compressed_length = ((buff[9] as u16) << 8) |
                            ( buff[10] as u16);
    let mut compressed_video = Vec::new();
    compressed_video.resize(compressed_length as usize, 0);
    stream.read_exact(&mut compressed_video[..]).map(|()| {
        Packet {
            from_id,
            order,
            width,
            height,
            compressed_length,
            compressed_video,
        }
    })
}

fn write_packet(packet: Packet, stream: &mut TcpStream) -> Result<(), std::io::Error> {
    let Packet{from_id, order, width, height, compressed_length, mut compressed_video} = packet;

    let mut buff = Vec::with_capacity(11 + compressed_length as usize);
    buff.push(from_id);
    buff.push((order >> 24) as u8);
    buff.push((order >> 16) as u8);
    buff.push((order >> 8)  as u8);
    buff.push( order        as u8);
    buff.push((width >> 8) as u8);
    buff.push( width       as u8);
    buff.push((height >> 8) as u8);
    buff.push( height       as u8);
    buff.push((compressed_length >> 8) as u8);
    buff.push( compressed_length       as u8);
    buff.append(&mut compressed_video);

    stream.write_all(&buff[..])
}
// ============================================================================
// ============================================================================
// ============================================================================
fn as_client(id: u8, server_address: &str) {
    // ===================== Setup graphics ======================

    implement_vertex!(QuadVertex, pos, uv);
    use glium::glutin;
    let events_loop = glutin::event_loop::EventLoop::new();
    let window = glutin::window::WindowBuilder::new().with_title("Mirror");
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let (vertex_layout, display) = generate_vertex_layout(display);
    let indices = glium::IndexBuffer::new(
        &display,
        glium::index::PrimitiveType::TrianglesList,
        &vec![0_u8, 1_u8, 2_u8, 1_u8, 3_u8, 2_u8],
    ).unwrap();
    let program = glium::Program::from_source(&display, vertex_shader(), fragment_shader(), None).unwrap();

    // ===================== Setup frames ======================

    let max_amount_of_people = 4;
    let amount_of_people = Arc::new(RwLock::new(1 as usize)); // start with just self TODO: remove usize

    // Stores actual camera frames
    let mut frames: Vec<Arc<Mutex<Option<glium::texture::RawImage2d<u8>>>>> = Vec::new();
    while frames.len() < max_amount_of_people {
        frames.push(Arc::new(Mutex::new(None)));
    }

    // Temporary buffer for opengl
    let mut buffers: Vec<Option<glium::texture::SrgbTexture2d>> = Vec::new(); // start with single (self)
    while buffers.len() < max_amount_of_people {
        buffers.push(None);
    }

    // ===================== Setup camera ======================

    let ctx = uvc::Context::new().expect("Could not get context");
    let dev = ctx
        // .find_device(Some(0x0bda), Some(0x5652), None)
        .find_device(None, None, None)
        .expect("Could not find device");

    let description = dev.description().unwrap();
    println!(
        "Found device: Bus {:03} Device {:03} : ID {:04x}:{:04x} {} ({})",
        dev.bus_number(),
        dev.device_address(),
        description.vendor_id,
        description.product_id,
        description.product.unwrap_or_else(|| "Unknown".to_owned()),
        description
        .manufacturer
        .unwrap_or_else(|| "Unknown".to_owned())
    );

    let devh = dev.open().expect("Could not open device");
    let format = uvc::StreamFormat {
        width: 1280,
        height: 720,
        fps: 30,
        format: uvc::FrameFormat::MJPEG,
    };
    let mut streamh = devh
        .get_stream_handle_with_format(format)
        .expect("Could not open a stream with this format");

    let my_frame = Arc::new(RwLock::new(None));
    
    let my_frame_clone = my_frame.clone();
    let _stream = streamh
        .start_stream(move |frame: &Frame, data: &mut Arc<Mutex<Option<glium::texture::RawImage2d<u8>>>>| {
            let x = frame_to_raw_image(frame).expect("Failed to unwrap frame_to_raw_image()");
            let cloned_image = to_my_frame(&x);
            {
                let mut data = Mutex::lock(&data).unwrap();
                *data = Some(x);
            }

            // XXX: a new frame is ready, time to send to others here!!!
            *my_frame_clone.write().unwrap() = Some(cloned_image);
        }, frames[0].clone()) // the first (index 0) frame is always present and represents self
        .unwrap();

    // ===================== Network start ======================

    let my_frame_clone = my_frame.clone();
    let temp_clone = frames[1].clone();
    let amount_clone = amount_of_people.clone();
    thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1500));
        if let Some(frame) = &*my_frame_clone.read().unwrap() {
            *Mutex::lock(&temp_clone).unwrap() = Some(from_my_frame(frame.clone()));
            *amount_clone.write().unwrap() += 1;
        }
    });

    // ===================== Start graphics loop ======================

    events_loop.run(move |event, _, control_flow| {
        if let glutin::event::Event::WindowEvent { event, .. } = event {
            if let glutin::event::WindowEvent::CloseRequested = event {
                *control_flow = glutin::event_loop::ControlFlow::Exit;
                return;
            }
        }

        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 1.0, 1.0);

        let amount : usize;
        {
            amount = *amount_of_people.read().unwrap();
        }

        for i in 0..amount {
            match Mutex::lock(&frames[i]).unwrap().take() {
                None => {}
                Some(image) => {
                    let image = glium::texture::SrgbTexture2d::new(&display, image)
                        .expect("Could not use image");
                    buffers[i] = Some(image);
                }
            }
        }

        for i in (0..amount).rev() { // reverse to make [0] (self) render last
            if let Some(ref b) = buffers[i] {
                let uniforms = uniform! { u_image: b };
                target.draw(
                    &vertex_layout[amount - 1][i], // layout indexing starts from 0
                    &indices,
                    &program,
                    &uniforms,
                    &Default::default(),
                ).unwrap();
            }
        }

        target.finish().unwrap();

        std::thread::sleep(Duration::from_millis(30));
    });
}



#[derive(Copy, Clone)]
pub struct QuadVertex {
    pos: (f32, f32),
    uv: (f32, f32),
}


fn frame_to_raw_image(
    frame: &Frame,
) -> Result<glium::texture::RawImage2d<'static, u8>, Box<dyn Error>> {
    let new_frame = frame.to_rgb()?;
    let data = new_frame.to_bytes();
    // println!("{} {} {}", data.len(), frame.width(), frame.height());

    let image = glium::texture::RawImage2d::from_raw_rgb(
        data.to_vec(),
        (new_frame.width(), new_frame.height()),
    );

    Ok(image)
}

// fn callback_frame_to_image(
//     frame: &Frame,
//     data: &mut Arc<Mutex<Option<glium::texture::RawImage2d<u8>>>>,
// ) {
//     let image = frame_to_raw_image(frame);
//     match image {
//         Err(x) => println!("{:#?}", x),
//         Ok(x) => {
//             let mut data = Mutex::lock(&data).unwrap();
//             *data = Some(x);
//         }
//     }
// }

#[derive(Clone)]
struct MyFrame {
    data: Vec<u8>,
    width: u16,
    height: u16,
}

// fn to_my_frame(image: &Arc<Mutex<Option<glium::texture::RawImage2d<u8>>>>) -> Option<MyFrame> {
//     if let Some(image) = &*Mutex::lock(&image).unwrap() {
//         Some(MyFrame{ data: image.data.to_vec(), width: image.width as u16, height: image.height as u16 })
//     } else {
//         None
//     }
// }
fn to_my_frame(image: &glium::texture::RawImage2d<u8>) -> MyFrame {
    MyFrame{ data: image.data.to_vec(), width: image.width as u16, height: image.height as u16 }
}

fn from_my_frame(MyFrame{ data, width, height }: MyFrame) -> glium::texture::RawImage2d<'static, u8> {
// fn from_my_frame<'a>(my_frame: 'a MyFrame) -> glium::texture::RawImage2d<'a, u8> {
    glium::texture::RawImage2d::from_raw_rgb(data, (width as u32, height as u32))
    // glium::texture::RawImage2d::from_raw_rgb(my_frame.data, (my_frame.width as u32, my_frame.height as u32))
}

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

// TODO: change Display type
fn generate_vertex_layout(mut display: glium::Display) -> (Vec<Vec<glium::VertexBuffer<QuadVertex>>>, glium::backend::glutin::Display) {
    let mut v = Vec::new();
    for i in 0..3 {
        let (vertices, display2) = generate_vertices(i, display);
        display = display2;
        v.push(vertices);
    }
    (v, display)
}

// TODO: can optimize
fn generate_vertices(amount_without_self: u8, display: glium::Display) -> (Vec<glium::VertexBuffer<QuadVertex>>, glium::backend::glutin::Display) {
    if amount_without_self > 3 {
        panic!("Too many people!!!");
    }

    let mut all = Vec::new();

    let small : glium::VertexBuffer<QuadVertex> = if amount_without_self == 0 {
        let vertices_full: [QuadVertex; 4] = [
            QuadVertex { pos: (-1.0, -1.0), uv: (1.0, 1.0) },
            QuadVertex { pos: (-1.0, 1.0), uv: (1.0, 0.0) },
            QuadVertex { pos: (1.0, -1.0), uv: (0.0, 1.0) },
            QuadVertex { pos: (1.0, 1.0), uv: (0.0, 0.0) },
        ];
        glium::VertexBuffer::new(&display, &vertices_full).unwrap()
    } else if amount_without_self == 2 || amount_without_self == 3 {
        let vertices: [QuadVertex; 4] = [
            QuadVertex { pos: (0.0, -1.0), uv: (1.0, 1.0) },
            QuadVertex { pos: (0.0, 0.0), uv: (1.0, 0.0) },
            QuadVertex { pos: (1.0, -1.0), uv: (0.0, 1.0) },
            QuadVertex { pos: (1.0, 0.0), uv: (0.0, 0.0) },
        ];
        glium::VertexBuffer::new(&display, &vertices).unwrap()
    } else { // == 1 and other
        let vertices_small: [QuadVertex; 4] = [
            QuadVertex { pos: (0.6, -0.9), uv: (1.0, 1.0) },
            QuadVertex { pos: (0.6, -0.6), uv: (1.0, 0.0) },
            QuadVertex { pos: (0.9, -0.9), uv: (0.0, 1.0) },
            QuadVertex { pos: (0.9, -0.6), uv: (0.0, 0.0) },
        ];
        glium::VertexBuffer::new(&display, &vertices_small).unwrap()
    };

    all.push(small);

    if amount_without_self == 1 {
        let vertices_full: [QuadVertex; 4] = [
            QuadVertex { pos: (-1.0, -1.0), uv: (1.0, 1.0) },
            QuadVertex { pos: (-1.0, 1.0), uv: (1.0, 0.0) },
            QuadVertex { pos: (1.0, -1.0), uv: (0.0, 1.0) },
            QuadVertex { pos: (1.0, 1.0), uv: (0.0, 0.0) },
        ];
        all.push(glium::VertexBuffer::new(&display, &vertices_full).unwrap());
    } else if amount_without_self == 2 {
        let vertices_1: [QuadVertex; 4] = [
            QuadVertex { pos: (-1.0, -1.0), uv: (1.0, 1.0) },
            QuadVertex { pos: (-1.0, 0.0), uv: (1.0, 0.0) },
            QuadVertex { pos: (0.0, -1.0), uv: (0.0, 1.0) },
            QuadVertex { pos: (0.0, 0.0), uv: (0.0, 0.0) },
        ];
        let vertices_2: [QuadVertex; 4] = [
            QuadVertex { pos: (-1.0, 0.0), uv: (1.0, 1.0) },
            QuadVertex { pos: (-1.0, 1.0), uv: (1.0, 0.0) },
            QuadVertex { pos: (0.0, 0.0), uv: (0.0, 1.0) },
            QuadVertex { pos: (0.0, 1.0), uv: (0.0, 0.0) },
        ];
        all.push(glium::VertexBuffer::new(&display, &vertices_1).unwrap());
        all.push(glium::VertexBuffer::new(&display, &vertices_2).unwrap());
    } else if amount_without_self == 3 {
        let vertices_1: [QuadVertex; 4] = [
            QuadVertex { pos: (-1.0, -1.0), uv: (1.0, 1.0) },
            QuadVertex { pos: (-1.0, 0.0), uv: (1.0, 0.0) },
            QuadVertex { pos: (0.0, -1.0), uv: (0.0, 1.0) },
            QuadVertex { pos: (0.0, 0.0), uv: (0.0, 0.0) },
        ];
        let vertices_2: [QuadVertex; 4] = [
            QuadVertex { pos: (-1.0, 0.0), uv: (1.0, 1.0) },
            QuadVertex { pos: (-1.0, 1.0), uv: (1.0, 0.0) },
            QuadVertex { pos: (0.0, 0.0), uv: (0.0, 1.0) },
            QuadVertex { pos: (0.0, 1.0), uv: (0.0, 0.0) },
        ];
        let vertices_3: [QuadVertex; 4] = [
            QuadVertex { pos: (0.0, 0.0), uv: (1.0, 1.0) },
            QuadVertex { pos: (0.0, 1.0), uv: (1.0, 0.0) },
            QuadVertex { pos: (1.0, 0.0), uv: (0.0, 1.0) },
            QuadVertex { pos: (1.0, 1.0), uv: (0.0, 0.0) },
        ];
        all.push(glium::VertexBuffer::new(&display, &vertices_1).unwrap());
        all.push(glium::VertexBuffer::new(&display, &vertices_2).unwrap());
        all.push(glium::VertexBuffer::new(&display, &vertices_3).unwrap());
    }

    (all, display)
}

// struct Vertices {
//     small: glium::VertexBuffer<QuadVertex>,
//     others: Vec<glium::VertexBuffer<QuadVertex>>,
// }
// ============================================================================
// ============================================================================
// ============================================================================
fn as_server(serving_address: &str) {
    println!(":> Shall start as server at {}!", serving_address);
    let listener = TcpListener::bind(serving_address).expect("::> Failed to start server");

    let mut targets_expecting_packets: Vec<Sender<Consumer>> = Vec::new();
    let mut all_consumers: Vec<Consumer> = Vec::new();

    // Each new incoming connection(Client) will get processed here
    for stream in listener.incoming() {
        let mut stream = stream.expect("::> Failed to unwrap stream");
        println!(":> Registering a new client with addr={:?}", stream.peer_addr());

        // The channle through which the new client will receive Packets
        let (consumer, mailbox): (Sender<Packet>, Receiver<Packet>) = channel();
        // The channel through which the new client will receive new clients to send Packets to
        let (consumer_sender, consumer_receiver): (Sender<Consumer>, Receiver<Consumer>) = channel();

        // Notify existing clients that now they also need to send stuff to this new client
        targets_expecting_packets.retain(|client| { // HACK to be able to remove while iterating
            !client.send(consumer.clone())
                .map_err(|_| {
                    println!(":> Won't notify an existing client as it is dead now, shall remove it from the Vec");
                })
                .is_err()
        });
        // Future new clients will also notify this new client
        targets_expecting_packets.push(consumer_sender);

        // Construct a Vec containing all existing clients for the new client to send Packets to
        let mut consumers = all_consumers.clone();
        all_consumers.push(consumer);

        let (wanna_quit_tx, wanna_quit_rx) = channel();

        // This thread receives(blocking) new Packet from self, then distributes among consumers
        let stream_clone = stream.try_clone().expect("::> Failed to clone stream");
        thread::spawn(move || {
            let mut stream = stream_clone;
            loop {
                // Receive a new packet
                if let Ok(packet) = read_packet(&mut stream) {
                    println!(":> [Info] Server received packet from {} with order {}", packet.from_id, packet.order);

                    // Register a new consumer to send to?
                    if let Ok(consumer) = consumer_receiver.try_recv() {
                        consumers.push(consumer);
                        println!(":> Server registers a new consumer for client {}", packet.from_id);
                    }

                    // Send to all consumers
                    consumers.retain(|consumer| { // HACK to be able to remove while iterating
                        // TODO: optimize last packet move
                        // Retain if no error while sending (the channel is alive)
                        !consumer.send(packet.clone())
                            .map(|_| {
                                // println!(":> [Info] Server sends packet from {} with order {} to someone", packet.from_id, packet.order);
                            })
                            .map_err(|_| {
                                println!(":> No longer have some Client, shall remove it from the Vec");
                            })
                            .is_err()
                    });
                } else {
                    println!(":> Failed to read packet from Client, it must have hung up. Shall end this Client");
                    wanna_quit_tx.send(()).unwrap();
                    return; // finish thread
                }
            }
        });

        // This thread collects(blocking) all Packets for self, then sends them to self Client
        thread::spawn(move || {
            loop {
                // Need timeout to be able not to rely on other clients to
                // send us data and thus unblock us so that we can check if we wanna_quit.
                let packet = mailbox.recv_timeout(Duration::from_millis(3000)).ok();

                // If the client has hung up(dead connection) => finish
                if wanna_quit_rx.try_recv().is_ok() {
                    println!(":> Received wanna_quit for a Client, shall quit");
                    return; // finish thread
                }

                // Otherwise send the packet
                if let Some(packet) = packet {
                    // println!(":> [Info] Server writes packet from {} with order {} to someone", packet.from_id, packet.order);
                    // We get notified about connection failure from the other thread => can ignore here
                    let _ = write_packet(packet, &mut stream);
                }
            }
        });
    }
}
// ============================================================================
// ============================================================================
// ============================================================================
fn encode(raw: &Vec<u8>) -> Vec<u8> {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::fast());
    e.write_all(&raw).unwrap();
    e.finish().unwrap()
}

fn decode(compressed: &Vec<u8>) -> Vec<u8> {
    let mut result = Vec::new();
    let mut deflater = DeflateDecoder::new(&compressed[..]);
    deflater.read_to_end(&mut result).unwrap();
    result
}
// ============================================================================
// ============================================================================
// ============================================================================
