use colored::Colorize;

fn print_log(tag: colored::ColoredString, text: String) {
    println!("{} {}", tag, text);
}
pub fn info(text: String) {
    print_log("[INFO]".cyan(), text);
}

pub fn error(text: String) {
    print_log("[ERROR]".red(), text);
}