extern crate ed25519_dalek;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use ed25519_dalek::{Signer, Verifier, Signature, VerifyingKey};
use rand_chacha::{ChaCha20Rng};
use rand_chacha::rand_core::{SeedableRng, RngCore};
use wasm_bindgen::prelude::*;
use wasm_peers::{ConnectionType, SessionId, UserId};
use wasm_peers::many_to_many::NetworkManager;
use crate::db_wrapper::*;

/* considered using wee_alloc, but our binary size is already in MBs...
extern crate wee_alloc;

// Use `wee_alloc` as the global allocator 
// - something like halving our wasm size in this case.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;
 */

pub mod db_wrapper {
    pub use cozo::*;
    use std::{collections::BTreeMap};
    use serde_json::json;
    use serde::{Serialize, Deserialize};

    // Define a struct for the anchor data
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Anchor {
        pub link_id: String,
        pub post_id: String,
        pub reference: String,
        pub referencing_post_id: String,
    }

    impl Anchor {
        fn from_named_rows(named_row: NamedRows) -> Vec<Self> {
            unimplemented!()
        }
    }

    // Define a struct for the post data
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Post {
        pub title: String,
        pub author: String,
        pub content: String,
        pub signature: String,
    }

    impl Post {
        fn from_named_rows(named_row: NamedRows) -> Vec<Self> {
            unimplemented!()
        }
    }

    impl Default for Post {
        fn default() -> Self {
            Post {
                title: "Untitled".to_string(),
                author: "No Author".to_string(),
                content: "There are no posts present, this is the default post text.".to_string(),
                signature: "".to_string(),
            }
        }
    }

    pub fn initialize(db: &DbInstance) -> Result<(), String> {
        let script = r#"
        {
            :create post {
                post_id: String,
                at: Validity,
                =>
                title: String,
                author: String,
                content: String,
                signature: Bytes
            }
        }
        {
            :create anchor {
                link_id: String,
                at: Validity,
                =>
                post_id: String, 
                reference: String, 
                referencing_post_id: String
            }
        }
        "#;
        let _ = db.run_script(script, BTreeMap::new());
            // we ignore eval::stored_relation_conflict because 
            //it's fine if the relations already exist
        Ok(())
    }

    pub fn create_post(
        db: &DbInstance,
        post_id: &str,
        post: &Post
    ) -> Result<NamedRows, String> {
        let script = r#"
    ?[post_id, at, title, author, content, signature] <- [[$post_id, 'ASSERT', $title, $author, $content, $signature]]
    :put post {post_id, at => title, author, content, signature}
    "#;
        let mut params = BTreeMap::new();
        params.insert("post_id".to_string(), DataValue::Str(post_id.into()));
        params.insert("title".to_string(), DataValue::Str(post.title.clone().into()));
        params.insert("author".to_string(), DataValue::Str(post.author.clone().into()));
        params.insert("content".to_string(), DataValue::Str(post.content.clone().into()));
        params.insert("signature".to_string(), DataValue::Str(post.signature.clone().into()));

        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    pub fn create_anchor(
        db: &DbInstance,
        anchor: &Anchor
    ) -> Result<NamedRows, String> {
        let script = r#"
    ?[link_id, at, post_id, reference, referencing_post_id]  <- [[$link_id, 'ASSERT', $post_id, $reference, $referencing_post_id]]
    :put anchor {link_id, at => post_id, reference, referencing_post_id}
    "#;
        let mut params = BTreeMap::new();
        params.insert("link_id".to_string(), DataValue::Str(anchor.link_id.clone().into()));
        params.insert("post_id".to_string(), DataValue::Str(anchor.post_id.clone().into()));
        params.insert("reference".to_string(), DataValue::Str(anchor.reference.clone().into()));
        params.insert("referencing_post_id".to_string(), DataValue::Str(anchor.referencing_post_id.clone().into()));

        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    pub fn retrieve_post_by_id(db: &DbInstance, post_id: &str) -> Result<NamedRows, String> {
        let script = "?[post_id, at, title, author, content, signature] := *post[post_id, at, title, author, content, signature], post_id = $post_id";
        let mut params = BTreeMap::new();
        params.insert("post_id".to_string(), post_id.into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    pub fn remove_post_by_id(db: &DbInstance, post_id: &str) -> Result<NamedRows, String> {
        let script = "?[post_id, at, title, author, content, signature] := *post[post_id, at, title, author, content, signature], post_id = $post_id";
        let mut params = BTreeMap::new();
        params.insert("post_id".to_string(), json!(post_id).into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }
    
    pub fn retrieve_posts_by_ids(db: &DbInstance, post_ids: Vec<String>) -> Result<NamedRows, String> {
        let script = "?[post_id, at, title, author, content, signature] := *post[post_id, at, title, author, content, signature], post_id in $post_ids";
        let mut params = BTreeMap::new();
        params.insert("post_ids".to_string(), json!(post_ids).into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }


    pub fn retrieve_anchor_by_id(db: &DbInstance, link_id: &str) -> Result<NamedRows, String> {
        let script = "?[link_id, at, post_id, reference, referencing_post_id] := *anchor[link_id, at, post_id, reference, referencing_post_id], link_id = $link_id";
        let mut params = BTreeMap::new();
        params.insert("link_id".to_string(), json!(link_id).into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    pub fn retrieve_anchors_by_post_id(db: &DbInstance, link_id: &str) -> Result<NamedRows, String> {
        let script = "?[link_id, at, post_id, reference, referencing_post_id] := *anchor[link_id, at, post_id, reference, referencing_post_id], post_id = $post_id";
        let mut params = BTreeMap::new();
        params.insert("post_id".to_string(), link_id.into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    pub fn retrieve_anchors_by_ids(db: &DbInstance, link_ids: Vec<String>) -> Result<NamedRows, String> {
        let script = "?[link_id, at, post_id, reference, referencing_post_id] := *anchor[link_id, at, post_id, reference, referencing_post_id], link_id in $link_ids";
        let mut params = BTreeMap::new();
        params.insert("link_ids".to_string(), json!(link_ids).into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    pub fn get_related_posts_by_same_author(
        db: &DbInstance,
        target_post_id: &str,
    ) -> Result<NamedRows, String> {
        let script = r#"
        ?[related_post_id, at, title, author, content, signature] :=
        *post[target_post_id, _, _, target_author, _, _],
        *anchor[_, _, target_post_id, _, related_post_id],
        *post[related_post_id, at, title, author, content, signature],
        target_author = author,
        target_post_id = $target_post_id
    "#;
    
        let mut params = BTreeMap::new();
        params.insert("target_post_id".to_string(), target_post_id.into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    pub fn get_related_posts_excluding_same_author(
        db: &DbInstance,
        target_post_id: &str,
    ) -> Result<NamedRows, String> {
        let script = r#"
        ?[post_id, at, title, author, content, signature] := 
        *post[post_id, at, title, author, content, signature],
        *anchor[link_id, target_post_id, reference, post_id],
        *post[target_post_id, _, _, not_author, _, _],
        author != not_author,
        target_post_id = $target_post_id
        "#;
    
        let mut params = BTreeMap::new();
        params.insert("target_post_id".to_string(), target_post_id.into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    pub fn get_related_authors(
        db: &DbInstance,
        author: &str
    ) -> Result<NamedRows, String> {
        let script = r#"
        referencing[given_author, other_author] := *post[post_id, _, _, other_author, _, _],
            *anchor[_, _, target_post_id, _, post_id],
            *post[target_post_id, _, _, given_author, _, _]
        referenced[given_author, other_author] := *post[post_id, _, _, given_author, _, _],
            *anchor[_, _, target_post_id, _, post_id],
            *post[target_post_id, _, _, other_author, _, _]
        ?[other_author] :=
            referencing[given_author, other_author] or
            referenced[given_author, other_author],
            given_author = $given_author,
            other_author != given_author
        "#;

        let mut params = BTreeMap::new();
        params.insert("given_author".to_string(), author.into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    pub fn get_ancestor_of_post(
        db: &DbInstance,
        post_id: &str,
    ) -> Result<NamedRows, String> {
        let script = r#"
            ancestor[post_id, oldest_post_id] :=
                *post[post_id, _, _, _, _, _],
                *anchor[link_id, _, target_post_id, _, post_id],
                *post[target_post_id, _, _, _, _, _],
                oldest_post_id = target_post_id
            ancestor[post_id, oldest_post_id] :=
                *post[post_id, _, _, _, _, _],
                *anchor[link_id, _, target_post_id, _, post_id],
                *post[target_post_id, _, _, _, _, _],
                ancestor[target_post_id, older_post_id],
                oldest_post_id = older_post_id
            ?[given_post_id, oldest_ancestor] :=
                given_post_id = $post_id,
                ancestor[given_post_id, oldest_ancestor]
        "#;
    
        let mut params = BTreeMap::new();
        params.insert("post_id".to_string(), post_id.into());
    
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }

    

    pub fn get_latest_post_per_author(
        db: &DbInstance,
    ) -> Result<NamedRows, String> {
        let script = r#"
        {:create _temp_posts {post_id, at, title, author, content, signature}}
        {
            ?[author] := *post[_, _, _, author, _, _],
            :replace _temp_authors {author}
        }
        %loop
            %if { len_a[count(x)] := *_temp_authors[x]; ?[x] := len_a[z], x = z <= 0 }
                %then %return _temp_posts
            %end
            {
                ?[author] := *_temp_authors[author]
                :limit 1
                :replace unique_author {author}
            }
            {
                ?[post_id, at, title, author, content, signature] :=
                    *post[post_id, at, title, author, content, signature],
                    *unique_author[author],
                :sort -at
                :limit 1
                :put _temp_posts {post_id, at, title, author, content, signature}
            }
            {
                ?[author] := *unique_author[author],
                :rm _temp_authors {author}
            }
        %end"#;
        let params = BTreeMap::new();
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
         })
    }


    pub fn get_most_diverse_posts(
        db: &DbInstance,
        max_results: u8,
    ) -> Result<NamedRows, String> {
        let script = r#"
        {
            author_diversity[post_id, count(author)] :=
                *post[post_id, at, title, author, _, _],
                *anchor[link_id, _, target_post_id, _, post_id],
                *post[target_post_id, _, _, not_author, _, _],
                author != not_author
            ?[post_id, diversity] := author_diversity[post_id, diversity]
            :replace _author_diversity {post_id, diversity}
        }
        {
            ?[post_id, diversity] :=
                *post[post_id, at, _, author, _, _],
                *anchor[link_id, _, target_post_id, _, post_id],
                *post[target_post_id, _, _, _, _, _],
                at_med_idx = to_int(length(list(at))/2),
                diversity = (max(to_int(at)) - to_int(get(list(at), at_med_idx))) / (to_int(now()) - max(to_int(at)))
            :replace _timescale_diversity {post_id, diversity}
        }
        {
            bridge_coefficient[post_id, count(orphaned_post_id)] :=
                *post[post_id, _, _, _, _, _],
                *post[not_post_id, _, _, _, _, _],
                *anchor[_, _, orphaned_post_id, _, post_id],
                *anchor[_, _, post_id, _, orphaned_post_id],
                not *anchor[_, _, not_post_id, _, orphaned_post_id],
                not *anchor[_, _, orphaned_post_id, _, not_post_id],
                post_id != not_post_id
            ?[post_id, coefficient] := bridge_coefficient[post_id, coefficient]
            :replace _bridge_coefficient {post_id, coefficient}
        }
        {
            def_score[post_id, default] := *post[post_id, _, _, _, _, _], default = rand_float()
            ?[post_id, final_score] :=
                *_author_diversity[post_id, author_div] or def_score[post_id, author_div],
                *_timescale_diversity[post_id, timescale_div] or def_score[post_id, timescale_div],
                *_bridge_coefficient[post_id, bridge_coeff] or def_score[post_id, bridge_coeff],
                epsilon = 1,
                final_score = (author_div + (bridge_coeff * author_div)) / (epsilon - timescale_div)
            :sort -final_score
            :replace latest_diversity_score {post_id, final_score}
            :limit $max_results
        }
        {
                ?[post_id, at, title, author, content, signature] :=
                *post[post_id, at, title, author, content, signature],
                *latest_diversity_score[post_id, final_score]
        }
        "#;
        let mut params = BTreeMap::new();
        params.insert("max_results".to_string(), json!(max_results).into());
        db.run_script(script, params).map_err(|e|{ 
            println!("{:?}",e);
            e.to_string()
        })
    }
}

#[wasm_bindgen]
pub struct Keypair {
    bytes: [u8; 32],
}

#[wasm_bindgen]
impl Keypair {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Keypair, JsValue> {
        let mut seed: [u8; 32] = [0u8; 32];
        if let Ok(_) = getrandom::getrandom(&mut seed){
            Ok(Keypair { bytes: seed })
        } else {
            // rng failed, probably bad environment
            Err(JsValue::from_str("keypair seed generation failed."))
        }
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let mut rng = ChaCha20Rng::from_seed(self.bytes);
        let mut secret_key: [u8; 32] = [0; 32];
        rng.fill_bytes(&mut secret_key);
        let keypair = ed25519_dalek::SigningKey::from_bytes(&secret_key);
        keypair.sign(message).to_bytes().to_vec()
    }

    pub fn public_key_bytes(&self) -> Vec<u8> {
        let mut rng = ChaCha20Rng::from_seed(self.bytes);
        let mut secret_key: [u8; 32] = [0; 32];
        rng.fill_bytes(&mut secret_key);
        ed25519_dalek::SigningKey::from_bytes(&secret_key).verifying_key().as_bytes().to_vec()
    }

    pub fn seed_bytes(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }

    pub fn from_seed(seed_bytes: &[u8]) -> Self {
        let mut seed: [u8; 32] = [0; 32];
        seed.clone_from_slice(&seed_bytes[..32]);
        Keypair { bytes: seed }
    }


    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let mut rng = ChaCha20Rng::from_seed(self.bytes);
        let mut secret_key: [u8; 32] = [0; 32];
        rng.fill_bytes(&mut secret_key);
        let mut sig: [u8; 64] = [0; 64];
        sig.clone_from_slice(&signature[..64]);
        ed25519_dalek::SigningKey::from_bytes(&secret_key).verifying_key()
            .verify(message, &Signature::from_bytes(&sig))
            .is_ok()
    }

    pub fn verify_with_key(pubkey_bytes: &[u8], message: &[u8], signature: &[u8]) -> bool {
        let mut pubkey_real_bytes: [u8; 32] = [0; 32];
        pubkey_real_bytes.clone_from_slice(&pubkey_bytes[..32]);
        if let Ok(pubkey) = VerifyingKey::from_bytes(&pubkey_real_bytes){
            let mut sig: [u8; 64] = [0; 64];
        sig.clone_from_slice(&signature[..64]);
        pubkey
            .verify(message, &Signature::from_bytes(&sig))
            .is_ok()
        } else {
            false
        }
    }
}


#[wasm_bindgen]
pub struct AppState {
    inner: Rc<RefCell<InnerAppState>>,
}

struct InnerAppState {
    db: DbInstance,
    network: Option<Messenger>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}


impl AppCallbacks for AppState {
    fn on_open_callback(&self, manager: &NetworkManager, user_id: &UserId) {
        
    }

    fn on_message_callback(&self, manager: &NetworkManager, user_id: &UserId, message: &String) {

    }
}

#[wasm_bindgen]
impl AppState {

    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<AppState, JsValue> {
        let db_instance = DbInstance::new("mem", "", "").unwrap();
        let network_option = Messenger::new("".to_string(), "main_sync".to_string()).ok();
        db_wrapper::initialize(&db_instance)
            .map(|_|{
                AppState {
                    inner: Rc::new(RefCell::new(
                        InnerAppState { 
                        db: db_instance,
                        network: network_option
                    }))
                }
            })
            .map_err(|e| serde_wasm_bindgen::to_value(&e).unwrap())
    }

    #[wasm_bindgen]
    pub fn start_networking(&self){
        let mut inner = self.inner.borrow_mut();
        if let Some(mut network) = inner.network.as_mut() {
            if !network.is_open {
                let app_callbacks= Arc::new(self.clone());
                network.register_callbacks(app_callbacks);
                network.open_peer();
            }
        }
    }

    // we can't control if the use messes up their own local db anyway.
    // so let's build the superpowers into the app.
    #[wasm_bindgen]
    pub fn script_raw(&self, script: &str) -> Result<JsValue, JsValue> {
        self.inner.borrow().db
            .run_script(script, Default::default())
            .map(|result| serde_wasm_bindgen::to_value(&result).unwrap())
            .map_err(|e| serde_wasm_bindgen::to_value(&e.root_cause().to_string()).unwrap())
    }

    #[wasm_bindgen]
    pub fn import(&self, data_json: &str) -> Result<JsValue, JsValue> {
        serde_json::from_str(data_json)
            .map_err(|_| JsValue::from_str("db import failed: invalid json"))
            .and_then(|data| {
                self.inner.borrow().db
                    .import_relations(data)
                    .map(|_| JsValue::from_bool(true))
                    .map_err(|e| JsValue::from_str(&e.root_cause().to_string()))
            })
    }

}


#[wasm_bindgen]
pub struct Messenger {
    self_peer: NetworkManager,
    room_name: SessionId,
    signaling_server: String,
    stun_server: String,
    on_message: Option<Rc<RefCell<Box<dyn FnMut(&NetworkManager, &UserId, &String) -> ()>>>>,
    on_open: Option<Rc<RefCell<Box<dyn FnMut(&NetworkManager, &UserId) -> ()>>>>,
    is_open: bool
}

pub trait AppCallbacks {
    fn on_open_callback(&self, manager: &NetworkManager, user_id: &UserId);
    fn on_message_callback(&self, manager: &NetworkManager, user_id: &UserId, message: &String);
}

impl Messenger {
    pub fn universal_on_open(network: &NetworkManager, user: &UserId) {
        
    }

    pub fn universal_on_message(network: &NetworkManager, user: &UserId, message: &String) {
        
    }

    pub fn register_callbacks(&mut self, app: Arc<dyn AppCallbacks>) {
        let on_open = {
            let app = app.clone();
            Rc::new(RefCell::new(Box::new(move |manager: &NetworkManager, user_id: &UserId| {
                app.on_open_callback(manager, user_id)
            }) as Box<dyn FnMut(&NetworkManager, &UserId)>))
        };

        let on_message = {
            let app = app.clone();
            Rc::new(RefCell::new(Box::new(move |manager: &NetworkManager, user_id: &UserId, message: &String| {
                app.on_message_callback(manager, user_id, message)
            }) as Box<dyn FnMut(&NetworkManager, &UserId, &String)>))
        };

        self.on_open = Some(on_open);
        self.on_message = Some(on_message);
    }
}

#[wasm_bindgen]
impl Messenger {
    #[wasm_bindgen(constructor)]
    pub fn new(stun_serv_url: String, room_name: String) -> Result<Messenger, JsValue> {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let location = document.location().expect("document should have a location");
        let hostname = location.hostname().expect("page should have hostname");
        let signal_serv_url = hostname + ":9001";
        if let Ok(nm) =  NetworkManager::new(
            signal_serv_url.as_str(), 
            SessionId::new(room_name.clone()), 
            ConnectionType::Stun { urls: stun_serv_url.clone() }
        ) {
            Ok(Messenger { 
                self_peer: nm, 
                room_name: SessionId::new(room_name), 
                signaling_server: signal_serv_url, 
                stun_server: stun_serv_url,
                on_message: None,
                on_open: None,
                is_open: false
            })
        } else {
            Err(JsValue::from_str("peer creation failed"))
        }
    }

    #[wasm_bindgen]
    pub fn foreign_connection(signal_serv_url: String, stun_serv_url: String, room_name: String) -> Result<Messenger, JsValue> {
        if let Ok(nm) =  NetworkManager::new(
            signal_serv_url.as_str(), 
            SessionId::new(room_name.clone()), 
            ConnectionType::Stun { urls: stun_serv_url.clone() }
        ) {
            Ok(Messenger { 
                self_peer: nm, 
                room_name: SessionId::new(room_name), 
                signaling_server: signal_serv_url, 
                stun_server: stun_serv_url,
                on_message: None,
                on_open: None,
                is_open: false
            })
        } else {
            Err(JsValue::from_str("peer creation failed"))
        }
    }


    #[wasm_bindgen]
    pub fn open_peer(&mut self){
        let on_open = self.on_open.as_ref().map(|cb| cb.clone());
        let on_message = self.on_message.as_ref().map(|cb| cb.clone());
        let network_join = self.self_peer.clone();
        let network_message = self.self_peer.clone();
        self.self_peer.start(
            move |user_id| {
                if let Some(cb) = &on_open {
                    Self::universal_on_open(&network_join.clone(), &user_id);
                    (cb.borrow_mut())(&network_join.clone(), &user_id);
                }
            },
            move |user_id, message| {
                if let Some(cb) = &on_message {
                    Self::universal_on_message(&network_message.clone(), &user_id, &message);
                    (cb.borrow_mut())(&network_message.clone(), &user_id, &message);
                }
            }
        );
        self.is_open = true;
    }
}
