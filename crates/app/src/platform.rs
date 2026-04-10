/// Platform-specific initialization (dock icon, etc.)

#[cfg(target_os = "macos")]
static ICON_PNG: &[u8] = include_bytes!("../../../assets/icon-256.png");

#[cfg(target_os = "macos")]
pub fn set_dock_icon() {
    use cocoa::appkit::NSApp;
    use cocoa::base::nil;
    use cocoa::foundation::NSData;
    use objc::runtime::Object;
    use objc::*;

    unsafe {
        let data = NSData::dataWithBytes_length_(
            nil,
            ICON_PNG.as_ptr() as *const std::os::raw::c_void,
            ICON_PNG.len() as u64,
        );
        let icon: *mut Object = msg_send![class!(NSImage), alloc];
        let icon: *mut Object = msg_send![icon, initWithData: data];
        if !icon.is_null() {
            let app = NSApp();
            let _: () = msg_send![app, setApplicationIconImage: icon];
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_dock_icon() {
    // Windows/Linux: icons handled via bundle manifests / .desktop files
}
