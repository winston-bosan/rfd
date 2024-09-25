use objc2_foundation::MainThreadMarker;

pub fn run_on_ios_main<R: Send, F: FnOnce(MainThreadMarker) -> R + Send>(run: F) -> R {
    if let Some(mtm) = MainThreadMarker::new() {
        run(mtm)
    } else {
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        objc2_foundation::run_on_main(|mtm| run(mtm))
    }
}
