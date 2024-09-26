use crate::{FileDialog, FileHandle, MessageDialog, MessageDialogResult};
use objc2::rc::autoreleasepool;
use objc2::ClassType;
use objc2::{msg_send, Encode, Encoding};
use objc2_foundation::{MainThreadMarker, NSArray, NSString, NSURL};
use objc2_ui_kit::{
    self as ui_kit, UIDocumentPickerDelegate, UIDocumentPickerViewController, UIViewController,
};
use std::cell::RefCell;
use std::path::PathBuf;

use objc2_uniform_type_identifiers::UTType;
use std::sync::Mutex;

use objc2::__framework_prelude::ProtocolObject;
use objc2::rc::{Allocated, Retained};
use objc2::{declare_class, msg_send_id, mutability, DeclaredClass};
use objc2_foundation::{NSObject, NSObjectProtocol};

// Module lvl static file marker, not the proudest moment of my life
pub static CHOSEN_FILE: Mutex<(Option<String>, Option<String>)> = Mutex::new((None, None));

use crate::backend::FilePickerDialogImpl;

// Helper Const Generics to deal with slices
fn u8_slice_to_string<const C: usize>(n: [u8; C]) -> String {
    let vec = n.into_iter().map(|t| t as char).collect();
    vec
}

fn u8_slice_to_path<const C: usize>(n: [u8; C]) -> PathBuf {
    let string  = u8_slice_to_string(n);
    string.into()
}

#[derive(Clone)]
pub struct FilePath(Option<[u8; 200]>);
impl From<[u8; 200]> for FilePath {
    fn from(value: [u8; 200]) -> Self {
        FilePath(Some(value))
    }
}

// Ref encode
unsafe impl Encode for FilePath {
    const ENCODING: Encoding = Encoding::Object;
}

impl FilePickerDialogImpl for FileDialog {
    fn pick_file(self) -> Option<PathBuf> {
        autoreleasepool(move |_| {
            run_on_ios_main(move |mtm| {
                let callback_checker = present_document_picker(mtm);

                // VERY DUMB
                if let Some(history) = callback_checker.get_uri_history().0 {
                    Some(u8_slice_to_string(history).into())
                // JUST JANK FOR NOW
                } else {
                    None
                }
            })
        })
    }

    fn pick_files(self) -> Option<Vec<PathBuf>> {
        todo!()
    }
}

use crate::backend::AsyncFilePickerDialogImpl;

impl AsyncFilePickerDialogImpl for FileDialog {
    fn pick_file_async(self) -> DialogFutureType<Option<FileHandle>> {
        let pre_pack_future = async {
            run_on_ios_main(move |mtm| {
                let callback_checker = present_document_picker(mtm);
                loop {
                    if let Some(history) = callback_checker.get_uri_history().0 {
                        return Some(u8_slice_to_path(history).into())
                    }
                }

            })
        };

        Box::pin(pre_pack_future)
    }

    fn pick_files_async(self) -> DialogFutureType<Option<Vec<FileHandle>>> {
        todo!()
    }
}

use crate::backend::FolderPickerDialogImpl;
impl FolderPickerDialogImpl for FileDialog {
    fn pick_folder(self) -> Option<PathBuf> {
        todo!()
    }

    fn pick_folders(self) -> Option<Vec<PathBuf>> {
        todo!()
    }
}

use crate::backend::AsyncFolderPickerDialogImpl;
impl AsyncFolderPickerDialogImpl for FileDialog {
    fn pick_folder_async(self) -> DialogFutureType<Option<FileHandle>> {
        todo!()
    }

    fn pick_folders_async(self) -> DialogFutureType<Option<Vec<FileHandle>>> {
        todo!()
    }
}

// -- File Saving --

use crate::backend::FileSaveDialogImpl;
impl FileSaveDialogImpl for FileDialog {
    fn save_file(self) -> Option<PathBuf> {
        todo!()
    }
}

use crate::backend::AsyncFileSaveDialogImpl;
impl AsyncFileSaveDialogImpl for FileDialog {
    fn save_file_async(self) -> DialogFutureType<Option<FileHandle>> {
        todo!()
    }
}

// --- Message Dialog Temp ---

use crate::backend::MessageDialogImpl;

use crate::backend::AsyncMessageDialogImpl;
impl MessageDialogImpl for MessageDialog {
    fn show(self) -> MessageDialogResult {
        todo!()
    }
}
impl AsyncMessageDialogImpl for MessageDialog {
    fn show_async(self) -> DialogFutureType<MessageDialogResult> {
        todo!()
    }
}

// --- ObjC setup below ---

#[derive(Clone)]
struct UriHistory {
    last_uri: RefCell<FilePath>,
}

impl UIDocPickerDelegate {
    fn report_uri_retrieved(&self) -> Option<String> {
        todo!()
    }
}

declare_class!(
    struct UIDocPickerDelegate;

    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - Interior mutability is a safe default, but we need mainthreatonly because it deals with UI.
    // - `UIDocPickerDelegate` does not implement `Drop`.
    unsafe impl ClassType for UIDocPickerDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "UIDocPickerDelegate";
    }

    impl DeclaredClass for UIDocPickerDelegate {
        type Ivars = UriHistory;
    }

    unsafe impl UIDocPickerDelegate {
        #[method_id(init:)]
        fn init_with(this: Allocated<Self>) -> Option<Retained<Self>> {
            let this = this.set_ivars(UriHistory {
                last_uri: RefCell::new(FilePath(Some([0;200])).into()),
            });
            unsafe { msg_send_id![super(this), init] }
        }

        #[method(get_history)]
        fn __get_history(&self) -> FilePath {
            self.ivars().last_uri.clone().borrow().clone()
        }

        #[method(set_history)]
        fn __set_history(&self, new_history: [u8; 200]) {
            let a = self.ivars();
            *a.last_uri.borrow_mut() = FilePath(Some(new_history)).into();
        }

    }

    unsafe impl NSObjectProtocol for UIDocPickerDelegate {}

    unsafe impl UIDocumentPickerDelegate for UIDocPickerDelegate {
        #[method(documentPicker:didPickDocumentsAtURLs:)]
        unsafe fn documentPicker_didPickDocumentsAtURLs(
            &self,
            controller: &UIDocumentPickerViewController,
            urls: &NSArray<NSURL>,
        ) {
            if let Some(selected_url) = urls.firstObject() {
                println!("Selected URL: {:?}", selected_url);
                if let Some(path) = selected_url.path() {
                    if let Ok(bytes) = <[u8; 200]>::try_from(path.to_string().as_bytes()) {
                        self.set_uri_history(bytes);
                    }
                }
            }
        }

        #[method(documentPickerWasCancelled:)]
        unsafe fn documentPickerWasCancelled(&self, _controller: &UIDocumentPickerViewController) {
            println!("Document picker was cancelled");
        }

        #[method(documentPicker:didPickDocumentAtURL:)]
        unsafe fn documentPicker_didPickDocumentAtURL(
            &self,
            _controller: &UIDocumentPickerViewController,
            url: &NSURL,
        ) {
            println!("Selected single document URL: {:?}", url);
            if let Some(path) = url.path() {
                if let Ok(bytes) = <[u8; 200]>::try_from(path.to_string().as_bytes()) {
                    self.set_uri_history(bytes);
                }
            }
        }
    }
);

impl UIDocPickerDelegate {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send_id![mtm.alloc::<UIDocPickerDelegate>(), init] }
    }
    pub fn get_uri_history(&self) -> FilePath {
        unsafe { msg_send![self, get_history] }
    }

    pub fn set_uri_history(&self, new_history: [u8; 200]) {
        unsafe { msg_send![self, set_history: new_history] }
    }
}

// --- Example Above ---

#[derive(Clone, Copy, Debug, PartialEq)]
enum FileDialogType {
    Document,
    Image,
}

struct FileDialogParams {
    dialog_type: FileDialogType,
    allow_editing: bool,
    source_type: ui_kit::UIImagePickerControllerSourceType,
    allowed_uti_types: Option<Vec<String>>,
    file_extensions_filter: Option<Vec<String>>,
}

use crate::backend::ios::util::run_on_ios_main;
use crate::backend::DialogFutureType;

// Placeholder for your error handling and other utilities
fn show_message(msg: &str) {
    // Implement your message display logic
}

fn show_error(err: &str) {
    // Implement your error display logic
}

fn ttl(s: &str) -> &str {
    // Placeholder for translation/localization
    s
}
struct Error {
    msg: String,
}

impl Error {
    fn msg(msg: &str) -> Self {
        Self {
            msg: msg.to_string(),
        }
    }

    fn context(&self, context: &str) -> String {
        format!("{}: {}", context, self.msg)
    }
}

// How do we define the PickerDelegate class implementing UIDocumentPickerDelegate

// Function to present the document picker
fn present_document_picker(mtm: MainThreadMarker) -> Retained<UIDocPickerDelegate> {
    use objc2_ui_kit::UIDocumentPickerViewController;
    use objc2_uniform_type_identifiers::UTType;

    unsafe {
        let delegate = UIDocPickerDelegate::new(mtm);

        let picker = if available("14.0.0") {
            let csv = UTType::typeWithFilenameExtension(&*NSString::from_str("csv")).unwrap();
            let pleco = UTType::typeWithFilenameExtension(&*NSString::from_str("pleco")).unwrap();
            let apkg = UTType::typeWithFilenameExtension(&*NSString::from_str("apkg")).unwrap();

            let types = NSArray::from_slice(&[csv.as_ref(), pleco.as_ref(), apkg.as_ref()]);
            let allocated_picker = mtm.alloc::<UIDocumentPickerViewController>();

            UIDocumentPickerViewController::initForOpeningContentTypes(allocated_picker, &types)
        } else {
            panic!("iOS version not supported")
        };

        let ui_doc_picker_dyn_protocol_obj: &ProtocolObject<dyn UIDocumentPickerDelegate> =
            ProtocolObject::from_ref(&*delegate);
        picker.setDelegate(Some(ui_doc_picker_dyn_protocol_obj));

        // TODO: Very deprecated, plz fix
        if let Some(root_view_controller) = ui_kit::UIApplication::sharedApplication(mtm).keyWindow().and_then(|window| window.rootViewController()) {
            root_view_controller.presentViewController_animated_completion(&picker, true, None);
        } else {
            println!("Failed to get root view controller");
        }

        delegate
    }
}

// Placeholder for the availability check, not sure how to do it in objc2 yetc
fn available(_version: &str) -> bool {
    true
}

// unsafe impl FileDialog {
//     #[sel(init)]
//     fn init(&mut self) -> Option<&mut Self> {
//         let this: Option<&mut Self> = unsafe { msg_send![super(self), init] };
//         if let Some(this) = this {
//             this.flutter_result = FlutterResult::new();
//             this.params = None;
//             this.is_pick_directory = false;
//         }
//         this
//     }

//     #[sel(pickFile:result:)]
//     fn pick_file(
//         &self,
//         params: FileDialogParams,
//         result: FlutterResult,
//     ) {
//         self.flutter_result = result.clone();
//         self.params = Some(params);

//         let view_controller: Option<Id<UIViewController, Shared>> = unsafe {
//             msg_send![UIApplication::shared_application(), keyWindow().rootViewController()]
//         };

//         if let Some(view_controller) = view_controller {
//             if params.dialog_type == FileDialogType::Image {
//                 let image_picker = UIImagePickerController::new();

//                 if UIImagePickerController::is_source_type_available(
//                     ui_kit::UIImagePickerControllerSourceType::PhotoLibrary,
//                 ) {
//                     image_picker.set_delegate(Some(self));
//                     image_picker.set_source_type(params.source_type);
//                     image_picker.set_allows_editing(params.allow_editing);

//                     view_controller.present_view_controller(&image_picker, true, None);
//                 }
//             } else {
//                 let document_types = params.allowed_uti_types.unwrap_or_else(|| {
//                     NSArray::from_vec(vec![
//                         NSString::from("public.text"),
//                         NSString::from("public.content"),
//                         NSString::from("public.item"),
//                         NSString::from("public.data"),
//                     ])
//                 });

//                 let document_picker = UIDocumentPickerViewController::new_for_import_with_types(
//                     &document_types,
//                 );
//                 document_picker.set_delegate(Some(self));

//                 view_controller.present_view_controller(&document_picker, true, None);
//             }
//         } else {
//             result.set_result(Some("Getting rootViewController failed".to_string()));
//         }
//     }

//     #[sel(pickDirectory:result:)]
//     fn pick_directory(&self, result: FlutterResult) {
//         self.flutter_result = result.clone();
//         self.is_pick_directory = true;()

//         let view_controller: Option<Id<UIViewController, Shared>> = unsafe {
//             msg_send![UIApplication::shared_application(), keyWindow().rootViewController()]
//         };

//         if let Some(view_controller) = view_controller {
//             let document_types = NSArray::from_vec(vec![NSString::from("public.folder")]);

//             let document_picker =
//                 UIDocumentPickerViewController::new_for_open_with_types(&document_types);
//             document_picker.set_delegate(Some(self));

//             view_controller.present_view_controller(&document_picker, true, None);
//         } else {
//             result.set_result(Some("Getting rootViewController failed".to_string()));
//         }
//     }

//     // Handle the image picker result
//     #[sel(imagePickerController:didFinishPickingMediaWithInfo:)]
//     fn image_picker_controller_did_finish_picking(
//         &self,
//         picker: &UIImagePickerController,
//         info: Id<Object, Shared>,
//     ) {
//         picker.dismiss_view_controller(true, None);
//         let params = self.params.as_ref().unwrap();

//         // Handle the picked image or file URL
//         if params.allow_editing {
//             if let Some(picked_image) = unsafe { msg_send![info, objectForKey: "UIImagePickerControllerEditedImage"] }
//             {
//                 // Save the image to a temporary directory
//                 self.save_image_to_temp(picked_image);
//             }
//         } else {
//             if let Some(picked_file_url) = unsafe { msg_send![info, objectForKey: "UIImagePickerControllerImageURL"] }
//             {
//                 self.handle_picked_file(picked_file_url);
//             }
//         }
//     }

//     #[sel(imagePickerControllerDidCancel:)]
//     fn image_picker_controller_did_cancel(&self, picker: &UIImagePickerController) {
//         picker.dismiss_view_controller(true, None);
//         self.flutter_result.set_result(None);
//     }

//     // Handle the document picker result
//     #[sel(documentPicker:didPickDocumentsAtURLs:)]
//     fn document_picker_did_pick_documents(
//         &self,
//         _controller: &UIDocumentPickerViewController,
//         urls: NSArray<NSURL, Shared>,
//     ) {
//         if let Some(url) = urls.first() {
//             self.handle_picked_file(url);
//         }
//     }

//     #[sel(documentPickerWasCancelled:)]
//     fn document_picker_was_cancelled(&self, _controller: &UIDocumentPickerViewController) {
//         self.flutter_result.set_result(None);
//     }

//     fn handle_picked_file(&self, url: &NSURL) {
//         if self.is_pick_directory {
//             // Handle directory picking logic
//             if let Ok(bookmark_data) = url.bookmark_data(
//                 NSURLBookmarkCreationOptions::MinimalBookmark,
//                 None,
//                 None,
//             ) {
//                 let base64_string = bookmark_data.base64_encoded_string();
//                 self.flutter_result.set_result(Some(base64_string.to_string()));
//             } else {
//                 self.flutter_result.set_result(Some("Permission error".to_string()));
//             }
//         } else {
//             // Handle file picking logic
//             let file_extension = url.path_extension().unwrap_or(NSString::from(""));
//             let params = self.params.as_ref().unwrap();

//             if let Some(filter) = &params.file_extensions_filter {
//                 if !filter.iter().any(|ext| ext.eq_ignore_ascii_case(&file_extension)) {
//                     self.flutter_result.set_result(Some("Invalid file type".to_string()));
//                     return;
//                 }
//             }

//             // Move file to a temporary location
//             self.move_file_to_temp(url);
//         }
//     }

//     fn move_file_to_temp(&self, url: &NSURL) {
//         // Logic to move the file to a temporary directory
//         let temp_dir = std::env::temp_dir();
//         let file_name = url.last_path_component().unwrap_or(NSString::from("unknown"));
//         let destination_url = temp_dir.join(file_name.to_string());

//         // Move or copy the file to the destination
//         if let Err(e) = std::fs::copy(url.path().unwrap(), &destination_url) {
//             self.flutter_result.set_result(Some(format!("File copy error: {}", e)));
//         } else {
//             self.flutter_result
//                 .set_result(Some(destination_url.to_string_lossy().into()));
//         }
//     }

//     fn save_image_to_temp(&self, image: Id<Object, Shared>) {
//         // Logic to save the UIImage to a temp directory
//     }
// }

// struct PickerDelegate;
//
// impl PickerDelegate {
//     // Implement the delegate method
//     extern "C" fn document_picker_did_pick_documents_at_urls(
//         &self,
//         _picker: *mut Object,
//         _cmd: Sel,
//         _document_picker: *mut Object,
//         urls: *mut Object, // NSArray<NSURL *> *
//     ) {
//         unsafe {
//             // Get the first URL from the array
//             let url: *mut Object = msg_send![urls, firstObject];
//             // Start accessing security scoped resource
//             let need_close: bool = msg_send![url, startAccessingSecurityScopedResource];
//
//             // Initialize error pointer
//             let mut error: *mut Object = std::ptr::null_mut();
//             // Read data from the URL
//             let data: *mut NSData = msg_send![NSData::class(), dataWithContentsOfURL: url options: 2 error: &mut error as *mut _];
//
//             if need_close {
//                 // Stop accessing security scoped resource
//                 let _: () = msg_send![url, stopAccessingSecurityScopedResource];
//             }
//
//             if data.is_null() {
//                 // Handle read failure
//                 show_message(ttl("read-file-failed")).error();
//                 if !error.is_null() {
//                     let msg: *const NSString = msg_send![error, localizedDescription];
//                     let msg_str = (*msg).as_str(); // Assuming as_str() is safe and implemented
//                     show_error(&Error::msg(msg_str).context(ttl("read-file-failed")));
//                 }
//             } else {
//                 // Get temporary directory
//                 extern "C" {
//                     fn NSTemporaryDirectory() -> *mut NSString;
//                 }
//                 let dir = NSTemporaryDirectory();
//                 // Generate UUID
//                 let uuid: *mut NSUUID = msg_send![NSUUID::class(), UUID];
//                 let uuid_str: *mut NSString = msg_send![uuid, UUIDString];
//
//                 // Create file path
//                 let path = format!("{}{}", (*dir).as_str(), (*uuid_str).as_str());
//                 // Write data to file
//                 let _: () = msg_send![data, writeToFile: str_to_ns(&path) atomically: YES_CONST];
//
//                 // Update the chosen file path
//                 CHOSEN_FILE.lock().unwrap().1 = Some(path);
//             }
//         }
//     }
// }
