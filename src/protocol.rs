#[derive(Debug)]
pub enum Command {
    Help,
    Quit,
    Nick(String),
    Create,
    Join(String),
    Msg(String)
}

pub fn parse_command(line: &str) -> Result<Command, String> {
    let mut parts = line.trim().splitn(2, ' ');

    let cmd = parts.next().unwrap_or("").to_uppercase();

    let rest = parts.next().map(str::trim);

    match cmd.as_str() {
        "HELP" => Ok(Command::Help),
        "QUIT" => Ok(Command::Quit),
        "NICK" => {
            let name = rest.ok_or("usage: NICK <name>")?;
            if name.is_empty() {
                return Err("nickname cannot be empty".into());
            }

            Ok(Command::Nick(name.to_string()))
        },
        "CREATE" => Ok(Command::Create),
        "JOIN" => {

            let code = rest.ok_or("usage: JOIN <CODE>")?;
            Ok(Command::Join(code.to_uppercase().to_string()))
        },
        "MSG" => {
            let text = rest.ok_or("usage: MSG <text>")?;

            Ok(Command::Msg(text.to_string()))
        },
        _ => Err(format!("unknown command: {}", cmd))
    }


}