# s3-file-upload

A command line utility that uploads files to an AWS S3 bucket.

```
$ s3-file-upload LOCAL_PATH BUCKET_NAME [OPTIONS]
```

Valid options:
```
--ignored_directories    List of directory names using a comma separator, e.g. --ignored_directories=ignored_dir_one,ignored_dir_two
```

AWS user details are found in a `credentials` file that should be present in the same directory the app is run from. This file should contain the `aws_access_key_id` and `aws_secret_access_key` for the user:
```
[user]
aws_access_key_id = ACCESS_KEY_ID
aws_secret_access_key = SECRET_ACCESS_KEY
```
____________________________________

This is a small utility I needed, and also something to use to try out [Rust](https://www.rust-lang.org/).
