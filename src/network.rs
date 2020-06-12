use std::io::prelude::*;
use std::net::{TcpStream, TcpListener};

use std::env;
// use rand::Rng;

use std::sync::mpsc::{channel, Sender, Receiver};
// use std::sync::{Mutex, Arc};
use std::thread;
use std::time::Duration;

// ============================================================================
// ============================================================================
// ============================================================================
fn as_client(id: u8, server_address: &str) {
    println!(":> Shall start as client with id {}!", id);

    let mut stream = TcpStream::connect(server_address).map_err(|_| {
        println!("::> Failed to establish connection to server at {}", server_address);
    }).expect("::> Shall panic");
    let stream_clone = stream.try_clone().unwrap();
    let (must_terminate_tx, must_terminate_rx) = channel();

    // This thread is responsible for receiving Packets and terminating the other
    // thread in case the Server dies
    thread::spawn(move || {
        let mut stream = stream_clone;
        loop {
            if let Ok(packet) = read_packet(&mut stream) {
                println!(":> [Info] Client {} received packet from {} with order {}", id, packet.from_id, packet.order);
            } else {
                println!(":> The server has died! Terminating...");
                must_terminate_tx.send(()).unwrap();
                return; // finish thread
            }
        }
    });

    // This thread is responsible for generating Packets and sending them
    let mut order = id as u32 * 1000;
    loop {
        if must_terminate_rx.try_recv().is_ok() {
            println!(":> Finishing client {}...", id);
            return; // finish thread
        }

        let packet = spawn_packet(id, order);
        println!(":> [Info] Client {} sent packet with order {}", id, packet.order);
        // We get notified about connection failure from the other thread => can ignore here
        let _ = write_packet(packet, &mut stream);
        order += 1;

        thread::sleep(time::Duration::from_millis(1500));
    }
}

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
// ============================================================================
// ============================================================================
// ============================================================================
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
