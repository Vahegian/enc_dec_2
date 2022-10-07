use std::sync::Arc;

use rocket::{
    data::{Data, ToByteUnit},
    http::{ContentType, Status},
    response::{
        status,
        stream::{ByteStream, ReaderStream},
    },
    serde::json::{serde_json::Value, Json},
    tokio::{
        fs,
        sync::{mpsc::Receiver, Mutex},
    },
    Request,
};
use rocket_dyn_templates::Template;
use serde_json::json;

use crate::{types::{DirLS, State}, BUFFER_SIZE};

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

pub struct ProxyData {
    data: Arc<Mutex<Receiver<Vec<u8>>>>,
    ext: String,
    is_done: Arc<std::sync::Mutex<bool>>,
}

use futures::{stream::Stream, StreamExt};

impl Stream for ProxyData {
    type Item = Arc<Mutex<Receiver<Vec<u8>>>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let done = {
            let b = self.is_done.clone();
            let b = (*b.lock().unwrap()).to_owned();
            // *b
            b
        };
        if done {
            return std::task::Poll::Ready(None);
        }
        std::task::Poll::Ready(Some(self.data.clone()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl<'r> rocket::response::Responder<'r, 'static> for ProxyData {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let mut res = rocket::Response::build().finalize();
        // copy_response_headers(&self.data, &mut res);
        res.set_header(ContentType::from_extension(&self.ext).unwrap_or(ContentType::Bytes));
        // let mut rr = self.data.get_mut().unwrap();
        let s = ByteStream::from(
            self.filter_map(|v| async move { v.clone().lock().await.recv().await }),
        );
        let ss = s.0.map(std::io::Cursor::new);

        res.set_streamed_body(ReaderStream::from(ss));
        Ok(res)
    }
}

#[get("/download?<path>")]
pub async fn download(
    path: &str,
    state: &rocket::State<State>,
) -> Result<ProxyData, status::Custom<Json<Value>>> {
    let full_path = format!("{}/{path}", state.data_dir);
    if !fs::metadata(&full_path).await.is_ok() {
        return Err(status::Custom(
            Status::InternalServerError,
            Json(json!("Download failed, bad path")),
        ));
    }
    let file_name = path.split("/").last().unwrap();
    let file_name = decrypt_str(file_name, &state.key[..], &state.nonce[..]).unwrap();
    let ext = file_name.split(".").last().unwrap();
    let (tx, rx) = rocket::tokio::sync::mpsc::channel(BUFFER_SIZE);
    let (key, nonce) = (state.key.to_owned(), state.nonce.to_owned());

    let res = ProxyData {
        data: Arc::new(Mutex::new(rx)),
        ext: ext.to_owned(),
        is_done: Arc::new(std::sync::Mutex::new(false)),
    };

    let is_done = res.is_done.clone();
    rocket::tokio::spawn(async move {
        if let Err(e) = decrypt_stream(&full_path, &key[..], &nonce[..], tx).await {
            error!("/download {e}")
        };
        let mut d = is_done.lock().unwrap();
        *d = true;
    });

    Ok(res)
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
