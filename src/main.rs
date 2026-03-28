use colored::Colorize;
mod jp;

fn print_usage() {
    println!(
        "{} jp <filename> [{}] \nUse {} option for more details.",
        "Usage:".green().bold(),
        "<options>".blue(),
        "-h".blue()
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let buf = jp::read_file(&args[1]).unwrap();

    let object = jp::parse_value(&mut buf.chars().peekable()).unwrap();

    println!("{}", object.pretty());
}
