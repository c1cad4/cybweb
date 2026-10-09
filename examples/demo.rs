fn main() {
    println!(
        "{}",
        cyb_web::html::html_to_text(
            "<body><h1>Local knowledge</h1><script>ignored()</script></body>"
        )
    );
}
