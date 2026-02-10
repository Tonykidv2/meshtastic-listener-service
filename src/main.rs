// This example connects to a radio via serial and prints out all received packets.
// This example requires a powered and flashed Meshtastic radio.

use std::io::{self, BufRead};

use meshtastic::api::StreamApi;
use meshtastic::utils;
use meshtastic::protobufs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_api = StreamApi::new();

    let available_ports = utils::stream::available_serial_ports()?;
    println!("Available ports: {:?}", available_ports);
    println!("Enter the name of a port to connect to:");

    let stdin = io::stdin();
    let entered_port = stdin
        .lock()
        .lines()
        .next()
        .expect("Failed to find next line")
        .expect("Could not read next line");

    let serial_stream = utils::stream::build_serial_stream(entered_port, None, None, None)?;
    let (mut decoded_listener, stream_api) = stream_api.connect(serial_stream).await;

    let config_id = utils::generate_rand_id();
    let stream_api = stream_api.configure(config_id).await?;

    println!("\n📡 Listening for messages... (Type 'quit' or 'exit' to stop)");

    // Spawn a task to read user input in a blocking manner
    let mut input_handle = tokio::spawn(async {
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();
        lines.next()
    });

    // Listen for messages and user input
    loop {
        tokio::select! {
            // Wait for received messages
            Some(decoded) = decoded_listener.recv() => {
                println!("\n=== Received FromRadio ===");
                println!("ID: {}", decoded.id);
                
                // Match on the payload_variant to get the actual packet
                if let Some(payload_variant) = &decoded.payload_variant {
                    use meshtastic::protobufs::from_radio::PayloadVariant;
                    
                    match payload_variant {
                        PayloadVariant::Packet(packet) => {
                            println!("From: {}", packet.from);
                            println!("To: {}", packet.to);
                            println!("Packet ID: {}", packet.id);
                            
                            // MeshPacket has its own payload_variant field
                            if let Some(packet_payload) = &packet.payload_variant {
                                use meshtastic::protobufs::mesh_packet::PayloadVariant as PacketPayload;
                                
                                match packet_payload {
                                    PacketPayload::Decoded(data) => {
                                        println!("Port Number: {}", data.portnum);
                                        
                                        // Check if it's a text message
                                        if data.portnum == protobufs::PortNum::TextMessageApp as i32 {
                                            if let Ok(text) = String::from_utf8(data.payload.clone()) {
                                                println!("📨 TEXT MESSAGE: {}", text);
                                            }
                                        } else {
                                            // For other message types
                                            match data.portnum {
                                                x if x == protobufs::PortNum::PositionApp as i32 => {
                                                    println!("📍 Position update received");
                                                }
                                                x if x == protobufs::PortNum::NodeinfoApp as i32 => {
                                                    println!("ℹ️  Node info received");
                                                }
                                                x if x == protobufs::PortNum::TelemetryApp as i32 => {
                                                    println!("📊 Telemetry data received");
                                                }
                                                _ => {
                                                    println!("Other message type (port: {})", data.portnum);
                                                }
                                            }
                                        }
                                    }
                                    PacketPayload::Encrypted(encrypted) => {
                                        println!("🔒 Encrypted payload ({} bytes)", encrypted.len());
                                    }
                                }
                            } else {
                                println!("Packet has no payload");
                            }
                        }
                        PayloadVariant::MyInfo(info) => {
                            println!("My Info - Node: {}", info.my_node_num);
                        }
                        PayloadVariant::NodeInfo(_node) => {
                            println!("Node Info received");
                        }
                        PayloadVariant::ConfigCompleteId(id) => {
                            println!("Config complete: {}", id);
                        }
                        _ => {
                            println!("Other payload type: {:?}", payload_variant);
                        }
                    }
                }
            }
            
            // Wait for user input
            result = &mut input_handle => {
                if let Ok(Some(Ok(input))) = result {
                    let input_lower = input.to_lowercase();
                    if input_lower == "quit" || input_lower == "exit" {
                        println!("\n👋 Disconnecting...");
                        break;
                    }
                } else {
                    break;
                }
            }
        }
    }

    let _stream_api = stream_api.disconnect().await?;

    Ok(())
}