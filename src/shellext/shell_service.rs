// Module that initializes and runs the shell services in a separate thread
// Similar to ShellServices::InitAndStartServiceTask in the C++ implementation

use crate::shellext::context_menu::ShellServices;
use crate::drive::manager::DriveManager;
use std::sync::{Arc, mpsc};
use std::thread;

pub fn init_and_start_service_task(drive_manager: Arc<DriveManager>) -> ServiceHandle {
    let (tx, rx) = mpsc::channel();

    let mut services = ShellServices::new(drive_manager);
    let handle = thread::spawn(move || {
        match services.init_and_start() {
            Ok(_) => {
                // Notify that initialization is complete
                let _ = tx.send(Ok(()));
                
                // Run the message loop
                if let Err(e) = services.run_message_loop() {
                    tracing::error!(target: "shellext::shell_service", "Error in message loop: {:?}", e);
                }
            }
            Err(e) => {
                tracing::error!(target: "shellext::shell_service", "Failed to initialize shell services: {:?}", e);
                let _ = tx.send(Err(e));
            }
        }
    });

    ServiceHandle {
        thread: Some(handle),
        init_result: rx,
    }
}

pub struct ServiceHandle {
    thread: Option<thread::JoinHandle<()>>,
    init_result: mpsc::Receiver<windows::core::Result<()>>,
}

impl ServiceHandle {
    pub fn wait_for_init(&mut self) -> windows::core::Result<()> {
        self.init_result.recv().map_err(|_| {
            windows::core::Error::from(windows::Win32::Foundation::E_FAIL)
        })?
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        tracing::info!(target: "shellext::shell_service", "Shutting down shell services thread...");
    }
}

