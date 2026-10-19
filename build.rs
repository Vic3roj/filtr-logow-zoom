fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon.ico");
        res.set("FileDescription", "Filtr logów Zoom FileUploader");
        res.set("ProductName", "Zoom FileUploader Link Filter");
        res.set("InternalName", "filtr_logow");
        res.set("OriginalFilename", "filtr_logow.exe");
        
        if let Err(e) = res.compile() {
            eprintln!("Błąd kompilacji zasobów: {}", e);
            std::process::exit(1);
        }
    }
}