use actix_web::{get, post, web::{self}, HttpResponse, Responder, HttpServer, App, middleware, http::header::{ContentDisposition, ContentEncoding}};
use base64::{engine::general_purpose, Engine};
use rand::{thread_rng, Rng, distributions::Alphanumeric};
use ring::{
    digest::{self, SHA256},
};
use serde::{Deserialize};
use serde_json::{json};
use std::{fs, io::Read, time::{SystemTime, UNIX_EPOCH}, collections::HashMap, sync::{Arc, Mutex}, net::SocketAddr, str::FromStr};
use std::process::Command;
use lazy_static::lazy_static;
use actix_files::Files;
use catalyst_wasm_logic::{db_wrapper, db_wrapper::*, Keypair};
use wasm_peers_signaling_server::many_to_many;
use warp::Filter;

lazy_static! {
    static ref TAR_GZ: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    static ref BIN: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
}

#[post("/post")]
async fn create_post(mut post: web::Json<Post>) -> impl Responder {
    let db = match DbInstance::new_with_str("rocksdb", "./catalystdb", "") {
        Ok(instance) => instance,
        Err(_) => return HttpResponse::InternalServerError().into(),
    };

    if let Err(_) = db_wrapper::initialize(&db) {
        return HttpResponse::InternalServerError().into();
    }

    // Verify the signature using the public key in the author field
    let public_key_bytes = match general_purpose::STANDARD.decode(&post.author) {
        Ok(bytes) => bytes,
        Err(_) => vec![],
    };

    // Generate a unique ID for the post using SHA-256
    let preimage = format!(
        "{}{}{}{:?}",
        post.title, post.author, post.content, post.signature
    );
    let post_id = digest::digest(&SHA256, preimage.as_bytes());
    let post_id_str;
    let signature = general_purpose::STANDARD
        .decode(&post.signature)
        .unwrap();

    match Keypair::verify_with_key(
        &public_key_bytes,
        post.content.as_bytes(),
        &signature,
    ) {
        true => {
            post_id_str = general_purpose::URL_SAFE_NO_PAD.encode(post_id.as_ref());
            // Check for any commands at the beginning of the post content
            let command_prefix = "{{";
            let command_suffix = "}}";
            if post.content.starts_with(command_prefix) {
                let end_index = post.content
                    .find(command_suffix)
                    .unwrap_or(post.content.len());
                let command = &post.content[command_prefix.len()..end_index];
                match command.split(":").collect::<Vec<&str>>().as_slice() {
                    ["delete", cmd_post_id] => {
                        let seek_post_id = cmd_post_id.trim();
                        let other_post_result = db_wrapper::retrieve_post_by_id(&db, seek_post_id);
                    
                        // Read the post data from the database
                        let other_post_row = other_post_result.unwrap().rows.into_iter().next().unwrap();
                        let mut row_iter = other_post_row.into_iter();
                        let post_id_string: String = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                        let _ = row_iter.next();
                        let title = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                        let author = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                        let content = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                        let signature = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                        
                        let mut other_post = Post {
                            title,
                            author,
                            content,
                            signature,
                        };
                    
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
                        let mut deleted_post = &mut other_post;
                        deleted_post.content = serde_json::to_string(&deleted_content).unwrap();
                    
                        // Create a new post with the same key (post_id) and the stub in place of its content
                        if let Err(_) = db_wrapper::create_post(&db, &post_id_string.as_str(), &deleted_post) {
                            return HttpResponse::InternalServerError().into();
                        }
                    },                    
                    ["link", cmd_post_id] => {
                        let seek_post_id = cmd_post_id.trim();
                        let other_post_result = db_wrapper::retrieve_post_by_id(&db, seek_post_id);
                    
                        // Check if the post exists
                        if let Err(_) = other_post_result {
                            return HttpResponse::NotFound().json(json!({
                                "error": "Post not found"
                            }));
                        }
                    
                        // Read the post data from the database
                        let other_post_row = other_post_result.unwrap().rows.into_iter().next().unwrap();
                        let mut row_iter = other_post_row.into_iter();
                        let _ = row_iter.next();
                        let title = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                        let author = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                        let content = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                        let signature = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                        
                        let other_post = Post {
                            title,
                            author,
                            content,
                            signature,
                        };
                    
                        // Check if post authors match
                        if post.author == other_post.author {
                            // Generate a unique link_id based on the post_id and a random string
                            let rand_str: String = thread_rng()
                                .sample_iter(&Alphanumeric)
                                .take(8)
                                .map(char::from)
                                .collect();
                            let link_id = format!("{}_{}", post_id_str.clone(), rand_str);
                    
                            // Create a new Anchor with an empty reference field
                            let new_anchor = Anchor {
                                post_id: cmd_post_id.to_string(),
                                link_id,
                                referencing_post_id: post_id_str.clone(),
                                reference: "".to_string(),
                            };
                    
                            let _ = do_save_anchor(&new_anchor);
                        }
                    },
                    _ => (),
                    
                }
            }
            if let Err(_) = db_wrapper::create_post(&db, &post_id_str, &post.0) {
                return HttpResponse::InternalServerError().into();
            }
        }
        false => {
            // Author labelled as anonymous if public key not provided/verified
            post.author = format!("Unverified: {}", &post.author.clone());
            post_id_str = format!(
                "unverified_{}",
                general_purpose::URL_SAFE_NO_PAD.encode(post_id.as_ref())
            );
            if let Err(_) = db_wrapper::create_post(&db, &post_id_str, &post.0) {
                return HttpResponse::InternalServerError().into();
            }
        }
    }

    // Return the post ID to the client
    HttpResponse::Ok().json(json!({ "postID": post_id_str }))
} 

fn do_save_anchor(anchor: &Anchor) -> impl Responder {
    let db = match DbInstance::new_with_str("rocksdb", "./catalystdb", "") {
        Ok(instance) => instance,
        Err(_) => return HttpResponse::InternalServerError().into(),
    };
    if let Err(_) = db_wrapper::initialize(&db) {
        return HttpResponse::InternalServerError().into();
    }

    // Verify link_id meets criteria
    let post_id_len = anchor.post_id.len();
    let link_id_len = anchor.link_id.len();
    if !anchor.link_id.starts_with(&anchor.post_id)
        || post_id_len > link_id_len
    {
        return HttpResponse::BadRequest().body("Invalid link_id format");
    }

    // Check that the referenced post exists
    match db_wrapper::retrieve_post_by_id(&db, &anchor.post_id) {
        Ok(named_rows) => {
            if named_rows.rows.is_empty() {
                return HttpResponse::NotFound().body(format!("Referenced post {} not found", &anchor.post_id));
            }
        }
        Err(_) => return HttpResponse::InternalServerError().into(),
    }

    // Check that the referencing post exists
    match db_wrapper::retrieve_post_by_id(&db, &anchor.referencing_post_id) {
        Ok(named_rows) => {
            if named_rows.rows.is_empty() {
                return HttpResponse::NotFound().body(format!("Referencing post {} not found", &anchor.referencing_post_id));
            }
        }
        Err(_) => return HttpResponse::InternalServerError().into(),
    }
    match db_wrapper::create_anchor(&db, &anchor) {
        Ok(_) => HttpResponse::Ok().json(json!({ "anchorID": anchor.link_id })),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

#[post("/anchor")]
async fn create_anchor(anchor: web::Json<Anchor>) -> impl Responder {
    do_save_anchor(&anchor)
}


#[get("/post/{post_id}")]
async fn get_post_by_id(post_id: web::Path<String>) -> impl Responder {
    let db = match DbInstance::new_with_str("rocksdb", "./catalystdb", "") {
        Ok(instance) => instance,
        Err(_) => return HttpResponse::InternalServerError().into(),
    };

    if let Err(_) = db_wrapper::initialize(&db) {
        return HttpResponse::InternalServerError().into();
    }

    match db_wrapper::retrieve_post_by_id(&db, &post_id) {
        Ok(named_rows) => {
            if let Some(row) = named_rows.rows.into_iter().next() {
                let mut row_iter = row.into_iter();
                let _ = row_iter.next();
                let _ = row_iter.next();
                let title = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                let author = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                let content = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                let signature = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                let post = Post {
                    title,
                    author,
                    content,
                    signature,
                };
                HttpResponse::Ok().json(post)
            } else {
                HttpResponse::NotFound()
                    .body(format!("Post with ID {} not found", post_id))
            }
        }
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}



#[derive(Debug, Deserialize)]
struct PostQuery {
    max_posts: Option<usize>,
    recency_days: Option<u32>,
}

#[get("/posts")]
async fn get_posts(web::Query(query): web::Query<PostQuery>) -> impl Responder {
    let db = match DbInstance::new_with_str("rocksdb", "./catalystdb", "") {
        Ok(instance) => instance,
        Err(mut e) => return HttpResponse::InternalServerError().body(e.insert_str(0, "create: ")).into(),
    };

    if let Err(mut e) = db_wrapper::initialize(&db) {
        return HttpResponse::InternalServerError().body(e.insert_str(0, "init: ")).into();
    }

    let _recency_days = query.recency_days.unwrap_or(7);
    let max_posts = query.max_posts.unwrap_or(10);

    match db_wrapper::get_latest_post_per_author(&db) {
        Ok(named_rows) => {
            let mut posts: HashMap<String, Post> = named_rows
                .rows
                .into_iter()
                .filter_map(|row| {
                    let mut row_iter = row.into_iter();
                    let post_id = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let _ = row_iter.next();
                    let title = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let author = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let content = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let signature = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));

                    Some((post_id, Post {
                        title,
                        author,
                        content,
                        signature,
                    }))
                })
                .collect();

            let limited_posts: HashMap<String, Post> = posts.drain().take(max_posts).collect();
            HttpResponse::Ok().json(limited_posts)
        }
        Err(mut e) => HttpResponse::InternalServerError().body(e.insert_str(0, "query: ")).into(),
    }
}

#[derive(Debug, Deserialize)]
struct ExploreQuery {
    max_posts: Option<usize>,
}

#[get("/explore")]
async fn explore_posts(web::Query(query): web::Query<ExploreQuery>) -> impl Responder {
    let db = match DbInstance::new_with_str("rocksdb", "./catalystdb", "") {
        Ok(instance) => instance,
        Err(_) => return HttpResponse::InternalServerError().into(),
    };

    if let Err(_) = db_wrapper::initialize(&db) {
        return HttpResponse::InternalServerError().into();
    }

    let max_posts = query.max_posts.unwrap_or(10);
    let max_results = max_posts as u8;

    match db_wrapper::get_most_diverse_posts(&db, max_results) {
        Ok(named_rows) => {
            let posts: HashMap<String, Post> = named_rows
                .rows
                .into_iter()
                .filter_map(|row| {
                    let mut row_iter = row.into_iter();
                    let post_id = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let _ = row_iter.next();
                    let title = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let author = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let content = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let signature = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    
                    Some((post_id, Post {
                        title,
                        author,
                        content,
                        signature,
                    }))
                })
                .collect();
            HttpResponse::Ok().json(posts)
        }
        Err(_) => HttpResponse::InternalServerError().into(),
    }
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
                    let link_id = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let validity = row_iter.next().unwrap();
                    let post_id = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let reference = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let referencing_post_id = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
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
                    let new_post_id = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let validity = row_iter.next().unwrap();
                    let title = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let author = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let content = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
                    let signature = serde_json::from_value(row_iter.next().unwrap().into()).unwrap_or(String::from("INVALID CONTENT"));
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
        let filename = current_exe.file_name().unwrap().to_string_lossy();
        if let Ok(mutex) = BIN.lock() {
            if let Some(buffer) = mutex.as_ref() {
                return HttpResponse::Ok()
                .content_type("application/octet-stream")
                .append_header(ContentDisposition::attachment(filename))
                .body(buffer.clone());
            }
        }
        let mut file = fs::File::open(&current_exe).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        let mut mutex = TAR_GZ.lock().unwrap();
        *mutex = Some(buffer.clone());
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
    let many_to_many_signaling = {
        let connections = many_to_many::Connections::default();
        let connections = warp::any().map(move || connections.clone());

        let sessions = many_to_many::Sessions::default();
        let sessions = warp::any().map(move || sessions.clone());

        warp::path("many-to-many")
            .and(warp::ws())
            .and(connections)
            .and(sessions)
            .map(|ws: warp::ws::Ws, connections, sessions| {
                ws.on_upgrade(move |socket| {
                    many_to_many::user_connected(socket, connections, sessions)
                })
            })
    };

    // Spawn the warp server in the background
    let warp_server = warp::serve(many_to_many_signaling);
    let warp_addr = SocketAddr::from_str("0.0.0.0:9001").expect("invalid IP address provided");
    tokio::spawn(async move {
        warp_server.run(warp_addr).await;
    });

    // Start the actix-web server
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
            .service(explore_posts)
            .service(Files::new("/", ".").index_file("index.html"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

