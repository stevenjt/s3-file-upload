/**
 * s3-file-upload
 */

extern crate rusoto;
extern crate crypto;
extern crate term_painter;

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::fs::{File, remove_file};
use std::io::{stdin, Read, Write};
use std::clone::Clone;
use std::collections::HashMap;

use rusoto::{ProfileProvider, Region};
use rusoto::s3::{Object, S3Client, ListObjectsV2Request, GetObjectRequest, PutObjectRequest};
use rusoto::default_tls_client;

use crypto::md5::Md5;
use crypto::digest::Digest;

use term_painter::ToStyle;
use term_painter::Color::*;

/**
 * Enum for file status
 */
#[derive(PartialEq)]
enum FileStatus
{
    NotModified,
    Modified,
    New
}

/**
 * Struct for files
 */
#[derive(Clone)]
struct LocalFile
{
    path: PathBuf,
    md5: String
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
        Some(&"txt")  => "text/plain",
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
            println!("{}: {}", Red.paint("Error"), error);
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
fn local_file_upload_to_bucket(file: &LocalFile, local_path: &String, bucket_name: &String, public_file: bool)
{
    let provider = ProfileProvider::with_configuration(Path::new("credentials"), "user");
    let client = S3Client::new(default_tls_client().unwrap(), provider, Region::EuWest1);

    let file = file.clone();

    print!("{} \"{}\" to \"{}/{}\"...", Yellow.paint("Uploading"), file.path.to_str().unwrap(), bucket_name, local_file_get_relative_path(&file, &local_path));
    Some(std::io::stdout().flush());

    let mut file_handle = File::open(&file.path).expect("Could not open file");
    let mut contents: Vec<u8> = Vec::new();

    match file_handle.read_to_end(&mut contents)
    {
        Ok(_) =>
        {
            let mut acl = String::from("private");
            if public_file
            {
                acl = String::from("public-read");
            }

            let request = PutObjectRequest
            {
                bucket: bucket_name.to_owned(),
                key: local_file_get_relative_path(&file, &local_path),
                body: Some(contents),
                acl: Some(acl),
                content_type: Some(local_file_get_mime(&file)),
                ..PutObjectRequest::default()
            };

            match client.put_object(&request)
            {
                Ok(_) =>
                {
                    println!(" {}", Green.paint("DONE"));
                }
                Err(error) =>
                {
                    println!(" {}: {}", Red.paint("Error"), error);
                }
            }
        }
        Err(error) =>
        {
            println!("{}: {}", Red.paint("Error"), error);
        }
    }
}

/**
 * Create the local file checksums
 */
fn local_file_create_checksums(files: &Vec<LocalFile>, local_path: &String) -> LocalFile
{
    let mut path = PathBuf::from(local_path);
    path.push("checksums.txt");

    let mut checksums_file = File::create(path.to_str().unwrap()).expect("Could not create checksums");

    for file in files.clone()
    {
        let checksums_line = format!("{} {}\n", local_file_get_relative_path(&file, local_path), file.md5);
        checksums_file.write(checksums_line.as_bytes()).expect("Could not write checksums");
    }

    return LocalFile { path: path, md5: String::from("") };
}

/**
 * Delete the local file checksums
 */
fn local_file_delete_checksums(local_path: &String)
{
    let mut path = PathBuf::from(local_path);
    path.push("checksums.txt");

    remove_file(path).expect("Could not remove checksums");
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
 * Struct for checksums
 */
#[derive(Clone)]
struct Checksums
{
    files: HashMap<String, String>
}

/**
 * Get the s3 bucket checksums
 */
fn get_bucket_object_checksums(bucket_name: &String, bucket_objects: &Vec<Object>) -> Option<Checksums>
{
    match bucket_objects.iter().position(|obj| obj.clone().key.unwrap() == String::from("checksums.txt"))
    {
        Some(index) =>
        {
            let provider = ProfileProvider::with_configuration(Path::new("credentials"), "user");
            let client = S3Client::new(default_tls_client().unwrap(), provider, Region::EuWest1);

            let obj = bucket_objects.get(index).clone().unwrap();

            let request = GetObjectRequest
            {
                bucket: bucket_name.to_owned(),
                key: obj.clone().key.unwrap(),
                ..GetObjectRequest::default()
            };

            if let Ok(response) = client.get_object(&request)
            {
                let contents_bytes = response.body.unwrap();
                let contents = String::from_utf8_lossy(&contents_bytes);

                let mut hash_map = HashMap::new();

                for line in contents.split("\n")
                {
                    let mut line_split = line.split_whitespace();
                    let path = line_split.next();
                    let md5 = line_split.next();

                    if path != None && md5 != None
                    {
                        let path = String::from(path.unwrap());
                        let md5 = String::from(md5.unwrap());
                        hash_map.insert(path, md5);
                    }
                }

                let checksums = Checksums {files: hash_map};
                Some(checksums)
            }
            else
            {
                None
            }
        },
        None =>
        {
            None
        }
    }
}

/**
 * Check if the file matches the checksums
 */
fn local_file_matches_checksums(local_path: &String, file: &LocalFile, checksums: &Checksums) -> FileStatus
{
    for (path, md5) in &checksums.files
    {
        if &local_file_get_relative_path(file, local_path) == path
        {
            if file.md5 == md5.to_owned()
            {
                return FileStatus::NotModified;
            }
            else
            {
                return FileStatus::Modified;
            }
        }
    }
    return FileStatus::New;
}

/**
 * Get files from a local path
 */
fn get_local_files(local_path: &String, files: &mut Vec<LocalFile>, ignored_directories: &mut Vec<String>)
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
                    if !ignored_directories.contains(&file.file_name().to_str().unwrap().to_owned())
                    {
                        get_local_files(&file.path().to_str().unwrap().to_owned(), files, ignored_directories);
                    }
                }
                else
                {
                    let temp_file = LocalFile { path: file.path(), md5: String::from("") };
                    let file = LocalFile { path: file.path(), md5: local_file_get_md5(&temp_file) };
                    files.push(file);
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
        println!("\nUsage:\n\ns3-file-upload LOCAL_PATH BUCKET_NAME [OPTIONS]");
        println!("\nOptions:\n\n--ignored_directories    List of directory names using a comma separator, e.g. --ignored_directories=ignored_dir_one,ignored_dir_two");
        return;
    }

    let local_path  = env::args().nth(1).unwrap();
    let bucket_name = env::args().nth(2).unwrap();

    if !Path::new("credentials").exists()
    {
        println!("{}", Red.paint("credentials file could not be found"));
        return;
    }

    let mut ignored_directories: Vec<String> = Vec::new();

    // Set the ignored directories if the ignored_directories parameter is set
    for parameter in env::args()
    {
        if parameter.contains("--ignored_directories=")
        {
            let directories = parameter.split("=").nth(1).unwrap();

            for directory in directories.split(",")
            {
                ignored_directories.push(String::from(directory));
            }
        }
    }

    let mut files: Vec<LocalFile> = Vec::new();
    get_local_files(&local_path, &mut files, &mut ignored_directories);

    let mut bucket_objects: Vec<Object> = Vec::new();
    get_bucket_objects(&bucket_name, &mut bucket_objects);

    let checksums = match get_bucket_object_checksums(&bucket_name, &bucket_objects)
    {
        Some(checksums) =>
        {
            Some(checksums)
        },
        None =>
        {
            let hash_map = HashMap::new();
            let checksums = Checksums {files: hash_map};
            Some(checksums)
        }
    }.unwrap();

    let mut new_files: Vec<LocalFile> = Vec::new();
    let mut modified_files: Vec<LocalFile> = Vec::new();

    for file in &files
    {
        let file_status: FileStatus = local_file_matches_checksums(&local_path, &file, &checksums);
        if file_status == FileStatus::New
        {
            new_files.push(file.to_owned());
        }
        else if file_status == FileStatus::Modified
        {
            modified_files.push(file.to_owned());
        }
    }

    if modified_files.len() > 0 || new_files.len() > 0
    {
        println!("\nFiles found to be uploaded:\n");

        for file in &new_files
        {
            println!("{}:      {}", Green.paint("New"), local_file_get_relative_path(file, &local_path));
        }

        for file in &modified_files
        {
            println!("{}: {}", Green.paint("Modified"), local_file_get_relative_path(file, &local_path));
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
            println!("");

            for file in &modified_files
            {
                local_file_upload_to_bucket(&file, &local_path, &bucket_name, true);
            }

            for file in &new_files
            {
                local_file_upload_to_bucket(&file, &local_path, &bucket_name, true);
            }

            let new_checksums = local_file_create_checksums(&files, &local_path);
            local_file_upload_to_bucket(&new_checksums, &local_path, &bucket_name, false);
            local_file_delete_checksums(&local_path);

            println!("\n{}", Green.paint("UPLOAD COMPLETE"))
        }
        else
        {
            println!("\n{}", Yellow.paint("UPLOAD CANCELLED"))
        }
    }
    else
    {
        println!("\n{}", Yellow.paint("No pending modified/new files"));
    }
}
