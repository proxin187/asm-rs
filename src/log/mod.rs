use colored::*;

pub fn info(message: &str) {
    println!("> {}: {}", "info".green(), message);
}

pub fn error(prefix: &str, message: &str) {
    println!("> {}:{}: {}", prefix, "error".red(), message);
}


