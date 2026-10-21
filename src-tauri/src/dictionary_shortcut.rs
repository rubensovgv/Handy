//! Explicit, on-demand dictionary capture. Never polls or reads whole text fields.
use crate::settings;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tauri_plugin_store::StoreExt;

static CAPTURING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, PartialEq)]
pub enum SelectionError {
    Empty,
    TooLong,
    Unavailable,
    Permission,
}

fn normalize_selection(text: &str) -> Result<String, SelectionError> {
    // Match CustomWords.tsx, while rejecting paragraphs rather than silently
    // inserting a long dictation into Whisper's vocabulary prompt.
    if text.contains(['\n', '\r']) {
        return Err(SelectionError::TooLong);
    }
    let word = text
        .replace(['<', '>', '"', '\''], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if word.is_empty() {
        return Err(SelectionError::Empty);
    }
    if word.chars().count() > 50 {
        return Err(SelectionError::TooLong);
    }
    Ok(word)
}

fn add_unique(words: &mut Vec<String>, word: String) -> bool {
    if words
        .iter()
        .any(|existing| existing.to_lowercase() == word.to_lowercase())
    {
        return false;
    }
    words.push(word);
    true
}

/// Called on key down. Work happens away from the event-tap thread; repeat
/// presses are coalesced. No synthetic copy and no clipboard reads/writes.
pub fn capture(app: &AppHandle) {
    if CAPTURING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                CAPTURING.store(false, Ordering::SeqCst);
            }
        }
        let _reset = Reset;
        let result = selected_text().and_then(|text| normalize_selection(&text));
        let status = match result {
            Ok(word) => {
                // Read settings only after selection capture has completed.
                // Preserve all other JSON fields, including unknown future ones.
                match app.store(crate::portable::store_path(settings::SETTINGS_STORE_PATH)) {
                    Ok(store) => {
                        let added = (|| {
                            let mut value = store.get("settings").ok_or(())?;
                            let mut words: Vec<String> = serde_json::from_value(
                                value.get("custom_words").cloned().ok_or(())?,
                            )
                            .map_err(|_| ())?;
                            if !add_unique(&mut words, word) {
                                return Ok(false);
                            }
                            value["custom_words"] = serde_json::to_value(words).map_err(|_| ())?;
                            let previous = store.get("settings").ok_or(())?;
                            store.set("settings", value);
                            if store.save().is_err() {
                                store.set("settings", previous);
                                return Err(());
                            }
                            Ok(true)
                        })();
                        match added {
                            Ok(true) => {
                                let _ = app.emit(
                                    "settings-changed",
                                    serde_json::json!({"setting": "custom_words"}),
                                );
                                "dictionaryAdded"
                            }
                            Ok(false) => "dictionaryDuplicate",
                            Err(()) => "dictionarySaveError",
                        }
                    }
                    Err(_) => "dictionarySaveError",
                }
            }
            Err(SelectionError::Empty) => "dictionaryEmpty",
            Err(SelectionError::TooLong) => "dictionaryTooLong",
            Err(SelectionError::Permission) => "dictionaryPermission",
            Err(SelectionError::Unavailable) => "dictionaryUnavailable",
        };
        crate::overlay::show_dictionary_feedback(&app, status);
    });
}

#[cfg(not(target_os = "macos"))]
fn selected_text() -> Result<String, SelectionError> {
    Err(SelectionError::Unavailable)
}

#[cfg(target_os = "macos")]
fn selected_text() -> Result<String, SelectionError> {
    macos::selected_text()
}

#[cfg(target_os = "macos")]
mod macos {
    use super::SelectionError;
    use std::ffi::{c_char, c_void};
    use std::ptr;

    type Ref = *const c_void;
    #[repr(C)]
    struct Range {
        location: isize,
        length: isize,
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXUIElementCreateSystemWide() -> Ref;
        fn AXUIElementGetTypeID() -> usize;
        fn AXUIElementSetMessagingTimeout(element: Ref, timeout: f32) -> i32;
        fn AXUIElementCopyAttributeValue(element: Ref, attribute: Ref, value: *mut Ref) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(value: Ref);
        fn CFGetTypeID(value: Ref) -> usize;
        fn CFStringGetTypeID() -> usize;
        fn CFStringCreateWithCString(allocator: Ref, value: *const c_char, encoding: u32) -> Ref;
        fn CFStringGetLength(value: Ref) -> isize;
        fn CFStringGetCharacters(value: Ref, range: Range, buffer: *mut u16);
    }
    struct Owned(Ref);
    impl Drop for Owned {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: each Owned is returned by a CF/AX Create or Copy call.
                unsafe { CFRelease(self.0) };
            }
        }
    }
    fn attribute(element: Ref, name: &std::ffi::CStr) -> Result<Owned, SelectionError> {
        // SAFETY: name is NUL-terminated; element is a retained AX element.
        unsafe {
            let name = Owned(CFStringCreateWithCString(
                ptr::null(),
                name.as_ptr(),
                0x08000100,
            ));
            if name.0.is_null() {
                return Err(SelectionError::Unavailable);
            }
            let mut value = ptr::null();
            let error = AXUIElementCopyAttributeValue(element, name.0, &mut value);
            let value = Owned(value);
            if error != 0 || value.0.is_null() {
                return Err(SelectionError::Unavailable);
            }
            Ok(value)
        }
    }
    fn string(value: Ref) -> Result<String, SelectionError> {
        // SAFETY: check the CF type before calling string accessors; bounded
        // UTF-16 length prevents copying large selections into our process.
        unsafe {
            if CFGetTypeID(value) != CFStringGetTypeID() {
                return Err(SelectionError::Unavailable);
            }
            let length = CFStringGetLength(value);
            if !(0..=200).contains(&length) {
                return Err(SelectionError::TooLong);
            }
            let mut buffer = vec![0u16; length as usize];
            CFStringGetCharacters(
                value,
                Range {
                    location: 0,
                    length,
                },
                buffer.as_mut_ptr(),
            );
            String::from_utf16(&buffer).map_err(|_| SelectionError::Unavailable)
        }
    }
    pub fn selected_text() -> Result<String, SelectionError> {
        // SAFETY: AX objects are owned until the operation finishes. Only the
        // focused element's selected text is requested, never AXValue/content.
        unsafe {
            if !AXIsProcessTrusted() {
                return Err(SelectionError::Permission);
            }
            let system = Owned(AXUIElementCreateSystemWide());
            if system.0.is_null() {
                return Err(SelectionError::Unavailable);
            }
            AXUIElementSetMessagingTimeout(system.0, 0.5);
            let focused = attribute(system.0, c"AXFocusedUIElement")?;
            if CFGetTypeID(focused.0) != AXUIElementGetTypeID() {
                return Err(SelectionError::Unavailable);
            }
            AXUIElementSetMessagingTimeout(focused.0, 0.5);
            if let Ok(role) = attribute(focused.0, c"AXSubrole") {
                if string(role.0).as_deref() == Ok("AXSecureTextField") {
                    return Err(SelectionError::Unavailable);
                }
            }
            let selection = attribute(focused.0, c"AXSelectedText")?;
            string(selection.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_names_and_phrases_without_changing_case() {
        assert_eq!(
            normalize_selection("  Rubén   Poveda "),
            Ok("Rubén Poveda".into())
        );
        assert_eq!(normalize_selection("Grafiks+"), Ok("Grafiks+".into()));
    }
    #[test]
    fn rejects_empty_paragraphs_and_overlong_selections() {
        assert_eq!(normalize_selection("  <>\"' "), Err(SelectionError::Empty));
        assert_eq!(
            normalize_selection("first\nsecond"),
            Err(SelectionError::TooLong)
        );
        assert_eq!(
            normalize_selection(&"á".repeat(51)),
            Err(SelectionError::TooLong)
        );
        assert!(normalize_selection(&"á".repeat(50)).is_ok());
    }
    #[test]
    fn duplicates_preserve_existing_spelling_and_other_words() {
        let mut words = vec!["Rubén".into(), "Grafiksbox".into()];
        assert!(!add_unique(&mut words, "RUBÉN".into()));
        assert!(add_unique(&mut words, "Masterink".into()));
        assert_eq!(words, ["Rubén", "Grafiksbox", "Masterink"]);
    }
}
