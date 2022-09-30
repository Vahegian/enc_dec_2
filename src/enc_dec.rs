use anyhow::anyhow;
use chacha20poly1305::KeyInit;
use chacha20poly1305::{aead::stream, XChaCha20Poly1305};
use rocket::data::DataStream;
use rocket::response::stream::ByteStream;
use rocket::tokio::fs::File;
use rocket::tokio::io::AsyncReadExt;
use rocket::tokio::io::AsyncWriteExt;
use std::io::ErrorKind;

pub fn encrypt_str(data: &str, key: &[u8], nonce: &[u8]) -> Result<String, anyhow::Error> {
    if key.len() != 32 {
        return Err(anyhow!("Key len not == 32"));
    }
    if nonce.len() != 19 {
        return Err(anyhow!("Nonce len not == 19"));
    }
    let aead = XChaCha20Poly1305::new(key.as_ref().into());
    let stream_encryptor = stream::EncryptorBE32::from_aead(aead, nonce.as_ref().into());

    let ciphertext = stream_encryptor
        .encrypt_last(data.as_bytes())
        .map_err(|err| anyhow!("Encrypting large file: {}", err))?;
    let h = hex::encode(ciphertext);
    Ok(h)
}

pub fn decrypt_str(data: &str, key: &[u8], nonce: &[u8]) -> Result<String, anyhow::Error> {
    if key.len() != 32 {
        return Err(anyhow!("Key len not == 32"));
    }
    if nonce.len() != 19 {
        return Err(anyhow!("Nonce len not == 19"));
    }
    let aead = XChaCha20Poly1305::new(key.as_ref().into());
    let stream_decryptor = stream::DecryptorBE32::from_aead(aead, nonce.as_ref().into());

    let p_data = stream_decryptor
        .decrypt_last(&hex::decode(data)?[..])
        .map_err(|err| anyhow!("Encrypting large file: {}", err))?;

    Ok(String::from_utf8(p_data)?)
}

pub async fn encrypt_stream(
    stream: &mut DataStream<'_>,
    dist_file_path: &str,
    key: &[u8],
    nonce: &[u8],
) -> Result<(), anyhow::Error> {
    if key.len() != 32 {
        return Err(anyhow!("Key len not == 32"));
    }
    if nonce.len() != 19 {
        return Err(anyhow!("Nonce len not == 19"));
    }
    let aead = XChaCha20Poly1305::new(key.as_ref().into());
    let mut stream_encryptor = stream::EncryptorBE32::from_aead(aead, nonce.as_ref().into());

    const BUFFER_LEN: usize = 2048;
    let mut dist_file = File::create(dist_file_path).await?;
    loop {
        let mut buffer = [0u8; BUFFER_LEN];
        match stream.read_exact(&mut buffer).await {
            Ok(_) => {
                let ciphertext = stream_encryptor
                    .encrypt_next(&buffer.to_vec()[..])
                    .map_err(|err| anyhow!("Encrypting large file: {}", err))?;
                dist_file.write(&ciphertext).await?;
            }
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
                let mut last_bytes: usize = BUFFER_LEN - 1; //&buffer.into_iter().filter(|x| x.gt(&0)).collect::<Vec<u8>>()[..];
                for i in 1..BUFFER_LEN {
                    if buffer[BUFFER_LEN - i] > 0 {
                        break;
                    }
                    last_bytes -= 1;
                }
                let ll = &buffer[..last_bytes];
                // println!("{last_bytes} -- ll {ll:?} -- {}", ll.len());
                let ciphertext = stream_encryptor
                    .encrypt_last(ll)
                    .map_err(|err| anyhow!("Encrypting large file: {}", err))?;
                dist_file.write(&ciphertext).await?;
                break;
            }
            _ => break,
        };
    }
    Ok(())
}

pub async fn decrypt_stream(
    dist_file_path: &str,
    key: &[u8],
    nonce: &[u8],
) -> Result<ByteStream![Vec<u8>], anyhow::Error> {
    if key.len() != 32 {
        return Err(anyhow!("Key len not == 32"));
    }
    if nonce.len() != 19 {
        return Err(anyhow!("Nonce len not == 19"));
    }
    let aead = XChaCha20Poly1305::new(key.as_ref().into());
    let mut stream_decryptor = stream::DecryptorBE32::from_aead(aead, nonce.as_ref().into());
    const BUFFER_LEN: usize = 2048 + 16;
    let mut buffer = [0u8; BUFFER_LEN];
    let mut dist_file = File::open(dist_file_path).await?;

    Ok(ByteStream! {
        loop {

            let read_count = dist_file.read(&mut buffer).await.unwrap();

            if read_count == BUFFER_LEN {
                let plaintext = stream_decryptor
                    .decrypt_next(buffer.as_slice())
                    .map_err(|err| anyhow!("Decrypting large file: {}", err)).unwrap();
                    yield plaintext.to_vec()
            } else if read_count == 0 {
                break;
            } else {
                let plaintext = stream_decryptor
                    .decrypt_last(&buffer[..read_count])
                    .map_err(|err| anyhow!("Decrypting large file: {}", err)).unwrap();
                    yield plaintext.to_vec();
                break;
            }
        }
    })
}

pub async fn encrypt_large_file(
    source_file_path: &str,
    dist_file_path: &str,
    key: &[u8],
    nonce: &[u8],
) -> Result<(), anyhow::Error> {
    if key.len() != 32 {
        return Err(anyhow!("Key len not == 32"));
    }
    if nonce.len() != 19 {
        return Err(anyhow!("Nonce len not == 19"));
    }
    let aead = XChaCha20Poly1305::new(key.as_ref().into());
    let mut stream_encryptor = stream::EncryptorBE32::from_aead(aead, nonce.as_ref().into());

    const BUFFER_LEN: usize = 2048;
    let mut buffer = [0u8; BUFFER_LEN];

    let mut source_file = File::open(source_file_path).await?;
    let mut dist_file = File::create(dist_file_path).await?;

    loop {
        let read_count = source_file.read(&mut buffer).await?;

        if read_count == BUFFER_LEN {
            let ciphertext = stream_encryptor
                .encrypt_next(&buffer.to_vec()[..])
                .map_err(|err| anyhow!("Encrypting large file: {}", err))?;
            dist_file.write(&ciphertext).await?;
        } else {
            let ciphertext = stream_encryptor
                .encrypt_last(&buffer[..read_count])
                .map_err(|err| anyhow!("Encrypting large file: {}", err))?;
            dist_file.write(&ciphertext).await?;
            break;
        }
    }

    Ok(())
}

pub async fn decrypt_large_file(
    encrypted_file_path: &str,
    dist: &str,
    key: &[u8],
    nonce: &[u8],
) -> Result<(), anyhow::Error> {
    if key.len() != 32 {
        return Err(anyhow!("Key len not == 32"));
    }
    if nonce.len() != 19 {
        return Err(anyhow!("Nonce len not == 19"));
    }
    let aead = XChaCha20Poly1305::new(key.as_ref().into());
    let mut stream_decryptor = stream::DecryptorBE32::from_aead(aead, nonce.as_ref().into());

    const BUFFER_LEN: usize = 2048 + 16;
    let mut buffer = [0u8; BUFFER_LEN];

    let mut encrypted_file = File::open(encrypted_file_path).await?;
    let mut dist_file = File::create(dist).await?;

    loop {
        let read_count = encrypted_file.read(&mut buffer).await?;

        if read_count == BUFFER_LEN {
            let plaintext = stream_decryptor
                .decrypt_next(buffer.as_slice())
                .map_err(|err| anyhow!("Decrypting large file: {}", err))?;
            dist_file.write(&plaintext).await?;
        } else if read_count == 0 {
            break;
        } else {
            let plaintext = stream_decryptor
                .decrypt_last(&buffer[..read_count])
                .map_err(|err| anyhow!("Decrypting large file: {}", err))?;
            dist_file.write(&plaintext).await?;
            break;
        }
    }

    Ok(())
}
