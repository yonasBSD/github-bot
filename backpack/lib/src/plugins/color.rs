use colored::*;
use rhai::EvalAltResult;
use std::fmt::Display;

/// A simple function to print a message in a specific color.
/// This function is registered with Rhai as `cprint(message, color)`.
pub fn cprint(message: &str, color_name: &str) -> Result<(), Box<EvalAltResult>> {
    // Helper function to print the colored text
    fn print_colored_text<T: Display + colored::Colorize>(text: T, color: Color) {
        println!("{}", text.color(color));
    }

    match color_name.to_lowercase().as_str() {
        "red" => print_colored_text(message, Color::Red),
        "green" => print_colored_text(message, Color::Green),
        "blue" => print_colored_text(message, Color::Blue),
        "yellow" => print_colored_text(message, Color::Yellow),
        "cyan" => print_colored_text(message, Color::Cyan),
        "magenta" => print_colored_text(message, Color::Magenta),
        "white" => print_colored_text(message, Color::White),
        "black" => print_colored_text(message, Color::Black),
        _ => {
            // If the color is unknown, just print the message normally and give a warning.
            eprintln!("Rhai Color Error: Unknown color '{color_name}'. Printing uncolored.");
            println!("{}", message);
        }
    }
    Ok(())
}
