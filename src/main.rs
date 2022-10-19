#[macro_use]
extern crate rocket;
use chacha20poly1305::aead::OsRng;
use rand::RngCore;
use rocket::Route;
use rocket_dyn_templates::Template;
use types::cli;
use types::State;
mod yaml_parser;
use pretty_env_logger;
mod enc_dec;
mod handlers;
mod types;
mod utils;
use utils::dec_local;
use utils::enc_local;

pub const BUFFER_SIZE: usize = 2048;

macro_rules! get_routes {
  ($n:ident -> [$($r:expr),*]) => {
    fn $n() -> Vec<Route>{
       routes![$($r),*]
    }
  };
}

get_routes!(app_routes ->[handlers::upload, handlers::download, handlers::browse, handlers::index]);

fn create_secrets() {
    let mut large_file_key = [0u8; 32];
    let mut large_file_nonce = [0u8; 19];
    OsRng.fill_bytes(&mut large_file_key);
    OsRng.fill_bytes(&mut large_file_nonce);
    println!("Key");
    large_file_key.map(|v| print!("{v:02X?}"));
    println!("\nNonce");
    large_file_nonce.map(|v| print!("{v:02X?}"));
    println!()
}

#[rocket::main]
async fn main() {
    pretty_env_logger::init();
    let opt = cli::Opt::get_args();

    if let Some(cli::Command::Gen) = opt.cmd {
        create_secrets();
        return;
    }

    let conf = match opt.config {
        Some(v) => yaml_parser::get_conf(&v),
        None => {
            println!("Please specify path to config file");
            return;
        }
    };

    let large_file_key = hex::decode(conf.key).unwrap();
    let large_file_nonce = hex::decode(conf.nonce).unwrap();

    match (opt.cmd, opt.input, opt.output) {
        (Some(cli::Command::Enc), Some(root_dir), Some(out_dir)) => {
            enc_local(
                &root_dir,
                &out_dir,
                &large_file_key[..],
                &large_file_nonce[..],
            )
            .await
            .unwrap();
            return;
        }

        (Some(cli::Command::Dec), Some(root_dir), Some(out_dir)) => {
            dec_local(
                &root_dir,
                &out_dir,
                &large_file_key[..],
                &large_file_nonce[..],
            )
            .await
            .unwrap();
            return;
        }

        _ => {}
    }

    let _ = rocket::build()
        .mount("/", app_routes())
        .manage(State {
            key: large_file_key,
            nonce: large_file_nonce,
            data_dir: conf.data_dir,
        })
        .attach(Template::fairing())
        .launch()
        .await
        .unwrap();
}
