use actix_web::{get, post, web, HttpResponse, Responder, HttpServer, App, middleware, http::header::{ContentDisposition, ContentEncoding}};
use base64::{engine::general_purpose, Engine};
use ring::{
    digest::{self, SHA256},
};
use serde::{Deserialize, Serialize};
use serde_json::{json};
use std::{fs, path::PathBuf, io::Read, time::{SystemTime, Duration, UNIX_EPOCH}, collections::HashMap, sync::{Arc, Mutex}};
use std::process::Command;
use lazy_static::lazy_static;
use actix_files::Files;
use catalyst_wasm_logic::{db_wrapper, db_wrapper::*, Keypair};


lazy_static! {
    static ref TAR_GZ: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
}

#[post("/post")]
// Define a function to create a new post in the database
async fn create_post(mut post: web::Json<Post>) -> impl Responder {
    let db: DbInstance = DbInstance::new_with_str("rocksdb", "./catalystdb", "").unwrap();
    db_wrapper::initialize(&db);
    // Verify the signature using the public key in the author field
    let public_key_bytes = match general_purpose::STANDARD.decode(&post.author) {
        Ok(bytes) => bytes,
        Err(_) => vec![],
    };

    // Generate a unique ID for the post using SHA-256
    let preimage = format!("{}{}{}{:?}", post.title, post.author, post.content, post.signature);
    let post_id = digest::digest(&SHA256, preimage.as_bytes());
    let post_id_str;
    
    match Keypair::verify_with_key(&public_key_bytes, post.content.as_bytes(), &post.signature) {
        true => {

            post_id_str = general_purpose::URL_SAFE_NO_PAD.encode(post_id.as_ref());
            // Check for any commands at the beginning of the post content
            let command_prefix = "{{";
            let command_suffix = "}}";
            if post.content.starts_with(command_prefix) {
                let end_index = post.content.find(command_suffix).unwrap_or(post.content.len());
                let command = &post.content[command_prefix.len()..end_index];
                match command.split(":").collect::<Vec<&str>>().as_slice() {
                    ["delete", cmd_post_id] => {
                        let seek_post_id = cmd_post_id.trim();
                        let post_dir = PathBuf::from("./posts").join(seek_post_id);
                        let post_file = post_dir.join("post.json");

                        // Check if the post exists
                        if !post_dir.exists() {
                            return HttpResponse::NotFound().json(json!({
                                "error": "Post not found"
                            }));
                        }

                        // Read the post data from disk
                        let post_data = fs::read_to_string(&post_file).unwrap();
                        let mut other_post: Post = serde_json::from_str(&post_data).unwrap();

                        // Check if the post author matches the command author
                        if other_post.author != post.author {
                            return HttpResponse::Forbidden().json(json!({
                                "error": "You are not authorized to delete this post"
                            }));
                        }

                        // Update the post content to indicate it was deleted
                        let content_hash = general_purpose::URL_SAFE_NO_PAD.encode(digest::digest(&SHA256, other_post.content.as_bytes()));
                        let deleted_content = json!({
                            "hash": content_hash,
                            "deleted": true
                        });
                        other_post.content = serde_json::to_string(&deleted_content).unwrap();
                        fs::write(&post_file, serde_json::to_string(&other_post).unwrap()).unwrap();
                    },
                    ["link", cmd_post_id] => {
                        // Open the post referenced in the command
                        let post_path = PathBuf::from("./posts").join(cmd_post_id);
                        let post_file = post_path.join("post.json");
                        let post_data = match fs::read_to_string(&post_file) {
                            Ok(data) => data,
                            Err(_) => return HttpResponse::InternalServerError().finish(),
                        };
                        let referenced_post: Post = match serde_json::from_str(&post_data) {
                            Ok(post) => post,
                            Err(_) => return HttpResponse::InternalServerError().finish(),
                        };
                        // Check if post authors match
                        if post.author == referenced_post.author {
                            // Open or create a links.json file in the referenced post dir
                            let links_file = post_path.join("links.json");
                            let mut links = match fs::OpenOptions::new().read(true).write(true).create(true).open(&links_file) {
                                Ok(file) => serde_json::from_reader(&file).unwrap_or_default(),
                                Err(_) => vec![],
                            };
                            // Append the new post id to the links.json file
                            links.push(post_id_str.clone());
                            match fs::write(&links_file, serde_json::to_string(&links).unwrap()) {
                                Ok(_) => (),
                                Err(_) => return HttpResponse::InternalServerError().finish(),
                            }
                        }
                    },
                    _ => (),
                }
            }
        },
        false => {
            // Author labelled as anonymous if public key not provided/verified
            post.author = format!("Unverified: {}", &post.author.clone());
            post_id_str = format!("unverified_{}", general_purpose::URL_SAFE_NO_PAD.encode(post_id.as_ref()));
        },
    }

    // Create a new directory for the post
    let post_dir = PathBuf::from("./posts").join(&post_id_str);
    fs::create_dir_all(&post_dir).unwrap();

    // Write the post data to a file
    let post_file = post_dir.join("post.json");
    let post_json = serde_json::to_string(&post.0).unwrap();
    fs::write(&post_file, post_json).unwrap();

    // Return the post ID to the client
    HttpResponse::Ok().json(json!({ "postID": post_id_str }))
}


#[post("/anchor")]
async fn create_anchor(anchor: web::Json<Anchor>) -> impl Responder {
    let db: DbInstance = DbInstance::new_with_str("rocksdb", "./catalystdb", "").unwrap();
    db_wrapper::initialize(&db);
    // Verify link_id meets criteria
    let post_id_len = anchor.post_id.len();
    let link_id_len = anchor.link_id.len();
    if !anchor.link_id.starts_with(&anchor.post_id)
        || post_id_len > link_id_len
    {
        return HttpResponse::BadRequest().body("Invalid link_id format");
    }

    // Check that the referenced post exists
    let referenced_post_dir = PathBuf::from("./posts").join(&anchor.post_id);
    if !referenced_post_dir.is_dir() {
        return HttpResponse::NotFound().body(format!("Referenced post {} not found", &anchor.post_id));
    }

    // Check that the referencing post exists
    let referencing_post_dir = PathBuf::from("./posts").join(&anchor.referencing_post_id);
    if !referencing_post_dir.is_dir() {
        return HttpResponse::NotFound().body(format!("Referencing post {} not found", &anchor.referencing_post_id));
    }

    // Create a new directory for the anchor within the post directory
    let mut anchor_dir = PathBuf::from("./posts")
        .join(&anchor.post_id)
        .join("anchors")
        .join(&anchor.link_id);

    if anchor.referencing_post_id.starts_with("unverified_") {
        let mut link_id = "unverified_".to_string();
        link_id.push_str(&anchor.link_id);
        anchor_dir = PathBuf::from("./posts")
            .join(&anchor.post_id)
            .join("anchors")
            .join(&link_id);
    }

    if anchor_dir.exists() {
        return HttpResponse::Conflict().body(format!("Anchor {} already exists", anchor.link_id));
    }

    fs::create_dir_all(&anchor_dir).unwrap();

    // Write the anchor data to a file
    let anchor_file = anchor_dir.join("anchor.json");
    let anchor_json = serde_json::to_string(&Anchor {
        link_id: anchor.link_id.clone(),
        post_id: anchor.post_id.clone(),
        reference: anchor.reference.clone(),
        referencing_post_id: anchor.referencing_post_id.clone(),
    })
    .unwrap();
    fs::write(&anchor_file, anchor_json).unwrap();

    // Return the anchor ID to the client
    HttpResponse::Ok().json(json!({ "anchorID": anchor.link_id }))
}


#[get("/post/{post_id}")]
async fn get_post_by_id(post_id: web::Path<String>) -> impl Responder {
    let db: DbInstance = DbInstance::new_with_str("rocksdb", "./catalystdb", "").unwrap();
    db_wrapper::initialize(&db);
    // Construct the path to the post directory
    let post_dir = PathBuf::from("./posts").join(post_id.into_inner());

    // Read the post data from the file
    let post_file = post_dir.join("post.json");
    let mut file = fs::File::open(&post_file).unwrap();
    let mut post_json = String::new();
    file.read_to_string(&mut post_json).unwrap();

    // Parse the post data and return it to the client
    let post: Post = serde_json::from_str(&post_json).unwrap();
    HttpResponse::Ok().json(post)
}

#[derive(Debug, Deserialize)]
struct PostQuery {
    max_posts: Option<usize>,
    recency_days: Option<u32>,
}

#[get("/posts")]
async fn get_posts(web::Query(query): web::Query<PostQuery>) -> impl Responder {
    let db: DbInstance = DbInstance::new_with_str("rocksdb", "./catalystdb", "").unwrap();
    db_wrapper::initialize(&db);

    let recency_days = query.recency_days.unwrap_or(7);
    let max_posts = query.max_posts.unwrap_or(10);

    // Get the list of post directories
    let post_dir = PathBuf::from("./posts");
    let post_dirs = match fs::read_dir(&post_dir){
        Ok(dir) => dir,
        Err(_) => return HttpResponse::Ok().json(vec![json!(Post::default())]),
    };

    // Filter the list of post directories to only include those that are recent enough
    let recency_threshold = SystemTime::now() - Duration::from_secs(u64::from(recency_days) * 24 * 60 * 60);
    let recent_post_dirs = post_dirs
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let post_dir_name = entry.file_name().to_str().unwrap().to_owned();
            !post_dir_name.contains("unverified_")
        })
        .filter(|entry| {
            let post_file = entry.path().join("post.json");
            match fs::metadata(&post_file) {
                Ok(metadata) => metadata.modified().unwrap() > recency_threshold,
                Err(_) => false,
            }
        })
        .collect::<Vec<_>>();

    let selected_post_dirs = recent_post_dirs.into_iter().take(max_posts);

    // Read the post data from each selected post directory
    let mut posts = HashMap::new();
    for post_dir in selected_post_dirs {
        let post_file = post_dir.path().join("post.json");
        let mut file = match fs::File::open(&post_file) {
            Ok(file) => file,
            Err(_) => continue,
        };
        let mut post_json = String::new();
        if let Err(_) = file.read_to_string(&mut post_json) {
            continue;
        }
        let post: Post = match serde_json::from_str(&post_json) {
            Ok(post) => post,
            Err(_) => continue,
        };
        let post_id = post_dir.file_name().to_str().unwrap().to_owned();
        posts.insert(post_id, post);
    }

    if posts.is_empty() {
        posts.insert("default".to_string(), Post::default());
    }

    HttpResponse::Ok().json(posts)
}

#[get("/post/{post_id}/anchors")]
async fn get_anchors(post_id: web::Path<String>) -> impl Responder {
    let db = match DbInstance::new_with_str("rocksdb", "./catalystdb", "") {
        Ok(instance) => instance,
        Err(_) => return HttpResponse::InternalServerError().into(),
    };

    if let Err(_) = db_wrapper::initialize(&db) {
        return HttpResponse::InternalServerError().into();
    }

    match db_wrapper::retrieve_anchors_by_post_id(&db, post_id.as_str()) {
        Ok(named_rows) => {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
            let anchors: Vec<Anchor> = named_rows
                .rows
                .into_iter()
                .filter_map(|row| {
                    let mut row_iter = row.into_iter();
                    let link_id = row_iter.next().unwrap().to_string();
                    let validity = row_iter.next().unwrap();
                    let post_id = row_iter.next().unwrap().to_string();
                    let reference = row_iter.next().unwrap().to_string();
                    let referencing_post_id = row_iter.next().unwrap().to_string();
                    let timestamp = validity.get_int().unwrap() as u128;
                    if timestamp <= now {
                        Some(Anchor {
                            link_id,
                            post_id,
                            reference,
                            referencing_post_id,
                        })
                    } else {
                        None
                    }
                })
                .collect();
            HttpResponse::Ok().json(anchors)
        }
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}


#[get("/post/{post_id}/linked_posts")]
async fn linked_posts(post_id: web::Path<String>) -> impl Responder {
    let db = match DbInstance::new_with_str("rocksdb", "./catalystdb", "") {
        Ok(instance) => instance,
        Err(_) => return HttpResponse::InternalServerError().into(),
    };

    if let Err(_) = db_wrapper::initialize(&db) {
        return HttpResponse::InternalServerError().into();
    }

    match db_wrapper::get_related_posts_by_same_author(&db, &post_id) {
        Ok(named_rows) => {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros();
            let linked_posts: HashMap<String, Post> = named_rows
                .rows
                .into_iter()
                .filter_map(|row| {
                    let mut row_iter = row.into_iter();
                    let new_post_id = row_iter.next().unwrap().to_string();
                    let validity = row_iter.next().unwrap();
                    let title = row_iter.next().unwrap().to_string();
                    let author = row_iter.next().unwrap().to_string();
                    let content = row_iter.next().unwrap().to_string();
                    let signature_base64 = row_iter.next().unwrap().to_string();
                    let signature = general_purpose::STANDARD
                        .decode(signature_base64)
                        .unwrap();
                    let timestamp = validity.get_int().unwrap() as u128;
                    if timestamp <= now {
                        Some((new_post_id, Post {
                            title,
                            author,
                            content,
                            signature,
                        }))
                    } else {
                        None
                    }
                })
                .collect();
            HttpResponse::Ok().json(linked_posts)
        }
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}


#[get("/bin")]
async fn get_current_bin() -> impl Responder {
    if let Ok(current_exe) = std::env::current_exe() {
        let mut file = fs::File::open(&current_exe).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        let filename = current_exe.file_name().unwrap().to_string_lossy();
        return HttpResponse::Ok()
            .content_type("application/octet-stream")
            .append_header(ContentDisposition::attachment(filename))
            .body(buffer);
    }
    HttpResponse::InternalServerError().into()
}

#[get("/src")]
async fn get_current_src() -> impl Responder {
    if let Ok(mutex) = TAR_GZ.lock() {
        if let Some(tar_gz) = mutex.as_ref() {
            return HttpResponse::Ok()
            .content_type("application/x-gtar")
            .append_header(ContentEncoding::Gzip)
            .append_header(ContentDisposition::attachment("src.tar.gz"))
            .body(tar_gz.clone());
        }
    }

    let output = Command::new("git")
        .args(&["archive", "--format=tar.gz", "HEAD"])
        .current_dir(".")
        .output()
        .expect("failed to execute git command");

    let tar_gz = output.stdout;
    let mut mutex = TAR_GZ.lock().unwrap();
    *mutex = Some(tar_gz.clone());

    HttpResponse::Ok()
        .content_type("application/x-gtar")
        .append_header(ContentEncoding::Gzip)
        .append_header(ContentDisposition::attachment("src.tar.gz"))
        .body(tar_gz)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::default())
            .service(create_anchor)
            .service(get_anchors)
            .service(create_post)
            .service(get_post_by_id)
            .service(get_posts)
            .service(linked_posts)
            .service(get_current_bin)
            .service(get_current_src)
            .service(Files::new("/", "."))

    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
