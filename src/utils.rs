use crate::enc_dec::{decrypt_large_file, decrypt_str, encrypt_large_file, encrypt_str};
use async_recursion::async_recursion;
use rocket::tokio::fs;

pub async fn create_dirs(
    path: &str,
    root_dir: &str,
    key: &[u8],
    nonce: &[u8],
) -> Result<(String, String), anyhow::Error> {
    let path_vec = path.split("/").collect::<Vec<&str>>();
    if path_vec.len() <= 1 {
        return Ok((root_dir.to_owned(), path_vec[path_vec.len() - 1].to_owned()));
    }

    let mut full_path = root_dir.to_owned();
    for p in path_vec[..path_vec.len() - 1].into_iter() {
        let enc_p = encrypt_str(p, key, nonce)?;
        if p.eq(&"") || p.eq(&".") {
            continue;
        }
        full_path = format!("{full_path}/{}", enc_p);
        // println!("\t\t {p} -- {enc_p} --- {full_path}");
    }
    fs::create_dir_all(&full_path).await?;
    Ok((full_path, path_vec[path_vec.len() - 1].to_owned()))
    // Err(anyhow!("gg"))
}

pub async fn create_dirs_dec(
    path: &str,
    root_dir: &str,
    key: &[u8],
    nonce: &[u8],
) -> Result<(String, String), anyhow::Error> {
    let path_vec = path.split("/").collect::<Vec<&str>>();
    if path_vec.len() <= 2 {
        return Ok((root_dir.to_owned(), path_vec[path_vec.len() - 1].to_owned()));
    }

    let mut full_path = root_dir.to_owned();
    for p in path_vec[2..path_vec.len() - 1].into_iter() {
        // println!("\t\t{p}");
        if p.eq(&"") || p.eq(&".") {
            continue;
        }
        let enc_p = decrypt_str(p, key, nonce)?;
        full_path = format!("{full_path}/{}", enc_p);
        // println!("\t\t {p} -- {enc_p} --- {full_path}");
    }
    fs::create_dir_all(&full_path).await?;
    Ok((full_path, path_vec[path_vec.len() - 1].to_owned()))
    // Err(anyhow!("gg"))
}

#[async_recursion]
pub async fn enc_local(
    input_dir: &str,
    out_dir: &str,
    key: &[u8],
    nonce: &[u8],
) -> Result<(), anyhow::Error> {
    let mut ls = fs::read_dir(input_dir).await?;
    loop {
        match ls.next_entry().await {
            Ok(v) => {
                if let Some(e) = v {
                    let path = e.path().to_str().unwrap().to_owned();
                    if e.path().is_dir() {
                        enc_local(&path, out_dir, key, nonce).await?;
                        continue;
                    }
                    let (enc_path, f_name) = create_dirs(&path, &out_dir, key, &nonce[..]).await?;
                    let full_path = format!("{enc_path}/{}", encrypt_str(&f_name, &key, &nonce)?);
                    println!("Encrypting {f_name}");
                    encrypt_large_file(&path, &full_path, &key, &nonce).await?;
                    continue;
                }
                break;
            }
            _ => break,
        }
    }
    Ok(())
}

#[async_recursion]
pub async fn dec_local(
    input_dir: &str,
    out_dir: &str,
    key: &[u8],
    nonce: &[u8],
) -> Result<(), anyhow::Error> {
    let mut ls = fs::read_dir(input_dir).await?;
    loop {
        match ls.next_entry().await {
            Ok(v) => {
                if let Some(e) = v {
                    let path = e.path().to_str().unwrap().to_owned();
                    if e.path().is_dir() {
                        dec_local(&path, out_dir, key, nonce).await?;
                        continue;
                    }
                    let (dec_path, f_name) =
                        create_dirs_dec(&path, &out_dir, key, &nonce[..]).await?;
                    let full_path = format!("{dec_path}/{}", decrypt_str(&f_name, &key, &nonce)?);
                    println!("Decrypting {f_name}");
                    decrypt_large_file(&path, &full_path, &key, &nonce).await?;
                    continue;
                }
                break;
            }
            _ => break,
        }
    }
    Ok(())
}
