use std::io::prelude::*;
use std::net::{TcpStream, TcpListener};

use std::env;

use std::sync::mpsc::{channel, Sender, Receiver};
// use std::sync::{Mutex, Arc};
use std::{thread, time};
use std::time::Duration;

// ============================================================================
// ============================================================================
// ============================================================================
fn as_client(id: u8) {
    println!(":> Shall start as client!");

    let mut stream = TcpStream::connect("127.0.0.1:1234").expect("Unable to bind on Server");
    let stream_clone = stream.try_clone().expect("Failed to clone");

    let (exit_tx, exit_rx) = channel();

    thread::spawn(move || {
        let mut stream = stream_clone;
        loop {
            if let Ok(packet) = read_packet(&mut stream) {
                println!("Client {} received packet from {} with order {}", id, packet.from_id, packet.order);
            } else {
                println!("The server stopped!!! Exiting...");
                exit_tx.send(()).expect("Failed to send with exit_tx");
                return;
            }
        }
    });

    let mut order = id as u32 * 1000;
    loop {
        if let Ok(_) = exit_rx.try_recv() {
            println!("Finishing client {}...", id);
            return;
        }

        let packet = spawn_packet(id, order);
        println!("Client {} sent packet with order {}", id, packet.order);
        write_packet(packet, &mut stream);
        order += 1;

        thread::sleep(time::Duration::from_millis(1500));
    }
}


fn as_server() {
    println!(":> Shall start as server!");

    let mut clients: Vec<Sender<Sender<Packet>>> = Vec::new();
    let mut global_targets: Vec<Sender<Packet>> = Vec::new();

    let listener = TcpListener::bind("127.0.0.1:1234").expect("Unable to bind on Server");
    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        println!("Got a client with addr={:?}", stream.peer_addr());

        // The channle through which the new client will receive Packets
        let (sender, receiver): (Sender<Packet>, Receiver<Packet>) = channel();
        // The channel through which the new client will receive new clients to send Packets to
        let (sender_sender, receiver_receiver): (Sender<Sender<Packet>>, Receiver<Sender<Packet>>) = channel();


        // Notify existing clients that now they also need to send stuff to this new client
        clients.retain(|client| { // HACK to be able to remove while iterating
            !client.send(sender.clone())
                .map_err(|_| {
                    println!("Won't notify an existing client as it is dead now, shall remove it from the Vec");
                })
                .is_err()
        });
        // Future new clients will also notify this new client
        clients.push(sender_sender);

        // Construct a Vec containing all existing clients for the new client to send Packets to
        let mut targets = global_targets.clone();
        global_targets.push(sender);

        let (wanna_quit_tx, wanna_quit_rx) = channel();

        // Receives(blocks) new Packet from self, then distributes among others
        let mut stream_clone = stream.try_clone().expect("Failed to clone stream");
        thread::spawn(move || {
            loop {
                // Receive a new packet
                if let Ok(packet) = read_packet(&mut stream_clone) {
                    println!("Server received packet from {} with order {}", packet.from_id, packet.order);

                    // Register a new client to send to
                    if let Ok(sender) = receiver_receiver.try_recv() {
                        targets.push(sender);
                        println!("Server registers a new sender for client {}", packet.from_id);
                    }

                    // Send to all clients
                    targets.retain(|target| { // HACK to be able to remove while iterating
                        // TODO: optimize last packet move
                        // Retain if no error while sending (the channel is alive)
                        !target.send(packet.clone())
                            .map(|_| {
                                println!("Server sends packet from {} with order {} to someone", packet.from_id, packet.order);
                            })
                            .map_err(|_| {
                                println!("No longer have some Client, shall remove it from the Vec");
                            })
                            .is_err()
                    });
                } else {
                    println!("Failed to read packet from Client, it must have hung up. Shall end this Client");
                    wanna_quit_tx.send(()).unwrap();
                    return; // finish thread
                }
            }
        });

        // Collects(blocks) all Packets for self and sends them to self
        thread::spawn(move || {
            let timeout = Duration::from_millis(3000);
            loop {
                // Need timeout to be able not to rely on other clients to
                // send us data and thus unblock us so that we can check if we wanna_quit.
                if let Ok(packet) = receiver.recv_timeout(timeout) {
                    // TODO: right now we most likely send the final Packet to a dead connection
                    println!("Server writes packet from {} with order {} to someone", packet.from_id, packet.order);
                    write_packet(packet, &mut stream);
                } else {
                    println!("Timeout reached for receiver.recv_timeout(). Probably we are alone");
                }

                if let Ok(_) = wanna_quit_rx.try_recv() {
                    println!("Received wanna_quit, shall quit");
                    return; // finish thread
                }
            }
        });
    }
}
// ============================================================================
// ============================================================================
// ============================================================================
#[derive(Clone)]
struct Packet {
    from_id: u8,
    order: u32,
    width: u16,
    height: u16,
    compressed_length: u16, // is equal to compressed_video.len()
    compressed_video: Vec<u8>,
}
// ============================================================================
// ============================================================================
// ============================================================================
fn read_packet(stream: &mut TcpStream) -> Result<Packet, std::io::Error> {
    let mut buff = [0_u8; 11]; // Packet header size in bytes
    // stream.read_exact(&mut buff).expect("Unable to read_exact()");
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
    // stream.read_exact(&mut compressed_video[..]).expect("Unable to read_exact()");
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

fn write_packet(packet: Packet, stream: &mut TcpStream) {
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



    stream.write_all(&buff[..]).expect("Unable to write_all()");
}

fn spawn_packet(from_id: u8, order: u32) -> Packet {
    let compressed_length: u16 = 8 * 1024;
    let mut compressed_video = Vec::with_capacity(compressed_length as usize);
    for i in 0..compressed_length {
        compressed_video.push(i as u8);
    }

    Packet {
        from_id,
        order,
        width: 1920,
        height: 1080,
        compressed_length,
        compressed_video,
    }
}
// ============================================================================
// ============================================================================
// ============================================================================
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!(":> Please specify either 'client' or 'server'");
        return;
    } else {
        if args[1] == "client" {
            as_client(args[2].parse().unwrap());
        } else if args[1] == "server" {
            as_server();
        } else {
            println!(":> Unknown option...");
        }
    }
}
