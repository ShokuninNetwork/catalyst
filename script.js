import init, { AppState, Keypair } from "./dist/wasm-logic.js";
await init("/dist/wasm-logic.wasm");
const app = new AppState();
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
  const postResponse = postResponse(postID);
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
  if (postContainer.children.length > maxPosts + 2) {
    // Remove the last post from the container
    postContainer.removeChild(postContainer.lastChild);
  }
}
async function getUnfilteredAnchors(postId) {
  const response = postResponse(postId, "/unfiltered_anchors");
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
  const response = postResponse(postId, '/anchors?max_posts=${userPrefs.maxPosts}&recency_days=${userPrefs.recencyDays}');
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
    let post = posts[postID];
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
document.getElementById('toggle-inkwell').addEventListener('click', function() {
  document.getElementById('inkwell').classList.toggle('collapsed');
});
document.addEventListener('click', (ev) => {
  const isToggleBtn = ev.target.classList.contains('toggle-button');
  const isPreviewBtn = ev.target.classList.contains('preview-button');

  if (isToggleBtn || isPreviewBtn) {
    const container = isToggleBtn ? editorContainer : previewContainer;
    const isHidden = container.classList.contains('hidden');

    if (isHidden) {
      container.classList.remove('hidden');
      container.classList.add('shown');
      if (isWideWindow.matches) {
        postContainer.classList.add('small');
      }
      if (!isToggleBtn) {
        const previewContent = postEditor.value;
        postRenderer(previewPost, previewContent);
      }
    } else {
      container.classList.add('hidden');
      container.classList.remove('shown');
      if (!isPreviewBtn) {
        postContainer.classList.remove('small');
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
    signature: postEditor.temp ? postEditor.temp.signature ? postEditor.temp.signature : "" : ""
  };

  // Send the new post object to the '/post' endpoint
  // Get the post ID from the response and log it to the console
  const postObj = await postMethod(newPost);

  logDebug(`New post created with ID: ${postObj.postID}`);
  if (postEditor.temp) if (postEditor.temp.pendingAnchors) postEditor.temp.pendingAnchors.forEach(pendingAnchor => {

    let postRendered = document.createElement("div");
    const previewContent = postEditor.value;
    postRenderer(postRendered, previewContent);
    let query = "a[href='#" + pendingAnchor.link_id + "']";
    let reference = postRendered.querySelector(query);
    let referenceText = reference ? reference.innerText : "";
    let refStart = postRendered.innerText.indexOf(referenceText);
    let refEnd = refStart + referenceText.length;

    let newAnchor = {
      link_id: pendingAnchor.link_id,
      post_id: pendingAnchor.post_id,
      reference: pendingAnchor.post_start + ":" + pendingAnchor.post_end + ":" + refStart + ":" + refEnd,
      referencing_post_id: postObj.postID
    }
    // Send the new anchor object to the '/anchor' endpoint
    // anchorMethod(anchor);
  });

  clearPost();
  appendPost(postObj.postID);
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
  const localStorageKeypair = localStorage.getItem("keySeed");
  logDebug(localStorageKeypair);
  let keypair = localStorageKeypair ?
    (() => {
      let keySeed = new Uint8Array(atob(localStorageKeypair).split("").map(c => c.charCodeAt(0)));
      return Keypair.from_seed(keySeed)
    })() :
    new Keypair();
  let save = btoa(
    String.fromCharCode(
      ...keypair.seed_bytes()
    )
  );
  localStorage.setItem("keySeed", save);
  // create an "aka" element which is appended to the post,
  // holding the non-cryptographic author name
  let aka = document.createElement('div');
  if (postAuthor.value) {
    aka.classList.add('aka');
    aka.innerText = postAuthor.value;
    postEditor.value += aka.outerHTML;
  }
  // Replace author with base64 of the pubkey
  postAuthor.value = btoa(
    String.fromCharCode(
      ...keypair.public_key_bytes()
    )
  );
  logDebug(keypair.public_key_bytes());
  // Sign the post content using the keypair
  const signature = keypair.sign(
    postEditor.value
  );
  logDebug(signature);
  // Store the signature in a variable waiting for post submission
  if (!postEditor.temp) { postEditor.temp = {} }
  postEditor.temp.signature = btoa(
    String.fromCharCode(...signature)
  );
  postEditor.readOnly = true;
  postAuthor.readOnly = true;
}
const signButton = document.getElementById('sign-button');
signButton.addEventListener('click', signPost);
const clearButton = document.getElementById('clear-button');
clearButton.addEventListener('click', clearPost);

document.getElementById("inkwell").addEventListener('signalingMessage', async (event) => {
    
  // Define the post object with the desired properties
  const stubPost = {
    title: "[Signal] Signaling details",
    author: event.detail.id,
    content: event.detail.sdpSender + " \n " + event.detail.manualSignal,
    signature: postEditor.temp ? postEditor.temp.signature ? postEditor.temp.signature : "" : ""
  };

  const postObj = await postMethod(stubPost);
  appendPost(postObj.postID);

  window.dispatchEvent(new Event('signalingMessage'));
  
  setTimeout(function() {
  logDebug('Received signaling event:', event.detail);
  }, 0);
  // Add logic here
});

document.getElementById('debugButton').addEventListener('click', function() {
  var debug = document.getElementById('debug');
  debug.style.display = (debug.style.display === 'none' || debug.style.display === '') ? 'block' : 'none';
});

function logDebug(message){
  document.getElementById("debugIframe").contentWindow.postMessage(message, 'http://localhost:8080/debugger.html');
}

document.getElementById('eventTest').addEventListener('click', function() {
  logDebug("Hello world");
});


async function postMethod(post){
  const R = await fetch('/post', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(post)
  });
    
  const D = await R.json();
  const ID = D.postID;

  return {
    response: R,
    data: D,
    postID: ID
  };
};
async function postResponse(postID, modifiers = ""){
  return await fetch(`/post/${postID}${modifiers}`);
};

async function anchorMethod(anchor) {
  const responsePromise = fetch('/anchor', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(anchor)
  });

  return responsePromise;
};
