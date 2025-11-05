// Context menu handler for Windows Explorer
// This implements a COM object that provides a custom context menu item

use std::sync::Arc;

use windows::{
    core::*,
    Win32::{
        Foundation::*,
        System::{
            Threading::CreateEventW,
            Com::*
        },
        UI::Shell::*,
        
    },
};

use crate::drive::manager::DriveManager;

// UUID for our context menu handler - matches the C++ implementation
const CLSID_TEST_EXPLORER_COMMAND: GUID = GUID::from_u128(0x165cd069_d9c8_42b4_8e37_b6971afa4494);

#[implement(IExplorerCommand)]
pub struct TestExplorerCommandHandler {
    drive_manager: Arc<DriveManager>,
    site: std::sync::Mutex<Option<IUnknown>>,
}

impl TestExplorerCommandHandler {
    pub fn new(drive_manager: Arc<DriveManager>) -> Self {
        Self {
            drive_manager,
            site: std::sync::Mutex::new(None),
        }
    }
}

impl IExplorerCommand_Impl for TestExplorerCommandHandler_Impl {
    fn GetTitle(&self, _items: Option<&IShellItemArray>) -> Result<PWSTR> {
        let title = w!("SFTP Test Command");
        unsafe { SHStrDupW(title) }
    }

    fn GetIcon(&self, _items: Option<&IShellItemArray>) -> Result<PWSTR> {
        Err(Error::from(E_NOTIMPL))
    }

    fn GetToolTip(&self, _items: Option<&IShellItemArray>) -> Result<PWSTR> {
        Err(Error::from(E_NOTIMPL))
    }

    fn GetCanonicalName(&self) -> Result<GUID> {
        Err(Error::from(E_NOTIMPL))
    }

    fn GetState(
        &self,
        _items: Option<&IShellItemArray>,
        _oktobeslow: BOOL,
    ) -> Result<u32> {
        Ok(ECS_ENABLED.0 as u32)
    }

    fn Invoke(
        &self,
        selection: Option<&IShellItemArray>,
        _bindctx: Option<&IBindCtx>,
    ) -> Result<()> {
        println!("=================================================");
        println!("SFTP Context Menu Command Invoked!");
        println!("=================================================");

        // Call async method from sync context using futures executor
        let drives = futures::executor::block_on(self.drive_manager.list_drives());
        println!("Found {} drive(s)", drives.len());
        for drive in &drives {
            println!("  - Drive: {}", drive.name);
        }

        if let Some(items) = selection {
            unsafe {
                let count = items.GetCount()?;
                println!("Selected {} item(s)", count);

                for i in 0..count {
                    let item = items.GetItemAt(i)?;
                    let display_name = item.GetDisplayName(SIGDN_FILESYSPATH)?;
                    let path = display_name.to_string()?;
                    println!("  [{}] {}", i + 1, path);
                }
            }
        } else {
            println!("No items selected");
        }

        println!("=================================================");
        Ok(())
    }

    fn GetFlags(&self) -> Result<u32> {
        Ok(ECF_DEFAULT.0 as u32)
    }

    fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
        Err(Error::from(E_NOTIMPL))
    }
}

// Class factory for creating instances of our context menu handler
#[implement(IClassFactory)]
pub struct TestExplorerCommandFactory{
    drive_manager: Arc<DriveManager>,
}

impl TestExplorerCommandFactory {
    pub fn new(drive_manager: Arc<DriveManager>) -> Self {
        Self {
            drive_manager,
        }
    }
}

impl IClassFactory_Impl for TestExplorerCommandFactory_Impl {
    fn CreateInstance(
        &self,
        outer: Option<&IUnknown>,
        iid: *const GUID,
        result: *mut *mut core::ffi::c_void,
    ) -> Result<()> {
        if outer.is_some() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }

        let handler = TestExplorerCommandHandler::new(self.drive_manager.clone());
        let handler: IUnknown = handler.into();
        
        unsafe {
            handler.query(iid, result).ok()
        }
        
    }

    fn LockServer(&self, _lock: BOOL) -> Result<()> {
        Ok(())
    }
}

// Shell services - registers COM objects for Windows Shell integration
pub struct ShellServices {
    cookies: Vec<u32>,
    drive_manager: Arc<DriveManager>,
}

impl ShellServices {
    pub fn new(drive_manager: Arc<DriveManager>) -> Self {
        Self {
            cookies: Vec::new(),
            drive_manager,
        }
    }

    pub fn init_and_start(&mut self) -> Result<()> {
        tracing::info!(target: "shellext::context_menu", "Initializing Shell Services (Context Menu Handler)...");

        unsafe {
            // Initialize COM for this thread
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            // Create and register the class factory
            let factory: IClassFactory = TestExplorerCommandFactory::new(self.drive_manager.clone()).into();
            
            let cookie = CoRegisterClassObject(
                &CLSID_TEST_EXPLORER_COMMAND,
                &factory,
                CLSCTX_LOCAL_SERVER,
                REGCLS_MULTIPLEUSE,
            )?;

            self.cookies.push(cookie);
            tracing::info!(target: "shellext::context_menu", "Context Menu Handler registered with cookie: {}", cookie);
            println!("CLSID: {{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}", 
                CLSID_TEST_EXPLORER_COMMAND.data1,
                CLSID_TEST_EXPLORER_COMMAND.data2,
                CLSID_TEST_EXPLORER_COMMAND.data3,
                CLSID_TEST_EXPLORER_COMMAND.data4[0],
                CLSID_TEST_EXPLORER_COMMAND.data4[1],
                CLSID_TEST_EXPLORER_COMMAND.data4[2],
                CLSID_TEST_EXPLORER_COMMAND.data4[3],
                CLSID_TEST_EXPLORER_COMMAND.data4[4],
                CLSID_TEST_EXPLORER_COMMAND.data4[5],
                CLSID_TEST_EXPLORER_COMMAND.data4[6],
                CLSID_TEST_EXPLORER_COMMAND.data4[7],
            );
        }

        Ok(())
    }

    pub fn run_message_loop(&self) -> Result<()> {
        tracing::info!(target: "shellext::context_menu", "Context Menu Handler is running. Press Ctrl+C to exit...");
        
        // Keep the thread alive to handle COM requests
        // In the C++ version, they use CoWaitForMultipleHandles
        // We'll use a simple approach - create a dummy event handle
        unsafe {
            // Use INVALID_HANDLE_VALUE as a dummy handle for CoWaitForMultipleHandles
            // This keeps the COM message pump running
            let dymmyevent = CreateEventW(None, FALSE, FALSE, None)?;
            let index = CoWaitForMultipleHandles(
                (COWAIT_DISPATCH_CALLS).0 as u32,
                u32::MAX,
                &[dymmyevent],
            );
            tracing::info!(target: "shellext::context_menu", "CoWaitForMultipleHandles index: {:?}", index);
        }

        Ok(())
    }
}

impl Drop for ShellServices {
    fn drop(&mut self) {
        tracing::info!(target: "shellext::context_menu", "Unregistering Shell Services...");
        unsafe {
            for cookie in &self.cookies {
                let _ = CoRevokeClassObject(*cookie);
            }
            CoUninitialize();
        }
    }
}

