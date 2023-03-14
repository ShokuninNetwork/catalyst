use actix_web::{get, post, web, HttpResponse, Responder, HttpServer, App, middleware, http::header::{ContentDisposition, ContentEncoding}};
use base64::{engine::general_purpose, Engine};
use rand::{seq::SliceRandom, thread_rng};
use ring::{
    digest::{self, SHA256},
    signature::{self, UnparsedPublicKey},
};
use serde::{Deserialize, Serialize};
use serde_json::{json};
use std::{fs, path::PathBuf, io::Read, time::{SystemTime, Duration}, collections::HashMap, process::Output, sync::{Arc, Mutex}};
use std::process::Command;
use lazy_static::lazy_static;


lazy_static! {
    static ref TAR_GZ: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
}

#[get("/")]
async fn index_page() -> impl Responder {
    HttpResponse::Ok().body(
        r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
        <meta charset="UTF-8">
        <title>Catalyst</title>
        <link rel="icon" href="/icon.svg"/>
        <link rel="manifest" href="/manifest.json"/>
            <style>
            body {
                background-image: url( '/icon.svg' ), url( '/icon_text.svg' ), linear-gradient(to bottom right, white, rgba(23, 11, 40, 0.3));
                background-position: top left, 12% 2%, top left;
                background-size: 8%, 20%, 130vw 130vh;
                background-repeat: no-repeat, no-repeat, no-repeat;
                overflow-x: hidden;
                overflow-y: hidden;
                font-family: "URW Gothic", sans-serif;
            }

            /* Define a style for the post container */
            .post-container {
                position: absolute;
                width: 100%;
                height: 100%;
                display: flex;
                gap: 10px;
                padding: 10px;
                transition: width 0.3s ease-in-out;
                align-items: center;
                flex-direction: column;
                justify-content: flex-start;
                overflow-x: hidden;
                overflow-y: auto;
                font-family: inherit;
            }

            .app-container {
                overflow-x: hidden;
                overflow-y: hidden;
            }


            /* Define a style for the posts */
            .post {
                font-size: 1.5em;
                background-color: #ffffff;
                border: 1px solid #dddddd;
                border-radius: 5px;
                box-shadow: 0px 2px 4px rgba(23, 11, 40, 0.3);
                padding: 10px;
                width: calc(85% - 10px);
                font-family: inherit;
            }

            /* Define a style for the post title */
            .post-title {
                font-size: 2em;
                font-weight: bold;
                margin-bottom: 10px;
            }

            /* Define a style for the post author */
            .post-author {
                font-size: 1em;
                color: #442178;
                margin-bottom: 10px;
            }

            /* Define a style for the post content */
            .post-content {
                line-height: 2.5;
                color: #170b28;
            }

            @media (min-width: 1080px) {
            .right-side {
                    position: absolute;
                    top: 0;
                    width: 50vw;
                    height: 100vh;
                    background-color: rgba(23, 11, 40, 0.5);
                    display: flex;
                    flex-direction: column;
                    justify-content: center;
                    align-items: center;
                    z-index: 999;
                }
                .hidden {
                    transform: translateX(150%);
                    visibility: hidden;
                }
                .shown {
                    visibility: visible;
                    transform: translateX(100%);
                }
                /* Define a style for the secondary button */
                button {
                    display: inline-block;
                    padding: 5px 10px;
                    border: 1px solid #8d5fd3;
                    border-radius: 0.5em;
                    font-size: 1em;
                    font-weight: bold;
                    text-align: center;
                    text-decoration: none;
                    background-color: #ffffff;
                    color: #8d5fd3;
                    cursor: pointer;
                    z-index: 999;
                    transition: background-color 0.2s ease-in-out;
                }
            }

            @media (max-width: 1079px) {
                .right-side {
                        position: absolute;
                        top: 0;
                        left: 0;
                        width: 100%;
                        height: 100%;
                        background-color: rgba(23, 11, 40, 0.5);
                        display: flex;
                        flex-direction: column;
                        justify-content: center;
                        align-items: center;
                        z-index: 999;
                    }
                    .hidden {
                        visibility: hidden;
                    }
                    .shown {
                        visibility: visible;
                    }
                    /* Define a style for the secondary button */
                    button {
                        display: inline-block;
                        padding: 9px 19px;
                        border: 1px solid #8d5fd3;
                        border-radius: 0.8em;
                        font-size: 1.5em;
                        font-weight: bold;
                        text-align: center;
                        text-decoration: none;
                        background-color: #ffffff;
                        color: #8d5fd3;
                        cursor: pointer;
                        z-index: 999;
                        transition: background-color 0.2s ease-in-out;
                    }
                }
          
            /* Define a style for the post editor textarea */
            .post-editor #editor-content {
              width: calc(100% - 2em);
              height: 95%;
              padding: 1em;
              border: none;
              border-radius: 0.5em;
              background-color: #ffffff;
              font-size: 1.5em;
              line-height: 1.5;
            }
          
            /* Define a style for the secondary button when hovered */
            button:hover {
              background-color: #c6afe9;
            }
            
            </style>
            <meta name="viewport" width="device-width" initial-scale="1" interactive-widget="resizes-content">
        </head>
        <body>
        <div class="app-container">
            <div class="post-container">
                <button class="toggle-button">Toggle Post Editor</button>
                <div id="posts-start-marker"></div>
            </div>
            <div class="post-editor right-side hidden">
                <div>
                    <textarea placeholder="Post title" id="editor-title"></textarea>
                    <textarea placeholder="Author name" id="editor-author"></textarea>
                    <button id="sign-button">Sign</button>
                    <button class="toggle-button">Hide</button>
                    <button class="save-button toggle-button">Post</button>
                    <button class="preview-button">Preview</button>
                </div>
                <textarea placeholder="Write your post here" id="editor-content"></textarea>
            </div>
            <div class="post-previewer right-side hidden">
                <button class="preview-button">Close Preview</button>
                <div class="post">
                    <div class="post-title"> Post Preview </div>
                    <div class="post-author"> You </div>
                    <div class="post-content preview-post"></div>
                </div>
            </div>
        </div>
        </body>
            <script>

                const editorContainer = document.querySelector('.post-editor');
                const postEditor = document.querySelector('.post-editor #editor-content');
                const previewContainer = document.querySelector('.post-previewer');
                const previewPost = document.querySelector('.preview-post');
                const postContainer = document.querySelector('.post-container');
                const isWideWindow = window.matchMedia('(min-width: 1080px)');

                // Get the user preferences from localStorage or set defaults
                const storedPreferences = localStorage.getItem('userPreferences');
                const userPrefs = storedPreferences ? JSON.parse(storedPreferences) : {
                    maxPosts: 10,
                    recencyDays: 30
                };

                localStorage.setItem('userPreferences', JSON.stringify(userPrefs));


                // Helper function to get the ID of the parent post element
                function getPostID(element) {
                    while (element && !element.classList.contains('post')) {
                        element = element.parentElement;
                    }
                    return element ? element.id : null;
                }



                function postRenderer(postContentElement, postContent) {
                    postContentElement.setHTML
                      ? postContentElement.setHTML(postContent)
                      : /<\/?[a-z][\s\S]*>/i.test(postContent)
                        ? ((postContentElement.innerHTML = "<b><a href='https://developer.mozilla.org/en-US/docs/Web/API/HTML_Sanitizer_API#browser_compatibility' target='_blank'>unsupported browser</a>, rendering in text mode: </b><br/>"), postContentElement.appendChild(document.createTextNode(postContent)))
                        : (postContentElement.innerHTML = postContent);
                }

                function postConstructor(postObject) {
                    const post = document.createElement('div');
                    const title = document.createElement('div');
                    const author = document.createElement('div');
                    const content = document.createElement('div');
                    post.classList.add('post');
                    title.classList.add('post-title');
                    author.classList.add('post-author');
                    content.classList.add('post-content');
                    postRenderer(title, postObject.title);
                    postRenderer(author, postObject.author);
                    postRenderer(content, postObject.content);
                    post.appendChild(title);
                    post.appendChild(author);
                    post.appendChild(content);
                    post.id = postObject.postID;
                    post.signature = postObject.signature
                    return post;
                }

                async function appendPost(postID) {
                    const postResponse = await fetch(`/post/${postID}`);
                    if (!postResponse.ok) {
                      console.error(`Failed to load post ${postID}`);
                      return;
                    }

                    const postObject = await postResponse.json();
                    postObject.postID = postID;
                    const post = postConstructor(postObject);
                    const postsStartMarker = document.getElementById('posts-start-marker');
                    postContainer.insertBefore(post, postsStartMarker.nextSibling);

                    // Check if the number of posts in the post container has exceeded the maxPosts limit
                    const maxPosts = userPrefs.maxPosts || 10;
                    if (postContainer.children.length > maxPosts+2) {
                      // Remove the last post from the container
                      postContainer.removeChild(postContainer.lastChild);
                    }
                }

                async function getUnfilteredAnchors(postId) {
                    const response = await fetch(`/post/${postId}/unfiltered_anchors`);
                    if (!response.ok) {
                      throw new Error(`Failed to fetch unfiltered anchors for post ${postId}: ${response.status} ${response.statusText}`);
                    }
                    const anchorJson = await response.json();
                    return anchorJson.map(anchor => ({
                      linkId: anchor.link_id,
                      postId: anchor.post_id,
                      reference: anchor.reference,
                      referencingPostId: anchor.referencing_post_id,
                    }));
                  }
                  
                  async function getFilteredAnchors(postId) {
                    const response = await fetch(`/post/${postId}/anchors?max_posts=${userPrefs.maxPosts}&recency_days=${userPrefs.recencyDays}`);
                    if (!response.ok) {
                      throw new Error(`Failed to fetch filtered anchors for post ${postId}: ${response.status} ${response.statusText}`);
                    }
                    const anchorJson = await response.json();
                    return anchorJson.map(anchor => ({
                      linkId: anchor.link_id,
                      postId: anchor.post_id,
                      reference: anchor.reference,
                      referencingPostId: anchor.referencing_post_id,
                    }));
                  }

                  function insertTextAtCursor(textarea, text) {
                    // Get the current cursor position
                    const startPos = textarea.selectionStart;
                    const endPos = textarea.selectionEnd;
                  
                    // Insert the text at the current cursor position
                    textarea.value = textarea.value.substring(0, startPos) + text + textarea.value.substring(endPos);
                  
                    // Set the new cursor position
                    textarea.selectionStart = startPos + text.length;
                    textarea.selectionEnd = startPos + text.length;
                  
                    // Set the focus back to the textarea
                    textarea.focus();
                  }                 

                // Define a function to load posts from the backend API
                async function loadPosts() {
                    // Set the query parameters based on the user preferences
                    const queryParams = `?max_posts=${userPrefs.maxPosts}&recency_days=${userPrefs.recencyDays}`;

                    // Fetch the list of posts from the backend API
                    const response = await fetch(`/posts${queryParams}`);
                    const posts = await response.json();
                    // Get an array of post IDs
                    const postIDs = Object.keys(posts);  

                    // Loop over the posts and create elements for each one
                    postIDs.forEach(postID => {
                        // add the ID to the post object.
                        post = posts[postID];
                        post.postID = postID;
                        // Create a div element for the post
                        const postDiv = postConstructor(post);

                        // Add the post div to the post container
                        postContainer.appendChild(postDiv);
                    });
                }

                // Call the function to load the posts into the post container
                loadPosts();

                // Add a button element to start creating a transformation
                        postContainer.addEventListener('mouseup', event => {
                        const postID = getPostID(event.target);
                        const selection = window.getSelection();
                        const selectedText = selection.toString().trim();
                            if (selectedText) {
                                // Copy selection to editor
                                const postID = getPostID(event.target);
                                const linkID = `${postID}${Math.random().toString(36).substring(8)}`; // Generate a unique ID for the link
                                const link = document.createElement('a');
                                link.href = `#${linkID}`;
                                link.innerText = selectedText;
                                insertTextAtCursor(postEditor, link.outerHTML);
                                // Save the link ID and start/end positions of the selected text
                                const postDiv = document.getElementById(postID);
                                const start = postDiv.innerText.indexOf(selectedText);
                                const end = start + selectedText.length;

                                //Anchor Format: link_id, post_id, reference, referencing_post_id
                                //Reference Format: (post_start, post_end, ref_start, ref_end)
                                // we have link_id, post_id, (post_start, post_end) at this point in time
                                // we will only have (ref_start, ref_end) and referencing_post_id once the
                                // referencing post has been saved.
                                postEditor.temp = postEditor.temp ? postEditor.temp : {};
                                postEditor.temp.pendingAnchors =
                                    postEditor.temp.pendingAnchors ?
                                    postEditor.temp.pendingAnchors :
                                    [];
                                postEditor.temp.pendingAnchors.push({
                                    link_id: linkID,
                                    post_id: postID,
                                    post_start: start,
                                    post_end: end,
                                });
                            }
                        });


                  document.addEventListener('click', () => {
                    const isToggleBtn = event.target.classList.contains('toggle-button');
                    const isPreviewBtn = event.target.classList.contains('preview-button');

                  
                    if (isToggleBtn || isPreviewBtn) {
                      const container = isToggleBtn ? editorContainer : previewContainer;
                      const isHidden = container.classList.contains('hidden');
                  
                      if (isHidden) {
                        container.classList.remove('hidden');
                        container.classList.add('shown');
                        if (isWideWindow.matches){
                            postContainer.style.width = '50%'
                        }
                        if (!isToggleBtn) {
                          const previewContent = postEditor.value;
                          postRenderer(previewPost, previewContent);
                        }
                      } else {
                        container.classList.add('hidden');
                        container.classList.remove('shown');
                        if (!isPreviewBtn) {
                            postContainer.style.width = '100%'
                        }
                      }
                    }
                  });
                

                  // Get a reference to the save button
                  const saveButton = document.querySelector('.save-button');
                  
                  // Add a click event listener to the save button
                  saveButton.addEventListener('click', async () => {
                    const postTitle = document.querySelector('.post-editor #editor-title');
                    const postAuthor = document.querySelector('.post-editor #editor-author');
                    const transformedText = postEditor.value;
                    const transformedTitle = postTitle.value;
                    const transformedAuthor = postAuthor.value;
                    // Create a new post object with the transformed text
                    const newPost = {
                      title: transformedTitle,
                      author: transformedAuthor,
                      content: transformedText,
                      signature: []
                    };
                  
                    // Send the new post object to the '/post' endpoint
                    const response = await fetch('/post', {
                      method: 'POST',
                      headers: {
                        'Content-Type': 'application/json'
                      },
                      body: JSON.stringify(newPost)
                    });
                  
                    // Get the post ID from the response and log it to the console
                    const data = await response.json();
                    const postID = data.postID;
                    console.log(`New post created with ID: ${postID}`);

                    if(postEditor.temp)if(postEditor.temp.pendingAnchors)postEditor.temp.pendingAnchors.forEach(pendingAnchor => {
                        
                        let postRendered = document.createElement("div");
                        const previewContent = postEditor.value;
                        postRenderer(postRendered, previewContent);
                        let query = "a[href='#"+pendingAnchor.link_id+"']";
                        let reference = postRendered.querySelector(query);
                        let referenceText = reference?reference.innerText:"";
                        let refStart = postRendered.innerText.indexOf(referenceText);
                        let refEnd = refStart + referenceText.length;
                        
                        let newAnchor = {
                            link_id: pendingAnchor.link_id,
                            post_id: pendingAnchor.post_id,
                            reference: pendingAnchor.post_start+":"+pendingAnchor.post_end+":"+refStart+":"+refEnd,
                            referencing_post_id: postID
                        }

                        // Send the new anchor object to the '/anchor' endpoint
                        const responsePromise = fetch('/anchor', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json'
                        },
                        body: JSON.stringify(newAnchor)
                        });

                    });
                  
                    clearPost();
                    appendPost(postID);
                  });


                function clearPost() {
                    const postTitle = document.querySelector('.post-editor #editor-title');
                    const postAuthor = document.querySelector('.post-editor #editor-author');
                    // Clear the post editor
                    postEditor.value = '';
                    postTitle.value = '';
                    postAuthor.value = '';
                    delete postEditor.temp;
                    postEditor.readOnly = false;
                    postAuthor.readOnly = false;
                }

                  // Click handler function for the sign button
                async function signPost() {
                    // Get the post content from a variable in-scope
                    const postAuthor = document.querySelector('.post-editor #editor-author');

                    // Check if there is an existing keypair in localStorage
                    let localStorageKeypair = localStorage.getItem("keypair");
                    let keypair = localStorageKeypair ? 
                    await crypto.subtle.importKey(
                        "jwk", 
                        localStorageKeypair, 
                        { name: "ED25519", namedCurve: "ed25519" },
                        true,
                        ["sign", "verify"]): 
                    await crypto.subtle.generateKey(
                        { name: "ED25519", namedCurve: "ed25519" },
                        true,
                        ["sign", "verify"]);
                    localStorage.setItem("keypair", await crypto.subtle.exportKey("jwk", keypair));

                    // create an "aka" element which is appended to the post,
                    // holding the non-cryptographic author name
                    let aka = document.createElement('div');
                    if(postAuthor.value){
                        aka.classList.add('aka');
                        aka.value = postAuthor.value;
                        postEditor.value += aka.outerHTML
                    }
                    // Replace author with base64 of the pubkey
                    postAuthor.value = btoa(
                        String.fromCharCode(
                            ...new Uint8Array(await crypto.subtle.exportKey("spki", keypair)
                        )
                    );

                    // Sign the post content using the keypair
                    const signature = await crypto.subtle.sign(
                        { name: "ED25519" },
                        keypair.privateKey,
                        new TextEncoder().encode(postEditor.value)
                    );

                    // Store the signature in a variable waiting for post submission
                    if(!postEditor.temp){postEditor.temp = {}}
                    postEditor.temp.signature = new Uint8Array(signature);
                    postEditor.readOnly = true;
                    postAuthor.readOnly = true;
                }
            </script>
        </html>
        "#,
    )
}


#[get("/icon.svg")]
async fn icon() -> impl Responder {
    HttpResponse::Ok()
    .append_header(("Content-Type", "image/svg+xml"))
    .body(
    r####"<?xml version="1.0" encoding="UTF-8"?>
    <!-- Created with Inkscape (http://www.inkscape.org/) -->
    <svg width="512px" height="512px" version="1.1" viewBox="0 0 512 512" xml:space="preserve" xmlns="http://www.w3.org/2000/svg" xmlns:cc="http://creativecommons.org/ns#" xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rect x="52.31" y="47.92" width="397.3" height="402.3" fill="#170b28"/><rect x="73" y="30.34" width="397.3" height="402.3" fill="#1a1a1a"/><g><path transform="translate(59.04 131.9)" d="m263.8-44.7 81.8 143.7 83.53 142.7-165.3-0.9999-165.3 0.9999 83.53-142.7z" fill="#5a2ca0"/><path transform="translate(59.04 131.9)" d="m263.8-44.7 81.8 143.7 83.53 142.7-165.3-0.9999-165.3 0.9999 83.53-142.7z" fill="#170b28"/><path transform="translate(76.97 120.5)" d="m263.8-44.7 81.8 143.7 83.53 142.7-165.3-0.9999-165.3 0.9999 83.53-142.7z" fill="#442178"/><path d="m275.5 338c-0.0538-0.0173-0.3274-0.1023-0.608-0.1888-1.066-0.3286-2.077-1.161-2.565-2.112-0.4389-0.8551-0.5402-2.169-0.2456-3.188 0.2834-0.9798 0.932-1.758 2.005-2.407 0.7019-0.4244 0.7124-0.426 3.664-0.5648 5.202-0.2445 9.093-1.054 14.24-2.961 2.327-0.8628 5.502-2.514 7.974-4.146 1.859-1.228 3.248-2.288 4.745-3.623 2.659-2.37 4.459-4.32 6.538-7.082 0.3316-0.4404 1.975-2.913 1.975-2.972 0-0.0315 0.1613-0.3093 0.3583-0.6174 0.1971-0.3081 0.5766-0.9804 0.8434-1.494 0.2668-0.5136 0.5665-1.088 0.666-1.276 0.3657-0.6922 1.091-2.289 1.26-2.776 0.0961-0.2759 0.333-0.8832 0.5264-1.35 0.1934-0.4664 0.4359-1.149 0.5388-1.517 0.1029-0.3678 0.3452-1.175 0.5384-1.794 0.3615-1.158 1.012-4.251 1.277-6.066 0.256-1.759 0.3768-5.532 0.2631-8.219-0.3447-8.141-3.324-16.52-8.263-23.24-1.423-1.936-2.371-3.043-4.215-4.926-2.32-2.369-4.337-4.035-7.051-5.828-2.437-1.609-5.708-3.313-7.925-4.128-0.4843-0.178-1.183-0.437-1.552-0.5755-0.3695-0.1385-0.9639-0.3334-1.321-0.433-0.357-0.0996-1.177-0.3364-1.823-0.5261-1.186-0.3484-4.183-0.9283-6.164-1.193-0.5919-0.079-2.111-0.1917-3.376-0.2505-3.004-0.1397-3.009-0.1405-3.713-0.566-1.445-0.8736-2.16-2.047-2.182-3.582-0.0275-1.916 1.042-3.425 2.931-4.137 0.3983-0.15 0.7461-0.1737 2.327-0.1588 7.119 0.0668 15.12 2.009 21.72 5.273 3.659 1.809 5.463 2.95 9.727 6.152 0.2509 0.1884 0.6989 0.5727 1.809 1.551 2.729 2.407 5.673 5.63 7.643 8.368 0.3221 0.4475 0.7254 1.005 0.8965 1.239 1.048 1.433 2.562 4.061 3.861 6.699l0.9853 2.001 1.092-2.196c1.898-3.818 4.058-7.22 6.178-9.73 0.2273-0.2691 0.5402-0.6394 0.6953-0.8231 2.002-2.37 4.978-5.279 7.266-7.102 1.943-1.548 5.045-3.694 6.127-4.237 0.1615-0.0811 0.4036-0.2237 0.5382-0.3169 0.2159-0.1496 1.558-0.8636 2.837-1.51 3.208-1.62 6.803-2.926 10.62-3.858 6.902-1.686 13.41-1.99 20.06-0.9368 0.861 0.1364 1.859 0.2902 2.218 0.3419 0.359 0.0516 1.35 0.2753 2.201 0.4969 0.8518 0.2216 1.791 0.4614 2.087 0.5328 0.8454 0.204 3.437 1.085 4.941 1.679 3.377 1.335 6.719 3.085 9.57 5.01 0.3662 0.2474 0.8326 0.5599 1.036 0.6944 0.7656 0.5054 2.937 2.163 3.677 2.806 0.2164 0.1882 0.6571 0.5694 0.9794 0.8473 0.5945 0.5126 3.969 3.874 4.496 4.48 0.7526 0.8634 1.977 2.36 2.474 3.021 0.3026 0.4036 0.7406 0.9833 0.9734 1.288 0.2328 0.3049 0.4232 0.5773 0.4232 0.6054 0 0.0281 0.2752 0.462 0.6116 0.9642 1.267 1.891 1.905 2.985 2.958 5.067 0.6976 1.38 1.616 3.312 1.616 3.4 0 0.032 0.2381 0.6561 0.5291 1.387 1.064 2.671 2.473 7.658 2.698 9.547 0.0513 0.4305 0.1385 1.047 0.1936 1.37 0.411 2.407 0.595 4.823 0.5958 7.827 0 2.963-0.1635 5.187-0.5564 7.531-0.081 0.4827-0.1874 1.231-0.2367 1.663-0.2117 1.857-1.624 6.862-2.695 9.55-0.291 0.7307-0.5291 1.355-0.5291 1.387 0 0.088-0.9183 2.02-1.616 3.4-1.053 2.082-1.691 3.176-2.958 5.067-0.3364 0.5022-0.6116 0.9356-0.6116 0.963 0 0.0275-0.2312 0.3528-0.5137 0.723-0.2825 0.3701-0.7529 0.9904-1.045 1.378-0.4778 0.6338-1.539 1.927-2.312 2.815-0.3923 0.4513-3.816 3.891-4.296 4.317-1.215 1.076-3.888 3.176-4.857 3.816-0.2038 0.1345-0.6701 0.447-1.036 0.6944-2.836 1.916-5.958 3.557-9.408 4.945-1.427 0.5739-4.602 1.657-5.25 1.791-0.2153 0.0444-1.088 0.2628-1.94 0.4852-0.8518 0.2224-1.842 0.4467-2.201 0.4983-0.359 0.0516-1.357 0.2056-2.218 0.3419-4.419 0.6999-8.852 0.8-13.18 0.2976-4.748-0.5513-9.489-1.711-13.26-3.243-0.7122-0.2893-1.337-0.526-1.388-0.526-0.0515 0-0.5735-0.2312-1.16-0.5137-0.5864-0.2825-1.172-0.5603-1.302-0.6172-0.4569-0.2005-2.967-1.526-3.214-1.698-0.1378-0.0955-0.3826-0.24-0.544-0.3211-1.082-0.5436-4.185-2.689-6.127-4.237-1.114-0.8875-2.854-2.45-4.153-3.73-1.019-1.004-3.47-3.728-3.853-4.282-0.0783-0.1133-0.3424-0.4514-0.587-0.7512-0.9666-1.184-2.996-4.178-3.676-5.423-0.1176-0.2152-0.4649-0.8316-0.7719-1.37-0.4723-0.828-1.476-2.837-2.022-4.049l-0.1492-0.331-0.5303 1.114c-0.7574 1.591-1.607 3.233-2.172 4.196-0.2681 0.4574-0.5253 0.8977-0.5716 0.9784-0.6524 1.138-2.965 4.464-3.752 5.397-0.166 0.1968-0.5441 0.6531-0.8401 1.014-1.47 1.791-3.977 4.362-5.629 5.771-0.3155 0.2691-0.7741 0.668-1.019 0.8865-0.2449 0.2185-0.559 0.4827-0.698 0.587-1.696 1.273-2.617 1.95-3.27 2.404-2.706 1.879-6.474 3.904-9.686 5.206-1.646 0.6672-4.839 1.77-5.55 1.918-0.2301 0.0476-1.211 0.2838-2.179 0.5247-2.536 0.6308-6.064 1.147-9.148 1.338-1.546 0.0956-3.07 0.1207-3.278 0.0538zm98.33-8.47c3.344-0.2026 6.999-0.8148 9.686-1.622 3.089-0.9283 5.717-1.967 8.023-3.172 1.279-0.6679 3.232-1.816 3.683-2.165 0.1961-0.1517 0.3873-0.2758 0.4251-0.2758 0.1495 0 3.044-2.144 3.821-2.831 0.2174-0.192 0.6946-0.6133 1.06-0.9362 1.686-1.488 3.71-3.648 5.422-5.786 0.4238-0.5295 1.88-2.57 1.88-2.635 0-0.0355 0.1493-0.2687 0.3318-0.5181 0.4163-0.5688 1.217-1.932 1.847-3.144 0.2656-0.5112 0.5446-1.027 0.62-1.146 0.0754-0.1194 0.2208-0.4275 0.3231-0.6849 0.1023-0.2574 0.3494-0.8202 0.5491-1.251 0.8298-1.788 2.153-5.83 2.532-7.733 0.8427-4.232 0.9823-5.594 0.9823-9.585 0-3.992-0.1396-5.353-0.9823-9.585-0.379-1.903-1.702-5.945-2.532-7.733-0.1997-0.4305-0.4506-1.003-0.5574-1.272-0.1068-0.2689-0.2296-0.5331-0.2728-0.587-0.0433-0.0539-0.2666-0.4723-0.4964-0.9297-0.9233-1.838-3.189-5.38-4.118-6.438-0.1274-0.145-0.3131-0.3901-0.4128-0.5446-0.4977-0.7714-3.707-4.158-5.112-5.394-0.367-0.3229-0.845-0.7442-1.062-0.9362-0.777-0.6865-3.671-2.831-3.821-2.831-0.0378 0-0.229-0.1241-0.4251-0.2758-0.4523-0.3499-2.409-1.5-3.683-2.165-0.8725-0.4552-3.512-1.669-3.629-1.669-0.0251 0-0.6103-0.2181-1.3-0.4848-0.6898-0.2666-1.558-0.5694-1.929-0.6729-0.3713-0.1035-1.203-0.3434-1.849-0.5331-1.25-0.3673-4.359-0.9614-6.276-1.199-0.6382-0.0793-2.01-0.1927-3.049-0.2522-5.113-0.2927-11.59 0.6677-16.36 2.425-0.5112 0.1884-1.216 0.4452-1.565 0.5705-0.8172 0.2927-3.11 1.345-4.305 1.975-2.051 1.082-3.956 2.286-5.765 3.644-1.915 1.437-2.809 2.215-4.562 3.966-1.943 1.941-2.677 2.785-4.306 4.954-1.9 2.528-3.501 5.245-4.868 8.258-0.293 0.6458-0.5881 1.35-0.6559 1.565-0.0677 0.2152-0.2301 0.6335-0.3608 0.9295-0.2705 0.6127-1.418 4.373-1.583 5.186-0.9455 4.669-1.003 5.162-1.06 9.05-0.0621 4.247 0.0441 5.682 0.666 9.001 0.3839 2.049 0.665 3.166 1.305 5.186 0.7443 2.348 1.512 4.28 2.373 5.968 0.7288 1.43 1.048 2.016 1.653 3.033 1.194 2.008 2.356 3.647 4.136 5.837 1.345 1.654 4.511 4.732 6.223 6.051 3.218 2.478 5.763 4.026 9.197 5.597 0.7022 0.3211 1.563 0.6864 1.913 0.8118 0.3498 0.1254 0.9882 0.3625 1.419 0.527 1.089 0.4161 4.428 1.359 5.43 1.533 0.4574 0.0796 1.272 0.2286 1.81 0.3311 3.02 0.5755 6.389 0.7821 9.589 0.5883z" fill="#8d5fd3" stroke-width=".09784"/><path d="m281.5 334.1c-0.0538-0.0173-0.3274-0.1023-0.608-0.1888-1.066-0.3286-2.077-1.161-2.565-2.112-0.4389-0.8551-0.5402-2.169-0.2456-3.188 0.2834-0.9798 0.9321-1.758 2.005-2.407 0.7019-0.4244 0.7124-0.426 3.664-0.5648 5.202-0.2445 9.093-1.054 14.24-2.961 2.327-0.8628 5.502-2.514 7.974-4.146 1.859-1.228 3.248-2.288 4.745-3.623 2.659-2.37 4.459-4.32 6.538-7.082 0.3316-0.4404 1.975-2.913 1.975-2.972 0-0.0315 0.1613-0.3093 0.3583-0.6174 0.1971-0.3081 0.5766-0.9804 0.8434-1.494 0.2668-0.5136 0.5665-1.088 0.666-1.276 0.3657-0.6922 1.091-2.289 1.26-2.776 0.0961-0.2759 0.333-0.8832 0.5264-1.35 0.1934-0.4664 0.4359-1.149 0.5388-1.517 0.1029-0.3677 0.3452-1.175 0.5384-1.794 0.3615-1.158 1.012-4.251 1.277-6.066 0.256-1.759 0.3768-5.532 0.2631-8.219-0.3448-8.141-3.324-16.52-8.263-23.24-1.423-1.936-2.371-3.043-4.215-4.926-2.32-2.369-4.337-4.035-7.051-5.828-2.437-1.609-5.708-3.313-7.925-4.128-0.4843-0.178-1.183-0.437-1.552-0.5756-0.3695-0.1385-0.9639-0.3334-1.321-0.433-0.357-0.0996-1.177-0.3364-1.823-0.5261-1.186-0.3484-4.183-0.9283-6.164-1.193-0.5919-0.079-2.111-0.1917-3.376-0.2505-3.004-0.1397-3.009-0.1405-3.713-0.566-1.445-0.8736-2.16-2.047-2.182-3.582-0.0275-1.916 1.042-3.425 2.931-4.137 0.3983-0.15 0.7461-0.1737 2.327-0.1588 7.119 0.0668 15.12 2.009 21.72 5.273 3.659 1.809 5.463 2.95 9.727 6.152 0.2509 0.1884 0.6989 0.5727 1.809 1.551 2.729 2.407 5.673 5.63 7.643 8.368 0.3221 0.4475 0.7254 1.005 0.8965 1.239 1.048 1.433 2.562 4.061 3.861 6.699l0.9853 2.001 1.092-2.196c1.898-3.818 4.058-7.22 6.178-9.73 0.2273-0.2691 0.5402-0.6394 0.6953-0.8231 2.002-2.37 4.978-5.279 7.266-7.102 1.943-1.548 5.045-3.694 6.127-4.237 0.1615-0.0811 0.4036-0.2237 0.5382-0.3169 0.2159-0.1496 1.558-0.8636 2.837-1.51 3.208-1.62 6.803-2.926 10.62-3.858 6.902-1.686 13.41-1.99 20.06-0.9368 0.861 0.1364 1.859 0.2902 2.218 0.3419 0.359 0.0516 1.35 0.2753 2.201 0.4969 0.8518 0.2216 1.791 0.4614 2.087 0.5328 0.8454 0.204 3.437 1.085 4.941 1.679 3.377 1.335 6.719 3.085 9.57 5.01 0.3662 0.2474 0.8326 0.5599 1.036 0.6944 0.7656 0.5054 2.937 2.163 3.677 2.806 0.2164 0.1882 0.6571 0.5694 0.9794 0.8473 0.5945 0.5126 3.969 3.874 4.496 4.48 0.7526 0.8634 1.977 2.36 2.474 3.021 0.3026 0.4036 0.7406 0.9833 0.9734 1.288 0.2328 0.3049 0.4232 0.5773 0.4232 0.6054s0.2752 0.462 0.6116 0.9642c1.267 1.891 1.905 2.985 2.958 5.067 0.6976 1.38 1.616 3.312 1.616 3.4 0 0.032 0.2381 0.6561 0.5291 1.387 1.064 2.671 2.473 7.658 2.698 9.547 0.0513 0.4305 0.1384 1.047 0.1936 1.37 0.411 2.407 0.595 4.823 0.5958 7.827 0 2.963-0.1634 5.187-0.5564 7.531-0.081 0.4826-0.1874 1.231-0.2367 1.663-0.2117 1.857-1.624 6.862-2.695 9.55-0.291 0.7307-0.5291 1.355-0.5291 1.387 0 0.088-0.9183 2.02-1.616 3.4-1.053 2.082-1.691 3.176-2.958 5.067-0.3364 0.5022-0.6116 0.9356-0.6116 0.963 0 0.0275-0.2312 0.3528-0.5137 0.723s-0.7529 0.9904-1.045 1.378c-0.4778 0.6338-1.539 1.927-2.312 2.815-0.3922 0.4513-3.816 3.891-4.296 4.317-1.215 1.076-3.888 3.176-4.857 3.816-0.2038 0.1345-0.6701 0.447-1.036 0.6944-2.836 1.916-5.958 3.557-9.408 4.945-1.427 0.5739-4.602 1.657-5.25 1.791-0.2153 0.0444-1.088 0.2628-1.94 0.4852-0.8518 0.2224-1.842 0.4467-2.201 0.4983-0.359 0.0516-1.357 0.2056-2.218 0.3419-4.419 0.6999-8.852 0.8-13.18 0.2976-4.748-0.5513-9.489-1.711-13.26-3.243-0.7122-0.2893-1.337-0.526-1.388-0.526-0.0515 0-0.5734-0.2312-1.16-0.5137-0.5864-0.2825-1.172-0.5603-1.302-0.6172-0.4569-0.2005-2.967-1.526-3.214-1.698-0.1378-0.0955-0.3826-0.24-0.544-0.3211-1.082-0.5436-4.185-2.689-6.127-4.237-1.114-0.8875-2.854-2.45-4.153-3.73-1.019-1.004-3.47-3.728-3.853-4.282-0.0783-0.1133-0.3424-0.4514-0.587-0.7512-0.9666-1.184-2.996-4.178-3.676-5.423-0.1176-0.2152-0.4649-0.8317-0.7719-1.37-0.4723-0.828-1.476-2.837-2.022-4.049l-0.1492-0.331-0.5303 1.114c-0.7574 1.591-1.607 3.233-2.172 4.196-0.2681 0.4574-0.5253 0.8977-0.5716 0.9784-0.6524 1.138-2.965 4.464-3.752 5.397-0.166 0.1968-0.5441 0.6531-0.8401 1.014-1.47 1.791-3.977 4.362-5.629 5.771-0.3155 0.2691-0.7741 0.668-1.019 0.8865-0.2449 0.2185-0.559 0.4827-0.698 0.5871-1.696 1.273-2.617 1.95-3.27 2.404-2.706 1.879-6.474 3.904-9.686 5.206-1.646 0.6672-4.839 1.77-5.55 1.918-0.2301 0.0476-1.211 0.2838-2.179 0.5247-2.536 0.6308-6.064 1.147-9.148 1.338-1.546 0.0956-3.07 0.1207-3.278 0.0538zm98.33-8.47c3.344-0.2026 6.999-0.8148 9.686-1.622 3.089-0.9283 5.717-1.967 8.023-3.172 1.279-0.6679 3.232-1.816 3.683-2.165 0.1961-0.1517 0.3873-0.2758 0.4251-0.2758 0.1496 0 3.044-2.144 3.821-2.831 0.2174-0.192 0.6946-0.6133 1.06-0.9362 1.686-1.488 3.71-3.648 5.422-5.786 0.4238-0.5295 1.88-2.57 1.88-2.635 0-0.0355 0.1493-0.2687 0.3318-0.518 0.4163-0.5688 1.217-1.932 1.847-3.144 0.2656-0.5112 0.5446-1.027 0.62-1.146 0.0754-0.1194 0.2208-0.4275 0.3231-0.6849 0.1023-0.2574 0.3494-0.8202 0.5491-1.251 0.8298-1.788 2.153-5.83 2.532-7.733 0.8427-4.232 0.9823-5.594 0.9823-9.585 0-3.992-0.1396-5.353-0.9823-9.585-0.379-1.903-1.702-5.945-2.532-7.733-0.1997-0.4305-0.4506-1.003-0.5574-1.272-0.1068-0.2689-0.2296-0.5331-0.2728-0.587-0.0433-0.0539-0.2666-0.4723-0.4964-0.9297-0.9233-1.838-3.189-5.38-4.118-6.438-0.1274-0.145-0.3131-0.3901-0.4128-0.5446-0.4977-0.7714-3.707-4.158-5.112-5.394-0.367-0.3229-0.845-0.7442-1.062-0.9362-0.777-0.6865-3.671-2.831-3.821-2.831-0.0378 0-0.229-0.1241-0.4251-0.2758-0.4523-0.3499-2.409-1.5-3.683-2.165-0.8725-0.4552-3.512-1.669-3.629-1.669-0.0251 0-0.6103-0.2182-1.3-0.4848-0.6898-0.2666-1.558-0.5694-1.929-0.6729-0.3713-0.1035-1.203-0.3434-1.849-0.5331-1.25-0.3673-4.359-0.9614-6.276-1.199-0.6382-0.0793-2.01-0.1927-3.049-0.2522-5.113-0.2927-11.59 0.6677-16.36 2.425-0.5112 0.1884-1.216 0.4452-1.565 0.5705-0.8172 0.2927-3.11 1.345-4.305 1.975-2.051 1.082-3.956 2.286-5.765 3.644-1.915 1.437-2.809 2.215-4.562 3.966-1.943 1.941-2.677 2.785-4.306 4.954-1.9 2.528-3.501 5.245-4.868 8.258-0.293 0.6458-0.5881 1.35-0.6559 1.565-0.0677 0.2152-0.2301 0.6335-0.3608 0.9295-0.2706 0.6127-1.418 4.373-1.583 5.186-0.9455 4.669-1.003 5.162-1.06 9.05-0.0621 4.247 0.0441 5.682 0.666 9.001 0.3839 2.049 0.665 3.166 1.305 5.186 0.7443 2.348 1.512 4.28 2.373 5.968 0.7288 1.43 1.048 2.016 1.653 3.033 1.194 2.008 2.356 3.647 4.136 5.837 1.345 1.654 4.511 4.732 6.223 6.051 3.218 2.478 5.763 4.026 9.197 5.597 0.7022 0.3211 1.563 0.6864 1.913 0.8118 0.3498 0.1254 0.9882 0.3625 1.419 0.527 1.089 0.4161 4.428 1.359 5.43 1.533 0.4574 0.0796 1.272 0.2286 1.81 0.3311 3.02 0.5754 6.389 0.7821 9.589 0.5883z" fill="#5a2ca0" stroke-width=".09784"/></g><metadata><rdf:RDF><cc:Work rdf:about=""/></rdf:RDF></metadata></svg>
    "####,)
}

#[get("/icon_text.svg")]
async fn icon_text() -> impl Responder {
    HttpResponse::Ok()
    .append_header(("Content-Type", "image/svg+xml"))
    .body(
    r####"<?xml version="1.0" encoding="UTF-8"?>
    <!-- Created with Inkscape (http://www.inkscape.org/) -->
    <svg width="85.94mm" height="16.65mm" version="1.1" viewBox="0 0 324.8 62.93" xmlns="http://www.w3.org/2000/svg"><g transform="translate(-5.092 -4.005)"><text x="2.1583736" y="54.138371" fill="#8d5fd3" font-size="66.67px" letter-spacing="9.21px" xml:space="preserve"><tspan x="2.1583736" y="54.138371" font-family="'URW Gothic'">Catalyst</tspan></text></g></svg>
    "####,)
}


#[get("/manifest.json")]
async fn manifest() -> impl Responder {
    HttpResponse::Ok().body(
    r####"{
        "name": "Catalyst",
        "short_name": "Catalyst",
        "start_url": "../",
        "icons": [{
            "src": "../icon.svg",
            "sizes": "any"
        }],
        "background_color": "#8d5fd3",
        "theme_color": "#8d5fd3",
        "display": "fullscreen"
    }"####,)
}

// Define a struct for the post data
#[derive(Debug, Serialize, Deserialize)]
struct Post {
    title: String,
    author: String,
    content: String,
    signature: Vec<u8>,
}


impl Default for Post {
    fn default() -> Self {
        Post {
            title: "Untitled".to_string(),
            author: "No Author".to_string(),
            content: "There are no posts present, this is the default post text.".to_string(),
            signature: vec![],
        }
    }
}

#[post("/post")]
// Define a function to create a new post in the database
async fn create_post(mut post: web::Json<Post>) -> impl Responder {
    // Verify the signature using the public key in the author field
    let public_key_bytes = match general_purpose::URL_SAFE_NO_PAD.decode(&post.author) {
        Ok(bytes) => bytes,
        Err(_) => vec![],
    };
    let public_key =
        UnparsedPublicKey::new(&signature::ED25519, &public_key_bytes);
    let signature_bytes = match general_purpose::URL_SAFE_NO_PAD.decode(&post.signature) {
        Ok(bytes) => bytes,
        Err(_) => vec![],
    };

    // Generate a unique ID for the post using SHA-256
    let preimage = format!("{}{}{}{:?}", post.title, post.author, post.content, post.signature);
    let post_id = digest::digest(&SHA256, preimage.as_bytes());
    let post_id_str;
    match public_key.verify(post.content.as_bytes(), &signature_bytes) {
        Ok(_) => {

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
        Err(_) => {
            // Author labelled as anonymous if public key not provided
            post.author = String::from("Unverified: ")+&post.author.clone();
            post_id_str = format!("unverified_{}", post.author.clone());
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

// Define a struct for the anchor data
#[derive(Debug, Serialize, Deserialize)]
struct Anchor {
    link_id: String,
    post_id: String,
    reference: String,
    referencing_post_id: String,
}


#[post("/anchor")]
async fn create_anchor(anchor: web::Json<Anchor>) -> impl Responder {
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
async fn get_random_posts(web::Query(query): web::Query<PostQuery>) -> impl Responder {
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

    // Select a random subset of post directories
    let mut rng = thread_rng();
    let random_post_dirs = recent_post_dirs.choose_multiple(&mut rng, max_posts);

    // Read the post data from each selected post directory
    let mut posts = HashMap::new();
    for post_dir in random_post_dirs {
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

#[get("/post/{post_id}/unfiltered_anchors")]
async fn get_unfiltered_anchors(post_id: web::Path<String>) -> impl Responder {
    // Get the list of anchor directories for the post
    let anchor_dir = PathBuf::from("./posts")
        .join(post_id.into_inner())
        .join("anchors");
    let anchor_dirs = fs::read_dir(&anchor_dir).unwrap();

    // Read the anchor data from each anchor directory
    let mut anchors = vec![];
    for anchor_dir in anchor_dirs {
        let anchor_file = anchor_dir.unwrap().path().join("anchor.json");
        let mut file = fs::File::open(&anchor_file).unwrap();
        let mut anchor_json = String::new();
        file.read_to_string(&mut anchor_json).unwrap();
        let anchor: Anchor = serde_json::from_str(&anchor_json).unwrap();
        anchors.push(anchor);
    }

    HttpResponse::Ok().json(anchors)
}

#[get("/post/{post_id}/anchors")]
async fn get_anchors(
    post_id: web::Path<String>,
    web::Query(query): web::Query<PostQuery>
) -> impl Responder {
    let recency_days = query.recency_days.unwrap_or(7);
    let max_anchors = query.max_posts.unwrap_or(10);
    // Get the list of anchor directories for the post
    let anchor_dir = PathBuf::from("./posts")
        .join(post_id.into_inner())
        .join("anchors");
    let anchor_dirs = fs::read_dir(&anchor_dir).unwrap();

    // Filter the list of anchor directories to only include those that are recent enough
    let recency_threshold = SystemTime::now() - Duration::from_secs(u64::from(recency_days) * 24 * 60 * 60);
    let recent_anchor_dirs = anchor_dirs
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            !entry.file_name().to_str().unwrap().to_owned().starts_with("unverified_")
        })
        .filter(|entry| {
            let anchor_file = entry.path().join("anchor.json");
            match fs::metadata(&anchor_file) {
                Ok(metadata) => metadata.modified().unwrap() > recency_threshold,
                Err(_) => false,
            }
        })
        .collect::<Vec<_>>();

    // Select a random subset of anchor directories
    let mut rng = thread_rng();
    let random_anchor_dirs = recent_anchor_dirs.choose_multiple(&mut rng, max_anchors);

    // Read the anchor data from each selected anchor directory
    let mut anchors = vec![];
    for anchor_dir in random_anchor_dirs {
        let anchor_file = anchor_dir.path().join("anchor.json");
        let mut file = fs::File::open(&anchor_file).unwrap();
        let mut anchor_json = String::new();
        file.read_to_string(&mut anchor_json).unwrap();
        let anchor: Anchor = serde_json::from_str(&anchor_json).unwrap();
        anchors.push(anchor);
    }

    HttpResponse::Ok().json(anchors)
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
            return HttpResponse::Ok().body(tar_gz.clone());
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
            .service(create_post)
            .service(index_page)
            .service(icon)
            .service(icon_text)
            .service(manifest)
            .service(get_post_by_id)
            .service(get_random_posts)
            .service(get_current_bin)
            .service(get_current_src)

    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
