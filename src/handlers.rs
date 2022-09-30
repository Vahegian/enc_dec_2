use rocket::{
    data::{Data, ToByteUnit},
    http::Status,
    response::{status, stream::ByteStream},
    serde::json::{serde_json::Value, Json},
    tokio::fs,
};
use rocket_dyn_templates::Template;
use serde_json::json;

use crate::types::{DirLS, State};

use crate::enc_dec::{decrypt_str, decrypt_stream, encrypt_str, encrypt_stream};
use crate::utils::create_dirs;

#[post("/upload?<path>", data = "<file>")]
pub async fn upload(
    path: &str,
    file: Data<'_>,
    state: &rocket::State<State>,
) -> status::Custom<Json<Value>> {
    let (enc_path, f_name) =
        match create_dirs(path, &state.data_dir, &state.key[..], &state.nonce[..]).await {
            Ok(v) => v,
            Err(e) => {
                error!("{e}");
                return status::Custom(
                    Status::InternalServerError,
                    Json(json!("Upload failed, bad path")),
                );
            }
        };
    let full_path = format!(
        "{enc_path}/{}",
        encrypt_str(&f_name, &state.key[..], &state.nonce[..]).unwrap()
    );
    match encrypt_stream(
        &mut file.open(2.gibibytes()),
        &full_path,
        &state.key[..],
        &state.nonce[..],
    )
    .await
    {
        Ok(_) => status::Custom(Status::Ok, Json(json!("Success"))),
        Err(e) => {
            error!("Upload failed {e}");
            status::Custom(Status::InternalServerError, Json(json!("Upload failed")))
        }
    }
}

#[get("/download?<path>")]
pub async fn download(
    path: &str,
    state: &rocket::State<State>,
) -> Result<ByteStream![Vec<u8>], status::Custom<Json<Value>>> {
    let full_path = format!("{}/{path}", state.data_dir);
    if !fs::metadata(&full_path).await.is_ok() {
        return Err(status::Custom(
            Status::InternalServerError,
            Json(json!("Download failed, bad path")),
        ));
    }
    match decrypt_stream(&full_path, &state.key[..], &state.nonce[..]).await {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("Download failed {e}");
            Err(status::Custom(
                Status::InternalServerError,
                Json(json!("Download failed")),
            ))
        }
    }
}

// #[get("/stream?<path>")]
// pub async fn stream(
//     path: &str,
//     state: &rocket::State<State>,
// ) -> Result<ByteStream![Vec<u8>], status::Custom<Json<Value>>> {
//     let full_path = format!("{}/{path}", state.data_dir);
//     if !fs::metadata(&full_path).await.is_ok() {
//         return Err(status::Custom(
//             Status::InternalServerError,
//             Json(json!("Download failed, bad path")),
//         ));
//     }
//     match decrypt_stream(&full_path, &state.key[..], &state.nonce[..]).await {
//         Ok(v) => Ok(v),
//         Err(e) => {
//             error!("Download failed {e}");
//             Err(status::Custom(
//                 Status::InternalServerError,
//                 Json(json!("Download failed")),
//             ))
//         }
//     }
// }

#[get("/browse?<path>")]
pub async fn browse(path: &str, state: &rocket::State<State>) -> status::Custom<Json<Value>> {
    let full_path = format!("{}/{path}", state.data_dir);
    if !fs::metadata(&full_path).await.is_ok() {
        return status::Custom(Status::InternalServerError, Json(json!("Failed, bad path")));
    }

    match fs::read_dir(full_path).await {
        Ok(mut ls) => {
            let mut dirls: Vec<DirLS> = vec![];
            loop {
                match ls.next_entry().await {
                    Ok(v) => {
                        if let Some(e) = v {
                            let name = e.file_name().into_string().unwrap();
                            dirls.push(DirLS {
                                enc_name: name.clone(),
                                name: decrypt_str(&name, &state.key[..], &state.nonce[..]).unwrap(),
                                is_dir: e.metadata().await.unwrap().is_dir(),
                            });
                            continue;
                        }
                        break;
                    }
                    _ => break,
                }
            }

            status::Custom(Status::Ok, Json(json!(dirls)))
        }
        Err(e) => {
            error!("{e}");
            status::Custom(
                Status::InternalServerError,
                Json(json!("Failed to read path")),
            )
        }
    }
}

#[get("/")]
pub fn index() -> Template {
    let context = false;
    Template::render("index", &context)
}
