use core_foundation::{
    base::{CFRelease, TCFType},
    boolean::CFBoolean,
    dictionary::CFMutableDictionary,
    string::CFString,
};

/// The key used to trigger the system accessibility permission prompt.
/// Defined in ApplicationServices/HIServices/AXUIElement.h
const AX_TRUSTED_CHECK_OPTION_PROMPT: &str = "AXTrustedCheckOptionPrompt";

extern "C" {
    /// Returns true if the current process is a trusted accessibility client.
    /// If `options` contains kAXTrustedCheckOptionPrompt=true and the process
    /// is NOT trusted, macOS shows the System Preferences prompt automatically
    /// and registers the *current* binary in the TCC database.
    fn AXIsProcessTrustedWithOptions(options: core_foundation::base::CFTypeRef) -> bool;
}

/// Check whether we have accessibility permission.
/// If `prompt` is true and we don't, macOS will open the System Settings
/// dialog for the user and register the current binary hash in TCC.
pub fn is_accessibility_trusted(prompt: bool) -> bool {
    unsafe {
        let mut dict: CFMutableDictionary<CFString, CFBoolean> = CFMutableDictionary::new();
        let key = CFString::new(AX_TRUSTED_CHECK_OPTION_PROMPT);
        let value = if prompt {
            CFBoolean::true_value()
        } else {
            CFBoolean::false_value()
        };
        dict.add(&key, &value);

        let trusted = AXIsProcessTrustedWithOptions(dict.as_CFTypeRef());

        // Explicitly release the dictionary to avoid leaks
        CFRelease(dict.as_CFTypeRef());
        std::mem::forget(dict); // prevent double-free from Drop

        trusted
    }
}
