/**
 * s3-file-upload
 */

extern crate rusoto;
extern crate crypto;

use std::env;
use std::fs;
use std::path::Path;
use std::fs::File;
use std::io::{stdin, Read, Write};
use std::clone::Clone;

use rusoto::{ProfileProvider, Region};
use rusoto::s3::{Object, S3Client, ListObjectsV2Request, PutObjectRequest};
use rusoto::default_tls_client;

use crypto::md5::Md5;
use crypto::digest::Digest;

/**
 * Struct for files
 */
#[derive(Clone)]
struct LocalFile
{
    path: std::path::PathBuf
}

/**
 * Get the mime type for a local file
 */
fn local_file_get_mime(file: &LocalFile) -> String
{
    let extension = file.path.extension().unwrap().to_str();

    let mime_type = match extension.as_ref()
    {
        Some(&"html") => "text/html",
        Some(&"css")  => "text/css",
        Some(&"png")  => "image/png",
        Some(&"gif")  => "image/gif",
        Some(&"jpg")  => "image/jpeg",
        Some(&"xml")  => "application/xml",
        _             => "application/octet-stream",
    };

    return mime_type.to_owned();
}

/**
 * Get the MD5 checksum of a local file
 */
fn local_file_get_md5(file: &LocalFile) -> String
{
    let mut file_handle = File::open(&file.path).unwrap();
    let mut contents: Vec<u8> = Vec::new();

    match file_handle.read_to_end(&mut contents)
    {
        Ok(_) =>
        {
            let mut md5_checksum = Md5::new();
            md5_checksum.input(&contents);
            return md5_checksum.result_str();
        }
        Err(error) =>
        {
            println!("Error: {}", error);
            return String::from("");
        }
    }
}

/**
 * Get a relative path for a local file
 */
fn local_file_get_relative_path(file: &LocalFile, local_path: &String) -> String
{
    let full_path: String = file.path.as_path().to_str().unwrap().to_owned();
    let last_path_section = full_path.split(local_path)
        .last()
        .unwrap()
        .replace("\\", "/");

    return last_path_section.to_owned();
}

/**
 * Upload a local file to an s3 bucket location
 */
fn local_file_upload_to_bucket(file: &LocalFile, local_path: &String, bucket_name: &String)
{
    let provider = ProfileProvider::with_configuration(Path::new("credentials"), "user");
    let client = S3Client::new(default_tls_client().unwrap(), provider, Region::EuWest1);

    let file = file.clone();

    print!("Uploading \"{}\" to \"{}/{}\"...", file.path.to_str().unwrap(), bucket_name, local_file_get_relative_path(&file, &local_path));
    Some(std::io::stdout().flush());

    let mut file_handle = File::open(&file.path).unwrap();
    let mut contents: Vec<u8> = Vec::new();

    match file_handle.read_to_end(&mut contents)
    {
        Ok(_) =>
        {
            let request = PutObjectRequest
            {
                bucket: bucket_name.to_owned(),
                key: local_file_get_relative_path(&file, &local_path),
                body: Some(contents),
                acl: Some(String::from("public-read")),
                content_type: Some(local_file_get_mime(&file)),
                ..PutObjectRequest::default()
            };

            match client.put_object(&request)
            {
                Ok(_) =>
                {
                    println!(" DONE");
                }
                Err(error) =>
                {
                   println!(" Error: {}", error);
                }
            }
        }
        Err(error) =>
        {
            println!("Error: {}", error);
        }
    }
}

/**
 * Get objects in an s3 bucket
 */
fn get_bucket_objects(bucket_name: &String, objects: &mut Vec<Object>)
{
    let provider = ProfileProvider::with_configuration(Path::new("credentials"), "user");
    let client = S3Client::new(default_tls_client().unwrap(), provider, Region::EuWest1);

    let request = ListObjectsV2Request
    {
        bucket: bucket_name.to_owned(),
        ..ListObjectsV2Request::default()
    };

    if let Ok(response) = client.list_objects_v2(&request)
    {
        for object in &response.contents.unwrap()
        {
            objects.push(object.clone());
        }
    }

}

/**
 * Get files from a local path
 */
fn get_local_files(local_path: &String, files: &mut Vec<LocalFile>)
{
    if let Ok(entries) = fs::read_dir(Path::new(&local_path))
    {
        for entry in entries
        {
            let file = entry.unwrap();

            if let Ok(metadata) = file.metadata()
            {
                if metadata.is_dir()
                {
                    get_local_files(&file.path().to_str().unwrap().to_owned(), files);
                }
                else
                {
                    files.push(LocalFile { path: file.path() });
                }
            }
        }
    }
}

/**
 * Main function
 */
fn main()
{
    if env::args().len() < 3
    {
        println!("Usage: s3-file-upload LOCAL_PATH BUCKET_NAME");
        return;
    }

    let local_path  = env::args().nth(1).unwrap();
    let bucket_name = env::args().nth(2).unwrap();

    if !Path::new("credentials").exists()
    {
        println!("credentials file could not be found");
        return;
    }

    let mut files: Vec<LocalFile> = Vec::new();
    get_local_files(&local_path, &mut files);

    // NOTE: bucket_objects is currently not used for anything.
    let mut bucket_objects: Vec<Object> = Vec::new();
    get_bucket_objects(&bucket_name, &mut bucket_objects);

    println!("\nFiles found to be uploaded:\n");

    for file in &files
    {
        println!("{} [{}]", file.path.to_str().unwrap(), local_file_get_md5(file));
    }

    let mut input_string = String::new();

    while input_string != "y" && input_string != "n"
    {
        println!("\nConfirm upload? <y/N>");

        input_string.clear();
        stdin().read_line(&mut input_string).expect("Did not input string");
        input_string = String::from(input_string.trim().to_lowercase());

        if input_string == "" || input_string == "no"
        {
            input_string = String::from("n");
        }
        else if input_string == "yes"
        {
            input_string = String::from("y");
        }
    }

    let confirm_upload = input_string == "y";

    if confirm_upload
    {
        for file in &files
        {
            local_file_upload_to_bucket(&file, &local_path, &bucket_name);
        }
    }
    else
    {
        println!("Upload cancelled")
    }
}
