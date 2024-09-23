use objc2::runtime::Object;
use objc2_ui_kit::{
    self as ui_kit, UIDocumentPickerMode, UIDocumentPickerDelegate,
};
use crate::{FileDialog, FileHandle};

#[derive(Clone, Copy, Debug, PartialEq)]
enum FileDialogType {
    Document,
    Image,
}

struct FileDialogParams {
    dialog_type: FileDialogType,
    allow_editing: bool,
    source_type: ui_kit::UIImagePickerControllerSourceType,
    allowed_uti_types: Option<Vec<NSString>>,
    file_extensions_filter: Option<Vec<String>>,
}

use crate::backend::AsyncFilePickerDialogImpl;
impl AsyncFilePickerDialogImpl for FileDialog {
    fn pick_file_async(self) -> DialogFutureType<Option<FileHandle>> {
        let win = self.parent.as_ref().map(window_from_raw_window_handle);

        let future = ModalFuture::new(
            win,
            move |mtm| Panel::build_pick_file(&self, mtm),
            |panel, res_id| {
                if res_id == NSModalResponseOK {
                    Some(panel.get_result().into())
                } else {
                    None
                }
            },
        );

        Box::pin(future)
    }

    fn pick_files_async(self) -> DialogFutureType<Option<Vec<FileHandle>>> {
        let win = self.parent.as_ref().map(window_from_raw_window_handle);

        let future = ModalFuture::new(
            win,
            move |mtm| Panel::build_pick_files(&self, mtm),
            |panel, res_id| {
                if res_id == NSModalResponseOK {
                    Some(
                        panel
                            .get_results()
                            .into_iter()
                            .map(FileHandle::wrap)
                            .collect(),
                    )
                } else {
                    None
                }
            },
        );

        Box::pin(future)
    }
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
//         self.is_pick_directory = true;

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