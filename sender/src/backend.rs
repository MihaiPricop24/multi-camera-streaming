use crate::types::CameraInfo;
use std::process::Command;

pub struct CameraBackend;

impl CameraBackend {
    pub fn detect_cameras() -> Result<Vec<CameraInfo>, String> {
        let mut cameras = Vec::new();

        println!("Starting camera detection...");

        // Method 1: Test GStreamer directly for each device index with different caps
        for i in 0..6 {
            let mut camera_works = false;
            let mut device_name = format!("Camera Device {}", i);

            // Try different test pipelines for virtual cameras vs physical cameras
            let test_commands = vec![
                // Standard test
                format!(
                    "gst-launch-1.0 ksvideosrc device-index={} num-buffers=1 ! videoconvert ! fakesink",
                    i
                ),
                // With explicit caps for virtual cameras
                format!(
                    "gst-launch-1.0 ksvideosrc device-index={} ! video/x-raw,width=640,height=480,framerate=30/1 ! videoconvert ! fakesink",
                    i
                ),
                // Alternative caps
                format!(
                    "gst-launch-1.0 ksvideosrc device-index={} ! video/x-raw,width=1280,height=720,framerate=30/1 ! videoconvert ! fakesink",
                    i
                ),
                // Try with different formats
                format!(
                    "gst-launch-1.0 ksvideosrc device-index={} ! video/x-raw ! videoconvert ! fakesink",
                    i
                ),
            ];

            for (cmd_idx, test_cmd) in test_commands.iter().enumerate() {
                println!(
                    "Testing camera index {} with command {}: {}",
                    i,
                    cmd_idx + 1,
                    test_cmd
                );

                if let Ok(output) = Command::new("cmd").args(&["/C", test_cmd]).output() {
                    let stderr_str = String::from_utf8_lossy(&output.stderr);

                    println!(
                        "Index {} Command {} - Exit code: {}",
                        i,
                        cmd_idx + 1,
                        output.status.code().unwrap_or(-1)
                    );

                    if output.status.success()
                        || stderr_str.contains("Setting pipeline to PAUSED")
                        || stderr_str.contains("PREROLL")
                    {
                        camera_works = true;

                        // Try to get device name from error output
                        if stderr_str.contains("device-name") {
                            for line in stderr_str.lines() {
                                if line.contains("device-name") && line.contains("=") {
                                    if let Some(name_part) = line.split("device-name=").nth(1) {
                                        if let Some(clean_name) = name_part.split(',').next() {
                                            device_name = format!(
                                                "Index {}: {}",
                                                i,
                                                clean_name.trim_matches('"').trim()
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        // Check for specific camera types
                        if stderr_str.contains("Camo") || device_name.contains("Camo") {
                            device_name = format!("Index {}: Camo Virtual Camera (iPhone)", i);
                        } else if stderr_str.contains("Integrated") || stderr_str.contains("USB") {
                            device_name = format!("Index {}: Integrated Camera", i);
                        }

                        println!(
                            "Found working camera: {} (using command {})",
                            device_name,
                            cmd_idx + 1
                        );
                        break;
                    } else {
                        println!("Index {} Command {} failed", i, cmd_idx + 1);
                        if stderr_str.contains("not-negotiated")
                            || stderr_str.contains("non negotiated")
                        {
                            println!("  -> Caps negotiation failed, trying next format...");
                        }
                    }
                } else {
                    println!("Failed to execute test command for index {}", i);
                }
            }

            if camera_works {
                cameras.push(CameraInfo {
                    index: i,
                    name: device_name,
                    device_path: format!("device-index={}", i),
                });
            } else {
                println!("Index {} - No working configuration found", i);
            }
        }

        // Method 2: Use PowerShell to get device names for reference
        if let Ok(output) = Command::new("powershell")
            .args(&[
                "-Command",
                "Get-WmiObject -Class Win32_PnPEntity | Where-Object { $_.PNPClass -eq 'Camera' -or $_.PNPClass -eq 'Image' -or $_.Name -like '*camera*' -or $_.Name -like '*webcam*' -or $_.Name -like '*camo*' } | Select-Object Name, DeviceID | Format-Table -AutoSize"
            ])
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                println!("PowerShell camera enumeration:");
                println!("{}", output_str);
            }
        }

        if cameras.is_empty() {
            println!("No cameras detected through any method");
        } else {
            println!("Total cameras detected: {}", cameras.len());
            for camera in &cameras {
                println!("  - Index {}: {}", camera.index, camera.name);
            }
        }

        Ok(cameras)
    }
}
