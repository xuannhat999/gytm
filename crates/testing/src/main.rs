use api::load_cookies_firefox_based;

#[tokio::main]
async fn main() {
    println!("=== BẮT ĐẦU TEST LOAD COOKIE FIREFOX TỪ .CONFIG ===");
    println!("Lưu ý: Hãy chắc chắn đã tắt Firefox (pkill firefox) trước khi chạy.");
    println!("--------------------------------------------------");

    match load_cookies_firefox_based() {
        Ok(cookies) => {
            println!("  THÀNH CÔNG RỰC RỠ!");
            println!("- Đang kiểm tra Cookie Jar...");
        }
        Err(e) => {
            eprintln!("  THẤT BẠI: {:?}", e);
            eprintln!("Nguyên nhân có thể do:");
            eprintln!("1. Thư mục ~/.config/mozilla/firefox không tồn tại hoặc trống.");
            eprintln!("2. Firefox đang mở và khóa file cookies.sqlite.");
            eprintln!("3. Bạn chưa đăng nhập YouTube trên Firefox.");
        }
    }
}
