# Multi-Camera Streaming with RaptorQ FEC

A Rust application for streaming multiple camera feeds over RTP with RaptorQ Forward Error Correction using GStreamer.

## Features

- **4 simultaneous camera streams**
- **RaptorQ FEC** for packet loss recovery
- **Configurable IP addresses and ports**
- **Real-time statistics** display
- **Windows GUI** using native-windows-gui

## Prerequisites

- **Rust** (latest stable)
- **GStreamer** with development libraries
- **Windows** (currently Windows-only due to GUI framework)

## Project Structure
├── sender/          # Multi-camera sender application
│   ├── src/
│   │   └── backend.rs  # Sender backend connections
│   │   └── gstreamer.rs  # Sender streaming logic 
│   │   └── main.rs  # Sender Main function
│   │   └── types.rs  # Sender Types 
│   │   └── ui.rs  # Sender GUI
│   └── Cargo.toml
├── receiver/        # Multi-camera receiver application
│   ├── src/
│   │   └── backend.rs  # Receiver backend connections
│   │   └── gstreamer.rs  # Receiver streaming logic 
│   │   └── main.rs  # Receiver Main function
│   │   └── stats_collector.rs  # Receiver statistics collector 
│   │   └── types.rs  # Receiver Types 
│   │   └── ui.rs  # Receiver GUI
│   └── Cargo.toml
└── README.md

## Building

```bash
# Build sender
cd sender
cargo build --release

# Build receiver  
cd ../receiver
cargo build --release
Usage

Start the receiver first to listen on the configured ports
Configure IP addresses and ports as needed
Start the sender and begin streaming
Click "Stats" on receiver to view RaptorQ statistics

Default Port Configuration

Camera 1: RTP 5000, FEC 5002
Camera 2: RTP 5004, FEC 5006
Camera 3: RTP 5008, FEC 5010
Camera 4: RTP 5012, FEC 5014
