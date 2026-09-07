use std::{
    env, fs,
    io::{self, BufRead, Write},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

fn main() {
    let args: Vec<_> = env::args().collect();
    match args.get(1).map(String::as_str) {
        None | Some("copy") => {
            io::copy(&mut io::stdin(), &mut io::stdout()).unwrap();
        }
        Some("fail") => std::process::exit(3),
        Some("memory") => {
            let bytes = vec![42u8; 128 * 1024 * 1024];
            std::hint::black_box(&bytes);
            thread::sleep(Duration::from_secs(30));
            println!("{}", bytes[0]);
        }
        Some("later") => {
            thread::sleep(Duration::from_secs(2));
            fs::write(&args[2], "child survived").unwrap();
        }
        Some("spawn") | Some("background") => {
            let _child = Command::new(env::current_exe().unwrap())
                .args(["later", &args[2]])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            if args[1] == "spawn" {
                thread::sleep(Duration::from_secs(30));
            }
        }
        Some("answer") => {
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line).unwrap();
            assert_eq!(line.trim(), "ping");
            println!("pong");
        }
        Some("judge") => {
            println!("ping");
            io::stdout().flush().unwrap();
            let mut line = String::new();
            io::stdin().lock().read_line(&mut line).unwrap();
            assert_eq!(line.trim(), "pong");
        }
        Some("generate") => println!("sample"),
        _ => panic!("Unknown helper mode"),
    }
}
